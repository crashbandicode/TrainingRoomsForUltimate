#!/usr/bin/python3
"""Acceptance checks for the rc.5 merge (p23/rc5-merge).

tips_are_ancestors: each finished tip is an ancestor of this checkout.
no_branch_test_lost: cargo test -- --list names at each tip are a subset of this checkout,
and a test that ran there is not #[ignore] here.
switch_release_nro_recorded: the release NRO exists and its sha256 is printed.
"""
import hashlib
import os
import subprocess
import sys

TIPS = (
    "95c6c884cbeb",
    "a1c342f95b96",
    "ac07a34fcbfc",
    "676ba3abb184",
    "2ceeefd6cdf5",
    "966d683de4a5",
    "53f6decf1588",
    "faaf9a81ddc259334f8c6c9d47005e13077b00d3",
)


def git(root, *args):
    return subprocess.run(["git", *args], cwd=root, check=True, capture_output=True, text=True).stdout


def tips_are_ancestors(root):
    """Exit 0 when every finished tip is an ancestor of HEAD."""
    head = git(root, "rev-parse", "HEAD").strip()
    failed = 0
    for tip in TIPS:
        full = git(root, "rev-parse", tip).strip()
        result = subprocess.run(["git", "merge-base", "--is-ancestor", full, "HEAD"], cwd=root)
        mark = "ancestor" if result.returncode == 0 else "NOT-ANCESTOR"
        if result.returncode != 0:
            failed += 1
        print(f"{mark} {full} {head}")
    return failed


def _names(text):
    running, ignored = set(), set()
    for line in text.splitlines():
        if line.endswith(": test"):
            running.add(line[: -len(": test")].strip())
        elif line.endswith(": ignore"):
            ignored.add(line[: -len(": ignore")].strip())
    return running, ignored


def _cargo_list(crate, target):
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = target
    env.setdefault("CARGO_BUILD_JOBS", "2")
    env.setdefault("CARGO_INCREMENTAL", "0")
    cargo = env.get("CARGO", "/home/intpa/.cargo/bin/cargo")
    result = subprocess.run(
        [cargo, "test", "--locked", "--", "--list"],
        cwd=crate, env=env, capture_output=True, text=True,
    )
    sys.stderr.write(result.stderr)
    if result.returncode != 0:
        sys.stderr.write(result.stdout)
        raise SystemExit(f"cargo test --list failed in {crate}")
    return _names(result.stdout)


def no_branch_test_lost(root):
    """Exit 0 when every tip's cargo test names are a subset of HEAD, and none were ignored."""
    target = os.environ.get(
        "CARGO_TARGET_DIR",
        "/home/intpa/smash-training-lobby/tmp-p23/rc5-merge-list-target",
    )
    head_run, head_ignore = _cargo_list(root, target)
    print(f"HEAD running {len(head_run)} ignored {len(head_ignore)}")
    failed = 0
    for tip in TIPS:
        full = git(root, "rev-parse", tip).strip()
        path = os.path.join(os.environ.get("TMPDIR", "/tmp"), f"rc5-list-{full[:12]}")
        if not os.path.exists(os.path.join(path, ".git")):
            git(root, "worktree", "add", "--detach", path, full)
        try:
            run, ignored = _cargo_list(path, target)
        finally:
            subprocess.run(["git", "worktree", "remove", "--force", path], cwd=root, check=False)
        missing = sorted(run - head_run - head_ignore)
        weakened = sorted((run & head_ignore) - ignored)
        print(f"{full[:12]} running {len(run)} missing {len(missing)} weakened {len(weakened)}")
        for name in missing:
            print(f"  missing {name}")
            failed += 1
        for name in weakened:
            print(f"  ignored {name}")
            failed += 1
    return failed


def switch_release_nro_recorded(path):
    """Print the release NRO's sha256 and byte length. Exit 0 when the file is a non-trivial NRO."""
    data = open(path, "rb").read()
    if len(data) < 1024 or data[:4] != b"NRO0":
        print(f"not an NRO: {path} ({len(data)} bytes)")
        return 1
    print(hashlib.sha256(data).hexdigest(), len(data), path)
    return 0


def main(argv):
    if len(argv) >= 3 and argv[2] in ("tips_are_ancestors", "no_branch_test_lost", "switch_release_nro_recorded"):
        root, cmd, extra = os.path.abspath(argv[1]), argv[2], argv[3:]
    else:
        root, cmd, extra = os.getcwd(), (argv[1] if len(argv) > 1 else ""), argv[2:]
    if cmd == "tips_are_ancestors":
        return tips_are_ancestors(root)
    if cmd == "no_branch_test_lost":
        return no_branch_test_lost(root)
    if cmd == "switch_release_nro_recorded" and extra:
        return switch_release_nro_recorded(extra[0])
    print("usage: rc5_merge_accept.py [root] tips_are_ancestors|no_branch_test_lost|switch_release_nro_recorded [nro]", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
