"""sync-01 continued (same boot): the publish from the diag reload thread with Macro Source = Named Library (pkg4's s4-07 path), the
named playback of the loaded macro, then more menu close cycles with the named buffers reserved, and the counters.

    rows_sync01b.py live-sync-01 [cycles]
"""
import json
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import ui_lent as u  # noqa: E402
import rows_sync01 as r  # noqa: E402

PHASE = sys.argv[1]
CYCLES = int(sys.argv[2]) if len(sys.argv) > 2 else 15
r.PHASE = PHASE
SD = r.SD


def main():
    # a whole (small) settings file through a rename, as diag_reload.rs asks; every other row takes its factory value
    m = r.count("diag_settings_reload")
    u.ps(f"[IO.File]::WriteAllText('{SD}\\diag-settings-next.tmp', '{{\"menu\":{{\"macro_source\":[0,1]}}}}'); "
         f"Move-Item -Force '{SD}\\diag-settings-next.tmp' '{SD}\\diag-settings-next.json'; 'written'", 60)
    reloaded = r.wait_more("diag_settings_reload", m, 15)
    m = r.count(r"\[NamedMacroDiag\] status")
    r.macro_cmd("status")
    r.wait_more(r"\[NamedMacroDiag\] status", m, 8)
    st = (r.lines(r"\[NamedMacroDiag\] status", 1) or ["none"])[-1]
    rl = (r.lines("diag_settings_reload", 1) or ["none"])[-1]
    r.row("publish from the diag reload thread (Macro Source = Named Library) completes; the game keeps running",
          reloaded and "named_source=true" in st, f"{rl[:90]} | {st[:120]}")
    r.fps_sample("after-named-publish")
    before = u.ps(f"(Get-Item '{SD}\\save_states.json' -ErrorAction SilentlyContinue).LastWriteTimeUtc.Ticks", 60).strip()
    m = r.count("file_written")
    u.ps(f"Set-Content -Path '{SD}\\diag-save.req' -Value 'x'", 60)
    written = r.wait_more("file_written", m, 10)
    after = u.ps(f"(Get-Item '{SD}\\save_states.json' -ErrorAction SilentlyContinue).LastWriteTimeUtc.Ticks", 60).strip()
    r.row("Save State: the writer thread writes save_states.json from its own copy (stream/sync (a))", written and after != before,
          f"file_written line {written}, mtime {before or 'none'} -> {after}")
    time.sleep(1.0)
    m = r.count("named_playback")
    u.keys(PHASE, "zr+up")
    played = r.wait_more("named_playback", m, 20)
    done = r.wait_more("named_playback_complete", r.count("named_playback_complete") - 1 if played else 10**9, 25)
    r.row("named playback of the loaded macro (synctest) runs and completes", played and done, " | ".join(x[:120] for x in r.lines("named_playback", 2)))
    u.shot(PHASE, f"after-named-playback-b{int(time.time())}")
    m0 = r.count(r"\[Ui\] menu closed")
    ok = 0
    for i in range(CYCLES):
        o, c = r.count(r"\[Ui\] menu opened"), r.count(r"\[Ui\] menu closed")
        u.keys(PHASE, "b+up")
        if not r.wait_more(r"\[Ui\] menu opened", o, 10):
            break
        u.keys(PHASE, "w5 b")
        if not r.wait_more(r"\[Ui\] menu closed", c, 10):
            r.row(f"named cycle {i + 1}: menu closed", False, "no close line in 10 s")
            break
        r.fps_sample(f"named-{i + 1}")
        ok += 1
        u.keys(PHASE, "w8")
    r.row(f"menu open/close cycles with Macro Source = Named ({ok}/{CYCLES})", ok == CYCLES, f"closes {m0} -> {r.count(r'[Ui] menu closed')}")
    stats = r.lines(r"hook_stats", 2)
    r.row("hook allocations (hook_stats)", bool(stats), stats[-1][:260] if stats else "no hook_stats line")
    out = r.OUT
    (out / "rows-b.json").write_text(json.dumps(r.ROWS, indent=1))
    (out / "fps-b.json").write_text(json.dumps(r.FPS, indent=1))
    print("FPS", json.dumps(r.FPS[:2]), "...", json.dumps(r.FPS[-2:]), flush=True)
    print(f"ROWS {sum(x['ok'] for x in r.ROWS)}/{len(r.ROWS)} pass", flush=True)


if __name__ == "__main__":
    main()
