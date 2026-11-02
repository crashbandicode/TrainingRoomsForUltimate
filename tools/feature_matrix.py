#!/usr/bin/env python3
"""Compile every cargo feature of the plugin (decision D11: every feature in the table must build).

The feature table of `Cargo.toml` is the source: this lists (and, with `--run`, compiles) a set of feature combinations:

  set `core`   default; no default features; default + diag; the three heap/allocator features alone and together
               (`console-practice-trace`, `native-unlink-probe`, `plugin-heap-meter`: the fork gave each its own global allocator;
               the plugin has one, `tests/pkg6_features.rs::the_plugin_has_one_global_allocator`)
  set `each`   every feature of the table on top of the default features
  set `alone`  every feature of the table by itself with `--no-default-features` (the feature's own `requires` closure only)
  set `all`    core + each + alone

`--mode host` runs `cargo check` (the pure model and the host tests' view of the features); `--mode switch` runs
`rustup run skyline-v3 cargo check` with build-std, the Skyline target spec and `--cfg skyline_std_v3` (what cargo-skyline runs; `cargo skyline check` takes no feature flags): the runtime and the carried Switch-only game code, the target every probe feature is for. The command prefix
(the host's memory and time guard, the campaign rustup home) is yours: `--runner "systemd-run --user --scope -p MemoryMax=6G
/usr/bin/timeout 1800"`; `--cargo` names the cargo (or cargo-skyline) executable. One cargo call each, `CARGO_TERM_COLOR=never`.

    python3 -I tools/feature_matrix.py --list                       # print the combinations, run nothing
    python3 -I tools/feature_matrix.py --run --mode switch --set core --cargo <rustup> --target-spec <aarch64-skyline-switch.json> --runner "..."
    python3 -I tools/feature_matrix.py --run --mode host --set all   # exit code 1 when any combination fails to build

Never run it beside another stream's Eden on the same host (docs/BUILD.md; the stream rules): check builds are builds.
"""
import argparse
import os
import shlex
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def read_features(manifest_text):
    """The `[features]` table of Cargo.toml as {name: [requires]} (one-line entries, as `tests/pkg6_features.rs` parses them)."""
    table, inside = {}, False
    for line in manifest_text.splitlines():
        if line.startswith("["):
            inside = line.strip() == "[features]"
            continue
        line = line.strip()
        if not inside or not line or line.startswith("#"):
            continue
        name, value = line.split("=", 1)
        value = value.strip()
        if not (value.startswith("[") and value.endswith("]")):
            raise SystemExit(f"feature table line is not one-line: {line}")
        table[name.strip()] = [d.strip().strip('"') for d in value[1:-1].split(",") if d.strip()]
    return table


CORE = [
    ("default", []),
    ("no-default-features", ["--no-default-features"]),
    ("default+diag", ["--features", "diag"]),
    # No `--all-features`: the fork's probe features are mutually exclusive by design (`compile_error!` pairs, e.g. the results
    # observer and the native adapter hook the same sites), so the union is not a product or a diag build.
    ("trace", ["--features", "diag,console-practice-trace"]),
    ("swap-trace", ["--features", "diag,console-swap-trace"]),
    ("unlink-probe", ["--features", "diag,native-unlink-probe"]),
    ("heap-meter", ["--features", "diag,plugin-heap-meter"]),
    ("trace+unlink+meter", ["--features", "diag,console-practice-trace,native-unlink-probe,plugin-heap-meter"]),
    ("hang-trace", ["--features", "diag,hang-trace"]),
]


# The fork's mutual exclusions, by design (each is a `compile_error!` in the carried sources): a combination whose feature closure
# holds both sides is not a build anyone makes, so the matrix skips exactly these and nothing else.
EXCLUSIONS = [
    ("automation-results-observer", ("automation-results-native",),
     "src/runtime/lobby/automation_helper.rs:7: results observer and native adapter both hook 0x150e5f0 and 0x150e9d0; enable only one"),
    ("native-body-bank-probe", ("native-body-bank-alpha",),
     "src/runtime/roster/character_roster.rs:32: The fixed bank probe and managed bank candidate cannot share native hooks"),

]


def closure(table, roots):
    seen, stack = set(), list(roots)
    while stack:
        name = stack.pop()
        if name in seen:
            continue
        seen.add(name)
        stack.extend(d for d in table.get(name, []) if "/" not in d and ":" not in d)
    return seen


def excluded(table, flags):
    """The fork's exclusion a combination falls under, or None."""
    roots = [] if "--no-default-features" in flags else ["default"]
    if "--features" in flags:
        roots += flags[flags.index("--features") + 1].split(",")
    enabled = closure(table, roots)
    for one, others, why in EXCLUSIONS:
        if one in enabled and any(o in enabled for o in others):
            return why
    return None


def combinations(table, which):
    out = []
    if which in ("core", "all"):
        out += CORE
    if which in ("each", "all"):
        out += [(f"default+{name}", ["--features", name]) for name in sorted(table)]
    if which in ("alone", "all"):
        out += [(f"alone:{name}", ["--no-default-features", "--features", name]) for name in sorted(table)]
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--set", choices=["core", "each", "alone", "all"], default="core")
    ap.add_argument("--mode", choices=["host", "switch"], default="host")
    ap.add_argument("--list", action="store_true", help="print the combinations and run nothing")
    ap.add_argument("--run", action="store_true", help="compile each combination (cargo check)")
    ap.add_argument("--runner", default="", help="command prefix, e.g. the systemd-run memory guard")
    ap.add_argument("--cargo", default="cargo", help="cargo (host) or the rustup executable (switch: `rustup run skyline-v3 cargo check`)")
    ap.add_argument("--target-spec", default="", help="switch: the Skyline target spec, aarch64-skyline-switch.json")
    ap.add_argument("--only", default="", help="substring filter on the combination label")
    ap.add_argument("--root", default=ROOT)
    ap.add_argument("--log-dir", default="", help="write the whole output of each failing combination here")
    args = ap.parse_args()
    with open(os.path.join(args.root, "Cargo.toml"), encoding="utf-8") as f:
        table = read_features(f.read())
    for name, deps in table.items():
        for dep in deps:
            if dep not in table:
                raise SystemExit(f"feature {name} requires undeclared {dep}")
    combos = [c for c in combinations(table, args.set) if args.only in c[0]]
    if args.list or not args.run:
        for label, flags in combos:
            print(label, " ".join(flags))
        print(f"{len(combos)} combinations over {len(table)} features", file=sys.stderr)
        return 0
    env = dict(os.environ, CARGO_TERM_COLOR="never", CARGO_TERM_HYPERLINKS="false")
    if args.mode == "switch":
        # `cargo skyline check` takes no feature flags, so run what cargo-skyline runs for a build, as a check: the skyline-v3
        # toolchain through rustup, build-std, the Skyline target spec and the `skyline_std_v3` cfg (docs/BUILD.md).
        if not args.target_spec:
            raise SystemExit("--mode switch needs --target-spec <aarch64-skyline-switch.json>")
        env["RUSTFLAGS"] = "--cfg skyline_std_v3"
        base = shlex.split(args.runner) + [args.cargo, "run", "skyline-v3", "cargo", "check", "-Z", "json-target-spec",
                                           "--target", args.target_spec, "-Z", "build-std=core,alloc,std,panic_abort"]
    else:
        base = shlex.split(args.runner) + [args.cargo, "check"]
    failed = []
    skipped = []
    for label, flags in combos:
        why = excluded(table, flags)
        if why:
            skipped.append(label)
            print(f"skip  {label}: the fork's exclusion, {why}", flush=True)
            continue
        started = time.time()
        proc = subprocess.run(base + flags, cwd=args.root, env=env, capture_output=True, text=True)
        text = (proc.stdout + proc.stderr).strip().splitlines()
        tail = [line for line in text if line.startswith("error") or line.lstrip().startswith("-->")][:24] or text[-6:]
        if proc.returncode != 0 and args.log_dir:
            os.makedirs(args.log_dir, exist_ok=True)
            with open(os.path.join(args.log_dir, label.replace(":", "_") + ".log"), "w", encoding="utf-8") as f:
                f.write("\n".join(text) + "\n")
        status = "ok" if proc.returncode == 0 else "FAIL"
        print(f"{status:4} {time.time() - started:6.1f}s {args.mode} {label}", flush=True)
        if proc.returncode != 0:
            failed.append(label)
            print("\n".join("     " + line for line in tail), flush=True)
    built = len(combos) - len(failed) - len(skipped)
    print(f"{built}/{len(combos) - len(skipped)} combinations build ({len(skipped)} skipped by the fork's exclusions)"
          + (f"; failed: {', '.join(failed)}" if failed else ""))
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
