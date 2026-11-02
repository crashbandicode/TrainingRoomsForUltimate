import sys, time
HERE = "/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work/eden"
src = open(HERE + "/route_local.py").read().replace("\nmain()\n", "\n")
ns = {"__file__": HERE + "/route_local.py", "__name__": "rl"}
exec(compile(src, HERE + "/route_local.py", "exec"), ns)
label, ref, box, thr, cmd = sys.argv[1], sys.argv[2], tuple(int(x) for x in sys.argv[3].split(",")), float(sys.argv[4]), sys.argv[5]
deadline = time.time() + 240
while time.time() < deadline:
    p = ns["snap"](label, "calm-wait")
    d = ns["dist"](p, ref, box)
    print(f"{time.strftime('%H:%M:%S', time.gmtime())}Z dist={d:.1f}", flush=True)
    if d <= thr:
        rc, out, err = ns["tr"](cmd, 120)
        print(out[-600:], err[-300:])
        break
    time.sleep(1)
else:
    print("never calm")