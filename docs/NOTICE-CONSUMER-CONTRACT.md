# Startup and save notice consumers

2026-11-03. Source implementation; host/native/runtime qualification pending.

The approved SPEC-core-01 section 2 requires one first-Training-HUD sequence:
TrainingRooms / Welcome! for 60 visible frames, then Open Menu, Save State,
Load State, Input Record and Input Playback with their current chord text for
120 visible frames each. SPEC-state-01 sections 2 and 5 require replacing the
previous Save State notice with Saved Slot N for 120 visible frames.

Before this slice, features/states.rs saved_notice only emitted a diag line and
discarded the slot in release. No first-HUD startup consumer existed. The existing
bounded Queue and UI draw were present; the missing link was the producers.

The pure boot_notices helper consumes its boot edge only when a real native
Training notice pane and published settings exist. Layout/session clear does not
reset that once-per-process state. It formats current chord values in fixed
buffers, includes the permanent B + D-pad Up chord even if optional shortcuts
are disabled, and uses the existing copied-body limit. Queue countdown pauses
behind the open menu according to the existing notification contract.

The UI callback reads the published settings and shortcut atomics before the
queue locks. Only the UI callback owns BOOT_NOTICES, and a pending atomic skips
the snapshot/boot lock after the first enqueue. The hook-lock inventory includes
this new lock; no publishing or file-worker path may reach it. No native hook,
native call, heap allocation or filesystem operation is added.

The state hook hands only the saved slot through the existing bounded mailbox.
The UI drain removes earlier Save State messages and posts the new static body.
An invalid slot is rejected without disturbing other notices.

Original regressions in tests/input05.rs cover deferred first-HUD readiness,
once-per-process behavior across queue clear, actual published chord values,
exact visible duration and ordering, menu-paused time, permanent shortcut
availability, save replacement/other-message preservation, every numbered slot,
invalid input, and first/repeated allocation counts. The next coherent targeted
gate must run input05 and hook_locks, plus exact native UI callback stack
qualification and fresh screenshots of startup/save notices. None is claimed
passed by this document.