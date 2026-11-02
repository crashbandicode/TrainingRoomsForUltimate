"""Write the package-2 settings files (`eden/settings/p2-*.json`): partial `menu` objects, every other row factory.
Row lengths and index orders are the model's (`src/model/factory.rs`, `src/model/menu/options.rs`).

Usage: mk_settings.py [outdir]
"""
import json
import sys
from pathlib import Path

LEN = {
    "mash_state": 30, "mash_triggers": 17, "attack_angle": 3, "throw_state": 5, "throw_delay": 31, "pummel_delay": 31,
    "full_hop": 2, "aerial_delay": 31, "fast_fall": 2, "oos_offset": 31, "reaction_time": 31, "ledge_roll_override": 30,
    "air_dodge_dir": 11, "di_state": 11, "sdi_state": 11, "sdi_strength": 4, "clatter_strength": 4, "ledge_state": 10,
    "ledge_delay": 31, "tech_state": 4, "miss_tech_state": 4, "shield_state": 4, "shield_tilt": 11, "crouch": 2,
    "stale_dodges": 2, "tech_hide": 2, "frame_advantage": 2, "settings_override_playback": 2,
}
DIRECTIONS = ["Out", "Up Out", "Up", "Up In", "In", "Down In", "Down", "Down Out", "Neutral", "Left", "Right"]
ACTIONS = ["Air Dodge", "Jump", "Shield", "Spot Dodge", "Roll Forwards", "Roll Backwards", "Neutral Air", "Forward Air",
           "Back Air", "Up Air", "Down Air", "Neutral Special", "Side Special", "Up Special", "Down Special", "Forward Smash",
           "Up Smash", "Down Smash", "Jab", "Forward Tilt", "Up Tilt", "Down Tilt", "Grab", "Dash", "Dash Attack",
           "Playback Slot 1", "Playback Slot 2", "Playback Slot 3", "Playback Slot 4", "Playback Slot 5"]
TRIGGERS = ["Hitstun", "Shieldstun", "Parry", "Tumble", "Landing", "Ledge Trump", "Footstool", "Clatter", "Ledge Option",
            "Tech Option", "Mistech Option", "Grounded", "Airborne", "Distance: Close", "Distance: Mid", "Distance: Far", "Always"]
LEDGE = ["Neutral Getup", "Roll", "Jump", "Getup Attack", "Wait", "Playback Slot 1"]
STRENGTH = ["None", "Normal", "Medium", "High"]
SHIELD = ["None", "Infinite", "Hold", "Constant"]
THROW = ["None", "Forward Throw", "Backward Throw", "Up Throw", "Down Throw"]
TECH = ["No Tech", "Roll Forwards", "Roll Backwards", "Tech In Place"]
MISTECH = ["Neutral Getup", "Getup Attack", "Roll Forwards", "Roll Backwards"]


def row(id_, *chosen, names=None):
    """A row with the given indices (or names, looked up in `names`) set to 1 (a checklist or a one-hot)."""
    out = [0] * LEN[id_]
    for c in chosen:
        out[names.index(c) if isinstance(c, str) else c] = 1
    return out


def settings(**rows):
    return {"menu": rows}


def build():
    p = {}
    # Factory plus the toggles the install line reports; DI / SDI / airdodge / clatter / staling (SPEC-cpu-02).
    p["p2-defence"] = settings(
        di_state=row("di_state", "Out", names=DIRECTIONS),
        sdi_state=row("sdi_state", "Up", names=DIRECTIONS),
        sdi_strength=row("sdi_strength", "High", names=STRENGTH),
        air_dodge_dir=row("air_dodge_dir", "Down Out", names=DIRECTIONS),
        clatter_strength=row("clatter_strength", "High", names=STRENGTH),
        stale_dodges=[0, 1],
    )
    p["p2-defence-b"] = settings(  # the second half of the B11 / Q18 A row: SDI changes to In, no reset
        di_state=row("di_state", "Up", names=DIRECTIONS),
        sdi_state=row("sdi_state", "In", names=DIRECTIONS),
        sdi_strength=row("sdi_strength", "Normal", names=STRENGTH),
        air_dodge_dir=row("air_dodge_dir", "Up Out", names=DIRECTIONS),
        clatter_strength=row("clatter_strength", "Normal", names=STRENGTH),
        stale_dodges=[1, 0],
    )
    p["p2-ledge-roll"] = settings(ledge_state=row("ledge_state", "Roll", names=LEDGE), ledge_delay=row("ledge_delay", 6))
    p["p2-ledge-wait"] = settings(ledge_state=row("ledge_state", "Wait", names=LEDGE))
    p["p2-ledge-empty"] = settings(ledge_state=[0] * 10)
    # Tech / mistech / hide tech (SPEC-cpu-04): tech in place everywhere, mistech getup attack, hide on.
    p["p2-tech"] = settings(tech_state=row("tech_state", "Roll Backwards", names=TECH),
                            miss_tech_state=row("miss_tech_state", "Getup Attack", names=MISTECH), tech_hide=[1, 0])
    p["p2-tech-no-hide"] = settings(tech_state=row("tech_state", "Tech In Place", names=TECH),
                                    miss_tech_state=row("miss_tech_state", "Roll Forwards", names=MISTECH), tech_hide=[0, 1])
    # Shield (SPEC-cpu-05): Infinite shield; then Hold, None.
    p["p2-shield-infinite"] = settings(shield_state=row("shield_state", "Infinite", names=SHIELD),
                                       shield_tilt=row("shield_tilt", "Up", names=DIRECTIONS))
    p["p2-shield-hold"] = settings(shield_state=row("shield_state", "Hold", names=SHIELD),
                                   shield_tilt=row("shield_tilt", "Down", names=DIRECTIONS))
    # Mash and the rest of the reactive rows (SPEC-cpu-01, SPEC-cpu-06, SPEC-cpu-07).
    p["p2-mash"] = settings(mash_state=row("mash_state", "Up Smash", names=ACTIONS),
                            mash_triggers=row("mash_triggers", "Hitstun", names=TRIGGERS),
                            frame_advantage=[1, 0])
    p["p2-mash-aerial"] = settings(mash_state=row("mash_state", "Jump", names=ACTIONS), mash_triggers=row("mash_triggers", "Always", names=TRIGGERS),
                                   fast_fall=[0, 1], full_hop=[1, 0], aerial_delay=row("aerial_delay", 4))
    p["p2-throw"] = settings(throw_state=row("throw_state", "Up Throw", names=THROW), throw_delay=row("throw_delay", 3),
                             pummel_delay=row("pummel_delay", 2), mash_state=row("mash_state", "Grab", names=ACTIONS),
                             mash_triggers=row("mash_triggers", "Always", names=TRIGGERS))
    p["p2-mash-off"] = settings(mash_state=[0] * 30, mash_triggers=[0] * 17)
    return p


def main():
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent / "settings"
    out.mkdir(parents=True, exist_ok=True)
    for name, body in build().items():
        for key, value in body["menu"].items():
            assert len(value) == LEN[key], (name, key, len(value))
        (out / f"{name}.json").write_text(json.dumps(body, separators=(",", ":")) + "\n", encoding="utf-8")
        print(name)


if __name__ == "__main__":
    main()
