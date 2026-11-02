#!/home/intpa/ssbu-recomp/deps/venv/bin/python3 -I
"""Render the SDI Up chip layer with the spark's burst stretched along the SDI vector (proposal v10).

Review-mockup tooling, never shipped. It re-runs the c1162 W9 chip renderer (`di-sdi-extract/tools/render_chips.py`,
read-only, imported from the W9 folder; needs numpy, so the ssbu-recomp venv Python) on the local stock 13.0.5
`ef_common.eff` and writes, to the output directory, the chosen file `sdi-up-4x-layer-stretch.png` and the compared
variants `variants/sdi-up-4x-layer-x<factor>.png`. Nothing is written into the W9 folder, and no .eff, texture, mesh
or parameter file is copied anywhere.

What changes against W9's `sdi-up-4x-layer.png` (one still at particle life t = 0.3): only the SDI burst's
`impactring1` emitter gets a taller scale-Y; its scale-X, the thin line (`spark1`, W9's calibrated quad), the origin,
the colours, the display gain, the framing (units per half chip) and the other emitter are unchanged. Two sources:
  * factor 1.17: the emitter's own scale-key table read at life 0.5 instead of 0.3 (y key 1.356 -> 1.584; the
    alpha is still 1.0 there). This is the file data's own stretch.
  * factors 1.4, 1.7, 2.0: the key table ends at y = 1.93 (x1.42 of the life-0.3 value) at life 1.0, where the alpha
    has faded to 0.14, so a larger stretch is not in the file for a solid burst: S.y of the life-0.3 still is
    multiplied by the factor. CALIBRATED against the captures, not file data.
The burst's width is untouched, so its height / width goes up by the factor (1.48 at life 0.3).

Self-check: with factor 1 the same code must reproduce W9's `sdi-up-4x-layer.png` pixel for pixel.

Usage: <venv python> -I tools/render_sdi_stretch.py <W9 di-sdi-extract dir> <out dir>
Prints one JSON record (chosen file, variants, parameters, pinned input hashes) for crops.json.
"""
import hashlib
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # importing W9's modules must not leave __pycache__ in the evidence folder

BASE_T = 0.3            # W9's still
# (factor, how): "key" = the emitter's own key table at KEY_LIFE; "scale" = S.y multiplied by the factor
VARIANTS = [(1.17, "key"), (1.4, "scale"), (1.7, "scale"), (2.0, "scale")]
KEY_LIFE = 0.5
CHOSEN = 2.0            # the variant the SDI chips draw (see docs/menu-mockup/index.md)
TOOL_SHA = "b123bc3baa7ba3c47d963327f756d6fc086433df2f9003303544558e73e7b0bb"   # W9 tools/render_chips.py
EFF_SHA = "8ff5bad0681cc922dfc4988acfb0212def0514d717bb62b4df17b916d04e0023"    # stock 13.0.5 ef_common.eff
W9_UP_SHA = "9d665a9051a1f54030ed53529f2eba1c18bf1a3196f5fdffe75b1b54fa061254"  # out/chips/sdi-up-4x-layer.png
NAME = "sdi-up-4x-layer-stretch.png"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    w9, out = Path(sys.argv[1]), Path(sys.argv[2])
    for path, want in ((w9 / "tools" / "render_chips.py", TOOL_SHA), (w9 / "re" / "eff" / "ef_common.eff", EFF_SHA),
                       (w9 / "out" / "chips" / "sdi-up-4x-layer.png", W9_UP_SHA)):
        if sha(path) != want:
            sys.exit(f"{path}: sha256 {sha(path)} != pinned {want}")
    sys.path.insert(0, str(w9 / "tools"))
    import numpy as np
    from PIL import Image
    import render_chips as rc

    data = (w9 / "re" / "eff" / "ef_common.eff").read_bytes()
    params = rc.read_set(data, "P_CmnHitStopSpark")
    texs, meshes = rc.load_assets(data, params)
    reach, _side = rc.extent(params, BASE_T, meshes)
    upc = reach / (rc.FILL * 2)  # W9's framing: the furthest reach (30 units) lands at 0.46 x chip size
    size = 108 * 4
    gain = rc.display_gain(rc.render_still(params, texs, meshes, BASE_T, size, 90, upc))
    base_state = rc.emitter_state
    ring = next(e for e in params["emitters"] if e["name"] == "impactring1")
    key = lambda t: rc.interp(ring["scale_keys"], t, (0, 1))[1]  # noqa: E731

    def layer(ring_life=BASE_T, factor=1.0):
        def patched(e, t):
            st = base_state(e, t)
            if e["name"] == "impactring1":
                st["scale"] = (st["scale"][0], e["base_scale"][1] * key(ring_life) * factor)
            return st
        rc.emitter_state = patched
        try:
            light = rc.render_still(params, texs, meshes, BASE_T, size, 90, upc)
        finally:
            rc.emitter_state = base_state
        return rc.rgba_layer(np.clip(gain * light, 0, 1))

    ref = Image.open(w9 / "out" / "chips" / "sdi-up-4x-layer.png")
    if not (np.array(layer()) == np.array(ref)).all():
        sys.exit("self-check failed: factor 1 does not reproduce W9's sdi-up-4x-layer.png")
    (out / "variants").mkdir(parents=True, exist_ok=True)
    variants, chosen = [], None
    for factor, how in VARIANTS:
        im = layer(KEY_LIFE, 1.0) if how == "key" else layer(BASE_T, factor)
        f = out / "variants" / f"sdi-up-4x-layer-x{factor}.png"
        im.save(f)
        rec = {"file": f"renders/variants/{f.name}", "sha256": sha(f), "size": [size, size], "factor": factor,
               "source": f"emitter key table at life {KEY_LIFE} (y key {key(BASE_T):.3f} -> {key(KEY_LIFE):.3f})"
                         if how == "key" else f"life-0.3 S.y x {factor} (calibrated)"}
        variants.append(rec)
        if factor == CHOSEN:
            im.save(out / NAME)
            chosen = {"file": f"renders/{NAME}", "sha256": sha(out / NAME), "size": [size, size], "factor": factor,
                      "how": how, "same_pixels_as": rec["file"]}
    print(json.dumps({
        **chosen, "variants": variants,
        "tool": "tools/render_sdi_stretch.py (imports W9 tools/render_chips.py)", "tool_sha256": TOOL_SHA,
        "eff_sha256": EFF_SHA, "base_render_sha256": W9_UP_SHA, "base_life": BASE_T, "key_life": KEY_LIFE,
        "self_check": "factor 1 reproduces W9 pixel for pixel",
    }))


if __name__ == "__main__":
    main()
