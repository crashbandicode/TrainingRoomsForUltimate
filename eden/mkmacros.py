import json, sys, copy
base = json.load(open(sys.argv[1]))
out = sys.argv[2]
def ev(frame, buttons=0, sticks=(0,0,0,0)):
    return {"buttons": buttons, "frame": frame,
            "motion": [{"accel":[0,0,0],"delta_us":10000,"gyro":[0,0,0]}]*2,
            "sticks": list(sticks), "triggers": [0,0]}
def make(name, events, duration):
    d = {k: copy.deepcopy(base[k]) for k in ("bindings","format","game_id","parent","player","version")}
    d["name"] = name; d["events"] = events; d["markers"] = []; d["duration_frames"] = duration
    json.dump(d, open(f"{out}/{name}.edenmacro.json","w"), indent=1)
make("tr-jump-probe", [ev(0), ev(60, 4), ev(70, 0)], 140)
make("tr-walk-right", [ev(0), ev(10, 0, (0.45,0,0,0)), ev(50, 0, (0,0,0,0))], 60)
make("tr-lra-chord", [ev(0), ev(10, 64+128+1), ev(40, 0)], 60)
make("tr-idle", [ev(0)], 30)
# --- added later (same helpers):
# make('tr-lra-hold-right', [ev(0), ev(10, 193, (0.45,0,0,0)), ev(90, 0, (0,0,0,0))], 100)

# tr-walk-chord: ev(0), ev(10,0,(0.45,0,0,0)), ev(50,0,(0,0,0,0)), ev(70,193), ev(100,0); duration 170
