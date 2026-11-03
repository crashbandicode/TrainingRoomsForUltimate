#!/usr/bin/env python3
"""Package 6 carry tool: manifest-driven copy of the fork's ours-new files with a deterministic path rewrite.

    python3 -I tools/carry_pkg6.py carry   [--stage m5o] [--update] [--manifest FILE]
    python3 -I tools/carry_pkg6.py verify  [--stage m5o] [--dest-only]

The manifest (docs/provenance/pkg6-carry-manifest.json) lists, for every carried file, the stage path, the
destination, the crate the file belongs to in the fork (root / consts / mii / tui), an optional list of exact-text
`edits` (a clean-room change that no path rewrite can express; each carries its reason and is a CARRY-OVER ledger
row) and the sha256 of the stage file and of the written destination. It also holds the module map the rewrite
uses: `modules[crate][fork module] = clean-room module path`.

* `carry` reads each stage file (only paths listed in the manifest; every one is an `ours-new` file of
  docs/provenance/provenance-files.json or one of the m5o additions recorded there), checks its sha256 against the
  manifest (a changed file needs `--update`, which also refreshes the manifest and prints the changed files for the
  ledger), applies the rewrite and the edits, writes the destination and records its sha256.
* `verify` re-derives every destination from the stage and compares bytes (or, with `--dest-only`, compares the
  destination's sha256 with the manifest: no stage needed, used by the host test). A hand-edited carried file fails.

A re-sync from a newer stage (W19 / W20 patched stages) is `carry --stage <name> --update` followed by the tests.
Nothing under the stage is ever written.
"""
import argparse, hashlib, json, os, posixpath, re, sys

STAGES = "/home/intpa/smash-training-lobby/artifacts/enhancements/console-practice91/lobby-parity167/parked-reuse175/refactor/stages"
DEFAULT_MANIFEST = "docs/provenance/pkg6-carry-manifest.json"
EXTERNAL = {"training_mod_consts": "consts", "training_mod_mii": "mii", "training_mod_tui": "tui", "byteflags": "byteflags",
            "training_mod_sync": "sync", "training_mod_macros": "macros", "log": "log"}
HEADS = "crate|" + "|".join(EXTERNAL)


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def split_top(inner: str):
    parts, depth, cur = [], 0, ""
    for ch in inner:
        if ch == "{": depth += 1
        if ch == "}": depth -= 1
        if ch == "," and depth == 0:
            parts.append(cur); cur = ""
        else:
            cur += ch
    if cur.strip():
        parts.append(cur)
    return [p.strip() for p in parts if p.strip()]


def rewrite_path(head_crate: str, member: str, modules: dict):
    """`member` is 'name' or 'name::rest' under a crate; returns the clean-room path or None when unmapped."""
    segs = member.split("::")
    table = modules.get(head_crate, {})
    for k in range(len(segs), 0, -1):  # longest mapped prefix wins (`training::practice_policy` before `training`)
        dest = table.get("::".join(segs[:k]))
        if dest is not None:
            rest = "::".join(segs[k:])
            return dest + ("::" + rest if rest else "")
    return None


def rewrite_source(text: str, file_crate: str, modules: dict, name: str) -> str:
    # the storage root moved (decision S1, model::storage): every fork path under sd:/ultimate/TrainingModpack is under TrainingRooms
    text = text.replace("sd:/ultimate/TrainingModpack", "sd:/ultimate/TrainingRooms")
    # 1. grouped `use <head>::{a, b::c, ...};` for the crate heads
    def group(m):
        head = m.group(1)
        crate = file_crate if head == "crate" else EXTERNAL.get(head)
        if crate is None:
            return m.group(0)
        members = split_top(m.group(2))
        out = []
        for mem in members:
            new = rewrite_path(crate, mem.split(" as ")[0].strip(), modules)
            alias = " as " + mem.split(" as ")[1].strip() if " as " in mem else ""
            if new is None:
                if mem.startswith("self"):
                    new = None
                out.append(f"{head}::{mem}")
            else:
                out.append(new + alias)
        return "use {" + ", ".join(out) + "};"

    text = re.sub(r"\buse\s+(" + HEADS + r")::\{((?:[^{}]|\{[^{}]*\})*)\}\s*;", group, text)

    # 2. plain paths
    def plain(m):
        head, member = m.group(1), m.group(2)
        crate = file_crate if head == "crate" else EXTERNAL[head]
        new = rewrite_path(crate, member, modules)
        if new is None:
            return m.group(0)
        return new

    text = re.sub(r"\b(" + HEADS + r")::([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)", plain, text)
    return text


def rewrite_paths(text: str, entry: dict, dest_by_src: dict) -> str:
    """`#[path = "x.rs"]` module declarations: the path is resolved against the stage file, mapped to the destination the
    manifest gives that file and written relative to this file's destination (so the children of a carried module keep
    resolving wherever the manifest puts them). A target the manifest does not hold yet is left as written (and reported):
    it resolves once the file that carries it joins the manifest (the cycle's other worker, or a later tranche)."""
    def engine_path(dest):
        # a pure file the manifest carries under src/model/ is the module crate::engine::<dir>::<name> (src/engine/mod.rs declares each area)
        if not dest.startswith("src/model/") or not dest.endswith(".rs"):
            return None
        parts = dest[len("src/model/"):-3].split("/")
        if parts[-1] == "mod":
            parts = parts[:-1]
        return "crate::engine::" + "::".join(parts) if parts else None

    def use_form(m):
        # `#[path = "x.rs"] mod name;` of a pure carried file becomes `use crate::engine::<area>::x as name;` (one module, not two copies)
        rel = m.group(1)
        target = posixpath.normpath(posixpath.join(posixpath.dirname(entry["src"]), rel))
        path = engine_path(dest_by_src.get(target, ""))
        if path is None:
            return m.group(0)
        return f"{m.group(2) or ''}use {path} as {m.group(4)};"
    if entry["dest"].startswith("src/runtime/"):  # a Switch-only file reaches the pure engine area; a pure file's own test modules stay `mod`
        text = re.sub(r'#\[path\s*=\s*"([^"]+)"\s*\]\s*(pub(\([a-z]+\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;', use_form, text)

    def one(m):
        rel = m.group(2)
        target = posixpath.normpath(posixpath.join(posixpath.dirname(entry["src"]), rel))
        if target not in dest_by_src:
            print(f"note: {entry['src']}: #[path = \"{rel}\"] names {target}, not in the manifest (left as written)", file=sys.stderr)
            return m.group(0)
        new = posixpath.relpath(dest_by_src[target], posixpath.dirname(entry["dest"]))
        return f'{m.group(1)}"{new}"]'
    return re.sub(r'(#\[path\s*=\s*)"([^"]+)"\s*\]', one, text)


def rewrite_includes(text: str, entry: dict, dest_by_src: dict) -> str:
    """`include_str!("../x")` (and include_bytes!/include!): the path is resolved against the stage file, mapped to the
    destination the manifest gives that file, and written relative to this file's destination."""
    def inc(m):
        rel = m.group(2)
        target = posixpath.normpath(posixpath.join(posixpath.dirname(entry["src"]), rel))
        if target not in dest_by_src:
            raise SystemExit(f"{entry['src']}: {m.group(1)}(\"{rel}\") names {target}, which is not in the manifest")
        new = posixpath.relpath(dest_by_src[target], posixpath.dirname(entry["dest"]))
        return f'{m.group(1)}("{new}")'
    return re.sub(r"\b(include_str!|include_bytes!|include!)\(\s*\"([^\"]+)\"\s*\)", inc, text)


def transform(entry: dict, modules: dict, stage_bytes: bytes, dest_by_src: dict) -> bytes:
    if not entry["src"].endswith(".rs"):
        out = stage_bytes
    else:
        text = stage_bytes.decode("utf-8")
        text = rewrite_source(text, entry["crate"], modules, entry["src"])
        text = rewrite_includes(text, entry, dest_by_src)
        text = rewrite_paths(text, entry, dest_by_src)
        if entry.get("widen"):
            # the fork's file sat directly under the crate root (or was the #[path] child of a root file), so `pub(super)` meant "the
            # whole crate"; in the engine tree it would stop at the area, so it becomes `pub(crate)` (an entry flag, a ledger row)
            text = re.sub(r"\bpub\(super\)", "pub(crate)", text)
        if entry.get("prepend"):
            # after the file's leading inner doc comments (`//!`), which must stay first
            lines = text.split("\n")
            k = 0
            while k < len(lines) and (lines[k].startswith("//!") or lines[k].strip() == ""):
                k += 1
            text = "\n".join(lines[:k]) + ("\n" if k else "") + entry["prepend"] + "\n".join(lines[k:])
        out = text.encode("utf-8")
    for e in entry.get("edits", []):
        s = out.decode("utf-8")
        if "delete_from" in e:
            # delete [delete_from, delete_to): `delete_from` must occur once; `delete_to` is the first occurrence after it
            # (or, with `delete_to_last`, the last occurrence in the file); the text from delete_to on stays
            if s.count(e["delete_from"]) != 1:
                raise SystemExit(f"{entry['src']}: delete_from `{e['delete_from'][:60]}` matches {s.count(e['delete_from'])} times")
            a = s.index(e["delete_from"])
            if "delete_to_last" in e:
                b = s.rindex(e["delete_to_last"])
            else:
                b = s.find(e["delete_to"], a + len(e["delete_from"]))
            if b < 0 or b < a:
                raise SystemExit(f"{entry['src']}: delete_to for `{e['delete_from'][:60]}` not found")
            out = (s[:a] + s[b:]).encode("utf-8")
            continue
        n = s.count(e["old"])
        if n != e.get("count", 1):
            raise SystemExit(f"{entry['src']}: edit `{e['old'][:60]}` matches {n} times, expected {e.get('count', 1)}")
        out = s.replace(e["old"], e["new"]).encode("utf-8")
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["carry", "verify", "ledger"])
    ap.add_argument("--stage", default=None)
    ap.add_argument("--update", action="store_true")
    ap.add_argument("--dest-only", action="store_true")
    ap.add_argument("--manifest", default=DEFAULT_MANIFEST)
    ap.add_argument("--root", default=".")
    a = ap.parse_args()
    mpath = os.path.join(a.root, a.manifest)
    manifest = json.load(open(mpath))
    stage_name = a.stage or manifest["stage"]
    stage = os.path.join(STAGES, stage_name, "stage")
    modules = manifest["modules"]
    dest_by_src = {e["src"]: e["dest"] for e in manifest["files"]}
    if a.cmd == "ledger":
        write_ledger(a.root, manifest)
        return
    bad = 0
    changed = []
    for entry in manifest["files"]:
        dest = os.path.join(a.root, entry["dest"])
        if a.cmd == "verify" and a.dest_only:
            if not os.path.exists(dest):
                print("MISSING", entry["dest"]); bad += 1; continue
            if sha(open(dest, "rb").read()) != entry["dest_sha256"]:
                print("EDITED ", entry["dest"]); bad += 1
            continue
        src_path = os.path.join(stage, entry["src"])
        data = open(src_path, "rb").read()
        src_hash = sha(data)
        if src_hash != entry["src_sha256"]:
            if a.cmd == "carry" and a.update:
                changed.append((entry["src"], entry["src_sha256"][:12], src_hash[:12]))
                entry["src_sha256"] = src_hash
            else:
                print("STAGE CHANGED", entry["src"], entry["src_sha256"][:12], "->", src_hash[:12]); bad += 1; continue
        out = transform(entry, modules, data, dest_by_src)
        if a.cmd == "carry":
            os.makedirs(os.path.dirname(dest), exist_ok=True)
            open(dest, "wb").write(out)
            entry["dest_sha256"] = sha(out)
        else:
            if not os.path.exists(dest) or open(dest, "rb").read() != out:
                print("DIFFERS", entry["dest"]); bad += 1
    if a.cmd == "carry":
        manifest["stage"] = stage_name
        json.dump(manifest, open(mpath, "w"), indent=1)
        open(mpath, "a").write("\n")
        for c in changed:
            print("re-synced", *c)
        print(f"carried {len(manifest['files'])} files from stage {stage_name}")
    if bad:
        print(f"{bad} problem(s)")
        sys.exit(1)
    if a.cmd == "verify":
        print(f"verified {len(manifest['files'])} files" + (" (dest only)" if a.dest_only else f" against stage {stage_name}"))


DELETED_SECTION = """
## Deleted fork functions (lead ruling 2026-11-02: single owner)

The fork called these from its mixed upstream code (hooks whose sites are not ours-new files). The clean room's packages 2-4 own the behaviour, so
the carried functions are deleted by recorded manifest edits of `training/practice.rs` and `training/lobby.rs`. "Proof" names the Eden row of the
package-6 lobby-practice set that shows the parity (rows are in `eden/EDEN-PACKAGE-6.md`).

| Fork function | Clean-room owner | Behaviour difference / status | Proof |
| --- | --- | --- | --- |
| `lobby::permits_entry(entry)` (waiting room serves only entries 0 and 4) | `model::cpu::role::is_cpu_role` / `Scene::Lobby` with `LOBBY_CPU_ENTRY = 4` (pkg2: "only entries 0 and 4 are serviced, entry 0 is never a CPU") | none known | CPU behaviours act on entry 4 and on no other waiting-room entry |
| `practice::waiting()` | `runtime::scene::current() == Some(Scene::Lobby)` (`model::scene`) | none known (both need Lobby Practice on and the practice UI ready) | lobby-practice row: scene reads Lobby only with the UI up |
| `practice::role(module)` (0 Player, 1 CPU, by entry) | **kept**: `cycle_preload::on_frame` (the preload planner, carried) reads it; pkg2 has `role::is_cpu_role(scene, entry)` and `RoleTable::accessor_of` for the CPU behaviours | not deleted: a live consumer exists | preload row on Player 1 in Training and in the waiting room |
| `practice::primary_cpu(module)` | `features::cpu::primary_entry(lobby)` and the per-body `FighterKey` (the second body, Nana, is a distinct key) | none known | CPU row with Ice Climbers as the CPU |
| `practice::visualizes(module)` (effects and HUD for a practice body, and for a weapon its owner role owns) | `scene::is_training()` (native Training or an active lobby, not busy) keys pkg4's input display and the notice HUD; pkg2/3 effect hooks key on the accessor | **unverified for weapon articles owned by a lobby role**: the fork showed effects for them explicitly | visualization row (a lobby CPU article effect) |
| `practice::panel_entry(module)` (extra CPUs of the native Training panels: entry receipts that release the stop) | **restored** (2026-11-02, pkg4 request): package 4's named panel holds every live fighter while it is open (`named_macros::hold_panel_fighter`, released through `entry_departed` / `forget_stopped_entry`); the fork's semantics (every live entry 0..8 in native Training, role-gated in the waiting room) | none | s6-04: save/load with an extra CPU leaves it unfrozen; pkg4 named-panel rows |
| `practice::acquire()` / `forget()` (the practice entry lease the save/record slots belong to) | **kept** (lead ruling): `runtime::lobby::frame` acquires the lease once both entries resolve and retires it through `refresh` when an entry is replaced; `lobby::update` forgets it at the lobby practice start and end; the roster's destruction path `entry_departed` retires it at once. pkg3 owns slots by entry and pkg4 by `RecordOwner` | none: the retirement of the session on a replaced entry is the fork's | hot swap during a practice session retires it |
| `practice::owns_cpu_controls(controls)` (FE+0x4178 controller owner of the CPU entry) | the input stage hands slot 1's record to `roster::handle_operator_input` (`practice::cpu_operator`); pkg2's `cpu_operated` | no controls-object ownership test: the slot index identifies the operator | controls-ownership row: Behavior = Control operator cycles the CPU |
| `cpu_pilot::thinking()` (the pilot's AI decision is running; the fork's input recorder asked it so the decision's controller writes were not taken for a player's input) | package 4's recorder records from the input stage's mapped poll record before the delay; the pilot writes its pad into the record after the delay (`p1::input`), so the two never meet. Recording while an AI drives P1 is refused (SPEC-input-01 B4, `RecordRefusal::P1DrivenByAi`, `natives::is_operation_cpu(0)`) | none known: the bracket's only reader was the fork's recorder hook; the `THINKING` flag and the scope-contract row are removed with it | CP0 / IN01-E19: P1 armed (pilot or fallback), the Record chord shows the refusal notice and nothing is recorded, the AI's inputs show on the display |
| `character_sources::reserved(body)` (skip plugin automations for a helper body the cycle owns: entries 2, 3, 5, 6) | automations are keyed by role in the clean room: pkg2's `model::cpu::role` (`is_cpu_role`, `FighterKey`; entries 0, 1 and the lobby's 4), pkg3's `Role::of_entry`, pkg4's `cpu_body()`; every other entry is ignored by construction | none known (`legacy_training_owner_reserved`, the probe's check, stays) | row: a managed Kirby / third source at entries 2/3/5/6 gets no CPU behaviour, no save-state step and no recording while the cycle owns it (hat shortcut, item grant, preload) |
| `heap_fields::HeapFields` (`heap_allocations=<calls>` of the fork's own install line) | package 1's install line owns `heap_allocations=` (hook allocations); the meter's call count is the `[HeapMeter]` diag line (`runtime::diag_log`, `compat::logging::heap_profile`) | the field name is package 1's; the meter reads `hook_stats::heap_bytes()` | diag row R5: `[HeapMeter]` lines with `plugin-heap-meter` |
| `heap_metrics::Meter` / `practice_trace::TraceAllocator` as `#[global_allocator]` (`plugin-heap-meter`, `console-practice-trace`, `native-unlink-probe`) | one global allocator, `runtime::alloc` (package 1's `CountingAllocator`); its byte meter (`hook_stats::heap_bytes`) is what the meter and the recorder read; under `console-practice-trace` the global allocator is the recorder's `TraceAllocator`, which forwards to `CountingAllocator` and records events 100-104 as before | none in the shipped build (the byte counters compile away); the diag features build (`tools/feature_matrix.py`) | `tests/hook_alloc.rs` (byte meter), `tests/pkg6_features.rs` (one allocator), diag boot rows |

## Clean-room natives deleted (lead ruling 2026-11-02)

Package 3's `runtime::state_natives` held seven natives for a save-state item restore of its own. The carried item engine (`roster::character_items`, reached through `features::item_bridge`) is the owner since the item engine took the save state's held item, and it calls the same game functions behind the rows `roster::CALLED_ROWS` lists, so the duplicates had no caller.

| Function | Owner now | Behaviour difference | Proof |
| --- | --- | --- | --- |
| `state_natives::{have_item, is_have_item, get_have_item_id, remove_item, set_have_item_action, object_category}` | `character_items` (`ItemModule::*` and `utility::get_category` through `smash::`, guarded by the `state_*` and `utility_get_category` rows) | none known | item save/restore with the owner not in the match (set D rows) |
| `state_natives::turnip_power(peach, n)` (two raw offsets, no guarded row) | `character_items` builds the turnip power thunks itself (`paste!`) | none known | `[ItemTable]` diag dump (turnip variations 0, 5, 6, 7) |

## Wired to the clean-room owner of the moment (lead ruling 2026-11-02)

The fork called these from mixed upstream code; the clean room's owner of each moment now calls them.

| Carried function | Owner of the moment (caller) | Behaviour difference | Proof |
| --- | --- | --- | --- |
| `kirby_hats::restore(body, kind)` | pkg3's load completion, `features::states::kirby_copy` (SPEC-state-05 section 5.2 step 6): while the roster cycle runs the hat engine restores the saved hat asynchronously (no opponent needed, waits for the load to finish); otherwise the opponent-only copy of step 2 | the hat comes back for any saved hat, not only the opponent's | Kirby hats row: save with a hat, load with the hat's owner not in the match |
| `kirby_hats::cancel_pending()` | `features::states::cancel_pending` (scene departure, hot swap, toggle off, Training reset) | a cancelled load drops its queued hat restore (and any shortcut hat request) | hot swap during a Kirby restore |
| `character_roster::solo_popo_active(cpu)` | `features::states::nana_removed`, the machine's Ice Climbers hand-off (SPEC-state-01 section 5.3 step 9) | without it a load of Popo out of REBIRTH waits for a Nana that Solo Popo removed (`NanaPosMove` never ends) | native Training, Solo Popo on: save and load finish |
| `presentation::{state_load_started, state_load_completed}` and `character_roster::main_base` (the Mii HUD portrait's state-load check) | `features::states`: started for both roles at the load request, completed per role in `finish` (SPEC-state-01 section 9, the per-role load notification) | none: a load must change no HUD card, the check logs `[Hud] state_load ... unchanged=` | save/load with a Mii or any swapped fighter: `[Hud] state_load owner=0 unchanged=true` |
| `character_sources::legacy_training_owner_reserved` | retired with its sole source-owner experiment consumer (RC RESET option2, lead 2026-11-03 10:42:04); ordinary role-based ownership unchanged | no shipped-path change | final RC parity pending |
| `cycle_preload_config::publish_applied(menu)` | `compat::menu::apply_snapshot` (every menu commit after the boot publish; decision D1 `preload::apply`) | none: the engine's INI boot cache and its cache and re-plan follow a menu change again | preload rows: change slots in the menu, `[CyclePreload] applied ...` and the new `cycle_preload.ini` |
| `character_roster::{current_mii_configuration, waiting_drafts_ready, cpu_reload_ready, request_cpu_kind}` | package 4's named-macro layer (`macros::named_macros`, `macros::replay_capture`) through its retired port: stream/pkg4 `d124246` (not merged here yet) | none | pkg4 named-macro rows |

## Carried helpers waiting for a screen (consumer not built; not deleted)

`character_items::request` (the Give item screen), the Mii editor bridge's `request`/`menu_ready`/`catalog_read` and the `ui::menu_app` Mii-catalog methods (the Mii list screen), `practice_stage_request::*` and `training_stage_rotation::*` (the Lobby Stages screen), `lobby::set_ui_ready` (the info_waiting tick). Their owners are the ui-screens stream and package 5; the lead's list names them.

RC warning-root audit (lead 2026-11-03 12:57Z): removed the now-orphaned
`character_roster::{original_player_kind, original_cpu_kind}`,
`cycle_preload::fighter_ready`, `native_body_bank::cycle_ready` and
`mii_ordinary_native::try_registry`. Across all Rust sources and tests there were
only their five definitions and the orphan `fighter_ready -> cycle_ready` call;
no screen or feature retained a caller. The live `original_kind`, progress,
registry and native readiness/transaction paths remain unchanged. These are
exact-text carry deletions, not lint suppression or feature-gated placeholders.
"""


def write_ledger(root, manifest):
    """The carried-file table of CARRY-OVER.md (stage, path, sha256, clean-room changes), generated from the manifest."""
    path = os.path.join(root, "docs/provenance/pkg6-carry-ledger.md")
    with open(path, "w") as f:
        f.write(f"# Package 6 carried files (stage {manifest['stage']})\n\n")
        f.write("Generated by `python3 -I tools/carry_pkg6.py ledger` from `pkg6-carry-manifest.json`; do not edit. Every file is a copy of\n")
        f.write("the stage file with the path rewrite of the tool (`crate::x` / `training_mod_*::x` to the clean-room module path, listed in the\n")
        f.write("manifest's `modules`) and, where a row says so, the exact-text edits below it. `src sha256` is the stage file, `here sha256` the\n")
        f.write("file in this repository.\n\n")
        f.write("| Here | Fork path (stage) | src sha256 | here sha256 | edits (clean-room changes, each with its reason) |\n| --- | --- | --- | --- | --- |\n")
        for e in manifest["files"]:
            parts = list(dict.fromkeys(x["reason"] for x in e.get("edits", [])))  # a reason shared by many edits once
            if e.get("widen"):
                parts.insert(0, "`pub(super)` widened to `pub(crate)` (the fork's parent was the crate root)")
            edits = "; ".join(parts) or "path rewrite only"
            f.write(f"| `{e['dest']}` | `{e['src']}` | `{e['src_sha256'][:16]}` | `{e['dest_sha256'][:16]}` | {edits} |\n")
        f.write(DELETED_SECTION)


main()
