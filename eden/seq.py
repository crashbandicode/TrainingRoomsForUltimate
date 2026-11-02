import sys, time
sys.path.insert(0, "/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work/eden")
import importlib.util
spec = importlib.util.spec_from_file_location("rl", "/home/intpa/smash-training-lobby/TrainingRoomsForUltimate-work/eden/route_local.py")
src = open(spec.origin).read().replace("\nmain()\n", "\n")
ns = {"__file__": spec.origin, "__name__": "rl"}
exec(compile(src, spec.origin, "exec"), ns)
label, n, gap = sys.argv[1], int(sys.argv[2]), float(sys.argv[3])
t0 = time.time()
for i in range(n):
    p = ns["snap"](label, f"seq{i:02d}")
    print(f"{time.strftime('%H:%M:%S', time.gmtime())}Z +{time.time()-t0:5.1f}s {p.name}", flush=True)
    time.sleep(gap)