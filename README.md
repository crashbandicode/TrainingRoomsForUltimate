# TrainingRoomsForUltimate

A Skyline plugin for Super Smash Bros. Ultimate 13.0.5 that adds practice
tools to Training mode and to the online waiting rooms.

**This is a clean-room implementation inspired by the Training Modpack.**
The Training Modpack ([jugeeya/UltimateTrainingModpack](https://github.com/jugeeya/UltimateTrainingModpack))
publishes no license, so none of its code, layouts or artwork is used here.
Features that originate in the Training Modpack were re-specified from their
observable behaviour by spec writers, and then implemented by people and agents
who worked from those specifications only and did not read the Training
Modpack's source. The hot-swap engine, Mii tools, CPU-as-Player-1 handoff,
cycle preloading, JSON saves, practice stages, lobby practice and the seam
guards were written for the earlier fork by this project's authors and are
carried over as they are.

Everything in this repository is licensed under the [MIT License](LICENSE).

This project was developed with the assistance of AI.

## Status

Phase 0 (provenance) and the start of phase 1 (feature inventory and
behavioural specifications). Nothing is built or installable yet. See
[PROVENANCE.md](PROVENANCE.md) for the fork-versus-upstream table,
[TRAININGROOMS-HANDOFF.md](TRAININGROOMS-HANDOFF.md) for the live state, and
`specs/` for the behavioural specifications. The menu proposal is
[docs/MENU-PROPOSAL.md](docs/MENU-PROPOSAL.md); the decisions still open for the
operator, one picture each, are in [docs/DECISIONS.md](docs/DECISIONS.md).

## Layout (planned)

The plugin follows the workspace playbook layout: a `cdylib` only; pure logic in
`model`-side crates with host tests; hooks in a `runtime` module compiled only
for the Switch; a game-version guard and a 64-byte guard on every hooked site;
a `diag` feature for Eden diagnostics that the shipped NRO does not carry; one
install line in `sd:/ultimate/TrainingRooms/events.log` and a size-capped
diagnostics file.

All of the plugin's files live under `sd:/ultimate/TrainingRooms/`. On its
first boot on a card that has the earlier fork's `sd:/ultimate/TrainingModpack/`
folder, it imports the menu settings, save states and setups, Mii presets and
named macro library once, by copying; the originals are left untouched
(`specs/SPEC-core-08-storage-root-and-migration.md`).

## Build and install

Not yet. The build commands, the install path and the evidence documents will
be added when the first implementation package lands.
