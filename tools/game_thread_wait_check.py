#!/usr/bin/env python3
"""rc.8 "Add New…" hang (worker A's finding A3, 2026-11-11): the proof on the built ELF that no game-thread path reaches a
primitive that spins on a peer's in-flight state.

The game's MainThread spun in std's `sync_channel` `try_recv` (array `start_recv` -> `Backoff::spin_light`), inlined into
the native service poll, while the menu worker it had preempted on core 0 held the slot it waited for. Every function a
hook or a game callback reaches by direct calls (`tools/stack_depth.py`'s roots: each `#[skyline::hook]` and C-ABI
callback under src/runtime, minus the plugin threads' own entries in `threads.rs`; machine-outlined bodies followed) must
not be one of:

  - `std::sync::mpmc::*` / `std::sync::mpsc::*` (std's channels: send and receive spin on a peer mid-operation);
  - `<std::sync::mpmc::utils::Backoff>::spin_light` / `spin_heavy` (their spin, wherever inlined);
  - anything from `crossbeam`.

    python3 -I tools/game_thread_wait_check.py <libtraining_rooms.so> [--repo <checkout>] [--paths N]

Exit 0: none reached; 1: each reached function with one call path from its root; 2: the ELF could not be read."""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import stack_depth as sd  # noqa: E402

# A function OF std's channels (its path or its impl's type starts there), not one that merely names a type from them in
# its generics: `drop_in_place::<std::sync::mpsc::TrySendError<..>>` drops std's plain error enum, which `nowait_channel`
# returns. Any channel code such a function reaches is a function of std's channels itself, and is caught.
FORBIDDEN = re.compile(r"^<?(std::sync::mpmc::|std::sync::mpsc::)|Backoff>::spin_(light|heavy)\b|crossbeam")
PLUGIN_THREAD_STEMS = {"threads"}


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        sys.exit(2)
    elf = args[0]
    repo = args[args.index("--repo") + 1] if "--repo" in args else "."
    paths = int(args[args.index("--paths") + 1]) if "--paths" in args else 12
    try:
        funcs = sd.analyse(elf)
    except Exception as error:  # noqa: BLE001 - any failure to read the ELF is exit 2
        print(f"cannot read {elf}: {error}")
        sys.exit(2)
    hooks = [(name, stem) for name, stem in sd.hook_names(repo) if stem not in PLUGIN_THREAD_STEMS]
    roots = sd.hook_roots(funcs, hooks)
    if not roots:
        print(f"no hook roots found in {elf} from {repo}/src/runtime")
        sys.exit(2)
    parent = {}
    order = []
    for root in sorted(roots):
        if root in parent:
            continue
        parent[root] = None
        stack = [root]
        while stack:
            a = stack.pop()
            order.append(a)
            for c in sorted(funcs[a]["calls"]):
                if c not in parent:
                    parent[c] = a
                    stack.append(c)
    hits = sorted((a for a in order if FORBIDDEN.search(funcs[a]["name"])), key=lambda a: funcs[a]["name"])
    print(f"{len(roots)} game-thread roots (hooks and callbacks), {len(order)} reachable functions, {len(hits)} forbidden")
    for a in hits[:paths]:
        chain = []
        x = a
        while x is not None:
            chain.append(funcs[x]["name"])
            x = parent[x]
        chain.reverse()
        print(f"FORBIDDEN 0x{a:x} {funcs[a]['name'][:160]}")
        for name in chain:
            print(f"    {name[:160]}")
    if len(hits) > paths:
        for a in hits[paths:]:
            print(f"FORBIDDEN 0x{a:x} {funcs[a]['name'][:160]}")
    sys.exit(1 if hits else 0)


if __name__ == "__main__":
    main()
