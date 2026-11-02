#!/bin/bash
# TrainingRooms package 2 (and any TrainingRooms stream): the gates and both Switch builds on the Ally's WSL (a remote
# Linux helper; adapted from worker-d/ally-run.sh and W20's ally-setup-w20.sh). Run it with `wsl -e bash <file> <stream>`
# from an MSI-pwsh snippet (see eden/ALLY-SETUP.md); never while eden.exe runs on the Ally.
#
#   ally-build.sh unpack <pack dir> <worktree name>   verify src.tar.gz against src.sha256, unpack at the Surface's absolute
#                                                    path (tar -m: fresh mtimes), copy the owned-module fixtures
#   ally-build.sh gates  <worktree name> <out dir>    cargo test, clippy -D warnings, no-default-features, Switch release and
#                                                    diag builds, nro_size; NROs and logs into <out dir> (a /mnt/c path)
set -uo pipefail
TOOLS=/home/intpa/smash-training-lobby/toolchain/tools
TC=$TOOLS/rustup/toolchains/1.98.1-x86_64-unknown-linux-gnu
NTC=$TOOLS/rustup/toolchains/nightly-2026-02-14-x86_64-unknown-linux-gnu
SKY=$TOOLS/skyline-install/bin/cargo-skyline
BASE=/home/intpa/smash-training-lobby
LOCK=/tmp/c1161-ally-compiler.lock

mode=$1
case $mode in
unpack)
  pack=$2; name=$3
  cd "$pack" || exit 5
  [ "$(sha256sum src.tar.gz | cut -d' ' -f1)" = "$(cut -d' ' -f1 src.sha256)" ] || { echo "pack hash mismatch"; exit 5; }
  free=$(df -BG --output=avail / | tail -1 | tr -dc 0-9)
  [ "$free" -ge 40 ] || { echo "STOP: ${free}G free"; exit 76; }
  rm -rf "$BASE/$name"
  tar -m -C "$BASE" -xzf "$pack/src.tar.gz"
  mkdir -p "$BASE/research/game-13.0.5/nro"
  [ -f "$pack/lua2cpp_common.nro" ] && cp -n "$pack/lua2cpp_common.nro" "$BASE/research/game-13.0.5/nro/"
  ls "$BASE/$name" | head -20
  echo unpack-ok
  ;;
gates)
  name=$2; out=$3
  work=$BASE/$name
  target=/home/intpa/c1161-targets/$name-ally
  mkdir -p "$out" "$target" /home/intpa/tmp-$name
  job() { # label toolchain-bin timeout cmd...
    local label=$1 tcb=$2 limit=$3; shift 3
    if tasklist.exe /FI "IMAGENAME eq eden.exe" < /dev/null 2>/dev/null | grep -qi 'eden.exe'; then echo "eden.exe runs: stop"; return 99; fi
    env -i HOME=/home/intpa USER=intpa \
      PATH="$TOOLS/skyline-install/bin:$tcb/bin:/home/intpa/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
      RUSTUP_HOME=$TOOLS/rustup CARGO_HOME=$TOOLS/cargo CARGO_TARGET_DIR=$target CARGO_BUILD_JOBS=4 \
      CARGO_TERM_COLOR=never CARGO_TERM_HYPERLINKS=false \
      RUSTUP_TOOLCHAIN=$(basename "$tcb") TMPDIR=/home/intpa/tmp-$name SKIP_EDEN_SCRIPT_TESTS=1 \
      bash -c 'cd "$0" && exec flock '"$LOCK"' timeout '"$limit"' "$@"' "$work" "$@" < /dev/null > "$out/$label.log" 2>&1
    local code=$?
    echo "$label exit=$code" | tee -a "$out/GATES.txt"
    return $code
  }
  : > "$out/GATES.txt"
  c=$TC/bin/cargo
  job host-test "$TC" 1800 "$c" test --no-fail-fast
  job host-clippy "$TC" 1800 "$c" clippy --all-targets -- -D warnings
  job host-nodefault "$TC" 1800 "$c" build --no-default-features
  job switch-release "$NTC" 1800 "$SKY" skyline build --release
  cp "$target/aarch64-skyline-switch/release/libtraining_rooms.nro" "$out/libtraining_rooms.release.nro" 2>/dev/null
  job switch-diag "$NTC" 1800 "$SKY" skyline build --release --features diag
  cp "$target/aarch64-skyline-switch/release/libtraining_rooms.nro" "$out/libtraining_rooms.diag.nro" 2>/dev/null
  (cd "$out" && sha256sum *.nro > NRO.sha256; cat NRO.sha256)
  grep -h -c '^warning' "$out"/switch-*.log | tr '\n' ' '; echo "(warning lines: release diag)"
  grep -h 'test result' "$out/host-test.log" | awk '{p+=$4; f+=$6} END {print "tests passed=" p " failed=" f}'
  echo gates-done
  ;;
esac
