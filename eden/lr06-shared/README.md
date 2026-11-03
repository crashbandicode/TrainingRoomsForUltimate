# lr-06 — warm scene-end loop stopped at a camera-dependent gate

**10 PASS / 1 FAILED / 11 unrun. RC-TRIO-END remains open.**

Shared Up loaded Yoga20:42:18.120617Z after PRIMARY's renewed short-loop
order. Shared Down in the caller's finally restored/separately verified and
released at20:45:02.203949Z. No row continued after the failure.

## Findings

Bootstrap09rows passed. `cycle1-warm-groups` passed correlated fresh Player
Falcon and CPU MiiGunner submit/ready/released predicates and changed/stable
HUD gates. Reviewed images show matching bodies/HUDs. The Gunner image shows
the scoped Fighters notice `Loaded 5/6 · swap the CPU to load 1 more`; this
is not full notice or pointed saved-Mii identity acceptance.

`cycle1-warm-return` first passed `training-hud-unpaused`, then failed the
implicit `training-closed` start gate of library `pause-open`. Its ROI
`[75,120,300,190]` is Training-stage grid background, which changes with the
camera. The failed screenshot clearly shows unpaused Falcon/Gunner at0%.
No pause, quit, warm-finalize or reentry input was issued in this row.
All three planned scene-end/reentry cycles and final continuation remain
unqualified. The inherited native-only prose in that row's `evidence` field
is retained verbatim; the new warm-finalize gates were never reached.

Next route correction: use a project native-quit hook that gates on the
already-qualified undimmed0% HUD, presses Plus, verifies actual native pause,
then reuses the library quit-focus/confirm recordings and checks clean warm
finalization/CSS. Keep the camera-grid failure and default tolerance8; no
product change or claim that the failed predicate passed.

## Closure and provenance

Shared cleanup is **PASS** under current playbook EDEN.md: cleanup_failures=[],
restoration/separateverify equal, owned release. Expected capped game.stop
terminal deadline remains in stop_errors; forced stop completed. Independent
21410-entry source AND profile equality,252 immutable links/20436 private
mutable files,0violations,0owned claims/game/displayPIDs/originalidentities.
Peak6301728768B below8GiB. Fullconsole822286B/5813lines,0Unmapped and0
panic/abort/crash matches; transferred SHA
`1edd827c34ba9c3cc0fa5bd75739b1fb15a5ffbdc2527d10f5b09c84bbadf84e` verified.

ProductRC8cf4bf44/diagnostic3ca7f0c5 unchanged. Playbook403f517a7927533290fdaaa582c519b77c12277c,
exact tool/hook/library identities inprovenance.json. All11receipts bind
byte-exact rows-executed.json; the shared runner's new manifests/ snapshot
also preserves that serialization. Portable rows.json freezes all library
values and paths. No shared engine copied into the product repository.

Shared harvest adds0segments/0gates; the passed navigation/group dependencies
already exist. Structural archive dry-run22rows/53gates/20recordings READY
does not qualify the failed route. Raw full outputs, tools, snapshots,
console/control logs and compliance are under
`/home/intpa/smash-training-lobby/artifacts/trainingrooms-lead-astra4/rc8/lr06-shared/`.

Phase1's13open top-levelrows and additionalR3 clauses remain assigned in
LEAD-INBOX → Lead → Phase 1 remaining rows. Tracing's requested Yoga package/
installed-smoke turn remains active; every new short loop needs strict fresh
admission. lr06 stays closed. No new productbuild/publication/device action.
