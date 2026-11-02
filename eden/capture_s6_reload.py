"""One lead-authorized s6-08 reproduction; arm before trigger, freeze on capture.

Run after install/bootcache/swap/kirby in the same boot. Does not grant acceptance.
The owner must collect, stop, restore and release after this bounded observation.
Uses the installed Eden one-hit tracepoint; no guest code or emulator changes.
"""
import json
import os
import re
import signal
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import rows_s6 as rows

sys.path.insert(0, "/home/intpa/Applications/Eden-GIF/current/tools")
from edenctl import Client

label = sys.argv[1]
run = Path("/home/intpa/trainingrooms-eden-pkg6/runs") / label
out = HERE / "runs" / label
out.mkdir(parents=True, exist_ok=True)
client = Client(run / "control.sock", timeout=2)


def request(method, params=None):
    reply = client.request(method, params)
    with (out / "tracepoint-requests.jsonl").open("a") as stream:
        stream.write(json.dumps({"time": time.time(), "method": method,
                                 "params": params, "reply": reply}) + "\n")
    if not reply.get("ok"):
        raise RuntimeError(reply)
    return reply["result"]


# A current guest instruction capture, matched against 64 bytes of owned main,
# supplies the mapping even when the launcher overrides Loader:Debug.
console = (run / "console.log").read_bytes()
mapping = json.loads((out / "main-mapping-verified.json").read_text())
assert mapping["guard_bytes"] == 64
base = int(mapping["main_base"], 16)
(out / "console-before-trigger.log").write_bytes(console)
manifest = json.loads((run / "run.json").read_text())
pid = int(manifest["verified_limits"]["MainPID"])
assert pid == mapping["eden_pid"]
command = Path(f"/proc/{pid}/cmdline").read_bytes().replace(b"\0", b" ")
assert b"eden" in command.lower(), command
address = base + 0x653350
armed = request("tracepoint.arm", {
    "address": hex(address), "registers": ["pc", "x0", "x8", "x9", "x12",
    "x19", "x21", "x22", "x23", "x24", "x25", "x30", "sp"],
    "memory_register": "x24", "memory_size": 32, "timeout_ms": 20000,
})
assert armed["state"] == "armed", "Unexpected pre-trigger hit; do not reload"
assert armed["process_id"] == mapping["process_id"]

# Exact r_r5_soak settings transition, without its one-second wait before capture.
live = json.loads((rows.SD / "settings.json").read_text()).get("menu", {})
settings = {"menu": {**live, **rows.BASE,
    "cycle_preload_slots": [0, 0, 1, 0, 0, 0],
    "cycle_preload_sources": [0, 0, 1], "cycle_preload_bank_members": [0, 0, 1]}}
(out / "trigger-settings.json").write_text(json.dumps(settings, indent=2) + "\n")
temporary = rows.SD / "diag-settings-next.tmp"
temporary.write_text(json.dumps(settings))
temporary.rename(rows.SD / "diag-settings-next.json")
deadline = time.monotonic() + 22
result = None
try:
    while time.monotonic() < deadline:
        result = request("tracepoint.status")
        if result["state"] != "armed":
            break
        time.sleep(0.005)
finally:
    # Freeze the owned emulator immediately, preserving diagnostics for collection.
    # The supervisor/claim remains owned; Down must follow, not another acceptance row.
    os.kill(pid, signal.SIGSTOP)
    receipt = {"main_base": hex(base), "address": hex(address), "pid_stopped": pid,
               "result": result, "acceptance": False, "time": time.time()}
    (out / "tracepoint-capture.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(json.dumps(receipt), flush=True)
