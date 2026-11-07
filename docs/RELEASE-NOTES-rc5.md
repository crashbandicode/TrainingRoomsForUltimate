# 0.2.0-rc.5 — draft release notes

Draft for players. It describes the recording redesign and the frame strip as they stand on `p23/rr-docs`, plus the pop-up, the Recording page, and the stick row, which are still being built. Sentences about those screens and that row are marked. This is not a release tag.

In a match on this branch, Load warps and starts nothing. Play on CPU is still the older slot or named-library playback. Play on Player 1 and the recording pop-up are chords only: the match does not act on them yet. The per-role machine described below is in the model.

## Frame strip

The input display is the frame strip: the last second of Player 1 and the CPU, a chip for each new button, and readouts for advantage, the link, and the timing of the press. How to read it, including what ADVANTAGE means, is [FRAME-STRIP.md](FRAME-STRIP.md).

ADVANTAGE is the CPU's first actionable frame minus yours after the last hit. Plus means you act first. The line says on hit or on shield. Actionable includes cancel windows, so an aerial can read more plus than a frame-data listing that waits out the landing lag.

A follow-up more than 20 frames after the CPU can act is a new exchange, not a dropped link.

Under Player 1's lane, a stick row shows the held direction on each frame. <!-- verify: rr-11 -->
A C-stick attack takes its chip arrow from the C-stick. <!-- verify: rr-11 -->
A press with the C-stick neutral keeps the left-stick arrow. <!-- verify: rr-11 -->

## Recording

How to use it is [RECORDING.md](RECORDING.md). The short version:

- A save state only warps. Load does not start a recording. Auto-load on death only warps to the bound save, and it does nothing when none is bound.
- Recording a macro also writes that macro's start save. The Play chord warps there once, when the switch is On, and then plays.
- Each role has one active macro, a recording or a chain. Play on Player 1 and Play on CPU play that role's macro.
- The loop never warps back. Later passes continue from where the fighters are.
- A chain starts at its first macro's start save, plays its recordings in order, and cannot contain a chain.
- Save last combo still saves the combo you landed. A drilled combo counts without having to be a true combo. A different move drops. Make combo and crop are on the file's own screen. <!-- verify: rr-6 -->
- A sequence is an ordered list of combos, and the helper drills one thing at a time.
- The pop-up and the new Recording page are the way you bind, set active, and drill. <!-- verify: rr-6 -->

## Chords

| Chord | Default | Change |
| --- | --- | --- |
| Save | ZL + D-pad Down | Writes the bound save. |
| Load | ZL + D-pad Up | Warps to the bound save. Starts nothing. |
| Record | ZR + D-pad Down | Same chord. The pop-up's role chooses who is recorded. <!-- verify: rr-6 --> |
| Play on CPU | ZR + D-pad Up | Was Input Playback. Plays the CPU's active macro. |
| Play on Player 1 | ZR + D-pad Left | New. Plays Player 1's active macro. |
| Recording pop-up | ZR + D-pad Right | New. Opens the pop-up. <!-- verify: rr-6 --> |
| Cycle CPU | ZL + D-pad Left | Unchanged. |
| Cycle Player | ZL + D-pad Right | Unchanged. |

Open menu stays B + D-pad Up. Menu shortcut 1 stays Minus. Both Play chords may be held at once. An old settings file that does not have the two new chords still loads, and those chords appear at the defaults above.

The first time Training opens on a new install, the six reminders are still Welcome, Open Menu, Save State, Load State, Input Record, and Input Playback. This branch does not post a replacement sentence. The redesign's wording for that one-time note is "Record, Play on Player 1, Play on CPU, and the recording pop-up replace Input Record and Input Playback."

## Migration

On the first build of the new library:

- Empty recording slots and empty save indexes are skipped.
- A used recording slot becomes a macro named `Slot N (YYYY-MM-DD)`, with the date the migration runs. It has no start save, so Play runs it from where the fighters are.
- A used save index becomes one save file of both fighters, with the same kind of name. The slot Save and Load were using becomes the bound save.
- The named library's selected file becomes Player 1's active macro.
- Nothing is turned into a chain. Sequences and chains start empty.
- Loop On becomes infinite. Loop Off, or no loop row, becomes 1. Player 1 and the CPU then keep their own counts.

## What leaves

The playback machine no longer starts a recording because a save loaded. That job is the macro's start save, started by the role's Play chord.

These rows leave the menu with the new Recording page. <!-- verify: rr-6 -->
The slot number and Randomize slots leave. <!-- verify: rr-6 -->
Record Slot, Trigger, and the Duration dropdown leave. <!-- verify: rr-6 -->
Button slots and the save-state playback checklist leave. <!-- verify: rr-6 -->
Source leaves. <!-- verify: rr-6 -->
Per-slot playback chords are not added. <!-- verify: rr-6 -->
The setting ids stay in the file, so older settings still open. <!-- verify: rr-6 -->

The Saves page keeps the chord switch, mirroring, auto-load on death, damage on load, buffs, and the setup library. Setups stay a separate library from save states. Respawn on platform stays under Fighters & Stages › Stages.

Pins are gone. The five most recently used of each kind are the ones the list outlines. <!-- verify: rr-6 -->
Y does nothing on the new recording screens. <!-- verify: rr-6 -->

## Not in this draft

No console capture is attached yet. The shots the notes above point at are the list in [MEDIA.md](MEDIA.md).
