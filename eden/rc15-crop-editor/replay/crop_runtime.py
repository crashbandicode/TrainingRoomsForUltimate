"""rc.15 real Crop acceptance. Uses the existing runner and recorded setup segments."""
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import cov
import gna
import gna_p2_rec as rec
import steps


def configure(ctx):
    import gdb_rc8
    out = {}
    for key, value in [('core.volume', 0), ('core.memory_layout_mode', 0)]:
        ctx.call('settings.set', {'key': key, 'value': value})
        out[key] = ctx.call('settings.get', {'key': key})
    ctx.call('settings.save', {})
    out['gdb'] = gdb_rc8.gdb_on(ctx)
    return cov.save(ctx, 'boot-settings', out)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def macro_crop(ctx):
    root = cov.prep(ctx, 'cov-crop')
    gna._base(ctx, recording_crop=gna.OFF)
    rec._popup_role_player(ctx)
    rec._ensure_notice_seen(ctx)
    rec._loop_to(ctx, 1)

    def attempts():
        ctx.press('x', ms=150)  # deliberate unwanted jump at the start
        time.sleep(1.8)
        ctx.play(steps.JAB)     # the attempt to keep
        time.sleep(0.5)

    source_id, source, recorded = rec._record(ctx, attempts, 0.3)
    source_path = root / 'macros' / (source_id + '.json')
    before = source_path.read_bytes()
    out = {'recorded': recorded, 'source_id': source_id, 'source_sha256': digest(source_path)}
    active = [i for i, frame in enumerate(source['frames']) if not rec._idle(frame)]
    assert active, ('no recorded input', recorded)
    final_start = active[-1]
    while final_start > 0 and not rec._idle(source['frames'][final_start - 1]):
        final_start -= 1
    assert final_start >= 20, ('no separated final input', recorded)
    cutoff = ((final_start - 2) // 20) * 20
    assert 0 < cutoff < len(source['frames']), (cutoff, recorded)
    out['cutoff'] = cutoff
    (ctx.row_dir / 'source-macro.json').write_bytes(before)
    offset, title = steps.open_library_item(ctx, True)
    steps.down_to_file_row(ctx, offset, title, 'Crop')
    frames = cov.diag(ctx)
    steps.press(ctx, 'a', 0.8)
    status = cov._named_req(ctx, 'status', r'\[NamedMacroDiag\] status [^\n]*')
    assert 'editor=true' in (status or ''), status
    out['before'] = str(ctx.shot('macro-editor-before'))
    out['frame_open'] = frames.wait(r'\[UiClipFrame\] published [^\n]*start=0 [^\n]*playing=false', cap=6)

    camera_dir = ctx.row_dir / 'crop-capture'
    camera_log = open(ctx.row_dir / 'capture.log', 'w')
    camera = subprocess.Popen([sys.executable, str(Path(__file__).with_name('capture_eden.py')),
        '--socket', ctx.instance['socket'], '--output-dir', str(camera_dir), '--seconds', '30',
        '--fps', '5', '--trigger', 'none'], stdout=camera_log, stderr=subprocess.STDOUT)
    time.sleep(0.7)
    try:
        steps.press(ctx, 'l', 0.2)
        ctx.call('input.button', {'button': 'zl', 'state': 'press', 'lease_ms': 5000})
        for _ in range(cutoff // 20):
            steps.press(ctx, 'right', 0.2)
        ctx.call('input.button', {'button': 'zl', 'state': 'release'})
        out['frame_selection'] = frames.wait(r'\[UiClipFrame\] published [^\n]*start=%d ' % cutoff, cap=6)
        out['selection'] = str(ctx.shot('macro-editor-kept-range'))
        preview = cov.diag(ctx)
        steps.press(ctx, 'x', 0.1)
        out['frame_preview'] = frames.wait(r'\[UiClipFrame\] published [^\n]*playing=true', cap=6)
        out['preview_started'] = str(ctx.shot('macro-preview-started'))
        out['preview_complete'] = preview.wait(r'\[ClipPreview\] inputs_complete revision=\d+ frames=\d+', cap=18)
        out['preview'] = str(ctx.shot('macro-preview-complete'))
        cov.real_keyboard(ctx, True)
        prior = rec._macro_ids(ctx)
        steps.press(ctx, 'a', 0.5)
        out['keyboard'] = cov.keyboard_type(ctx, 'Kept jab')
        assert out['keyboard']['path'] == 'eden-keyboard', out['keyboard']
        created = cov.wait_until(lambda: sorted(rec._macro_ids(ctx) - prior), cap=6)
        assert len(created or []) == 1, created
        new_id = created[0]
        kept_path = root / 'macros' / (new_id + '.json')
        kept = json.loads(kept_path.read_text())
        assert kept['name'] == 'Kept jab', kept['name']
        assert kept['frames'] == source['frames'][cutoff:], (len(kept['frames']), len(source['frames']))
        assert source_path.read_bytes() == before, 'source macro changed'
        out.update(new_id=new_id, new_frames=len(kept['frames']), original_unchanged=True,
                   kept_sha256=digest(kept_path), saved=str(ctx.shot('macro-saved-as-new')))
        (ctx.row_dir / 'kept-macro.json').write_bytes(kept_path.read_bytes())
    finally:
        camera.wait(timeout=40)
        camera_log.close()
        cov.real_keyboard(ctx, False)
    steps.press(ctx, 'b', 1.0)
    out['frame_hidden'] = frames.wait(r'\[UiClipFrame\] hidden seq=\d+', cap=6)
    out['closed'] = str(ctx.shot('macro-editor-closed'))
    (ctx.row_dir / 'macro-frame-publish.log').write_text(frames.text())
    offset, title = steps.open_library_item(ctx, True)
    steps.down_to_file_row(ctx, offset, title, 'Play on CPU')
    play = cov.diag(ctx)
    steps.press(ctx, 'a', 0.4)
    out['play_new'] = play.wait(r'screen=playback role=cpu', cap=8)
    gna._close(ctx)
    time.sleep(3.0)
    out['play_result'] = str(ctx.shot('new-macro-played'))
    assert source_path.read_bytes() == before, 'source changed after playback'
    return cov.save(ctx, 'macro-crop', out)


def combo_crop(ctx):
    root = cov.card(ctx)
    source_path = root / 'combos' / 'combo-1.json'
    before = source_path.read_bytes()
    source = json.loads(before)
    before_ids = {p.name for p in source_path.parent.glob('*.json')}
    out = {'source_sha256': digest(source_path), 'source_links': len(source['links'])}
    cov.open_combo_file(ctx)
    steps.press(ctx, 'down', 0.25)  # Rename
    steps.press(ctx, 'down', 0.25)  # Crop
    steps.press(ctx, 'a', 0.8)
    status = cov._named_req(ctx, 'status', r'\[NamedMacroDiag\] status [^\n]*')
    assert 'editor=true' in (status or ''), status
    out['before'] = str(ctx.shot('combo-editor-before'))
    frames = cov.diag(ctx)
    ctx.press('r', ms=40)
    ctx.press('left', ms=40)
    out['frame_selection'] = frames.wait(r'\[UiClipFrame\] published [^\n]*start=0 end=1 ', cap=6)
    out['selected'] = str(ctx.shot('combo-one-link'))
    steps.press(ctx, 'x', 0.25)
    out['after_x'] = str(ctx.shot('combo-no-x-action'))
    cov.real_keyboard(ctx, True)
    steps.press(ctx, 'a', 0.5)
    out['keyboard'] = cov.keyboard_type(ctx, '')
    assert out['keyboard']['path'] == 'eden-keyboard', out['keyboard']
    created = cov.wait_until(lambda: sorted({p.name for p in source_path.parent.glob('*.json')} - before_ids), cap=6)
    assert len(created or []) == 1, created
    kept_path = source_path.parent / created[0]
    kept = json.loads(kept_path.read_text())
    assert kept['links'] == source['links'][:1], kept
    assert kept['name'].startswith('Combo '), kept['name']
    assert source_path.read_bytes() == before, 'source combo changed'
    out.update(new_file=created[0], name=kept['name'], links=len(kept['links']), original_unchanged=True,
               saved=str(ctx.shot('combo-saved-new')))
    (ctx.row_dir / 'source-combo.json').write_bytes(before)
    (ctx.row_dir / 'kept-combo.json').write_bytes(kept_path.read_bytes())
    steps.press(ctx, 'b', 0.8)
    cov.real_keyboard(ctx, False)
    return cov.save(ctx, 'combo-crop', out)


def replay_clip_redraw(ctx):
    """Regression of the Replay panel entry into the same standalone Clip phase."""
    frames = cov.diag(ctx)
    out = {'continued_from': 'visible Fighter Mismatch Keep; prior failed receipt preserved'}
    ctx.press('a', ms=40) # observed Keep current fighter on this pinned fixture
    out['frame_open'] = frames.wait(r'\[UiClipFrame\] published [^\n]*playing=false', cap=8)
    out['before'] = str(ctx.shot('replay-editor-before'))
    ctx.press('l', ms=40)
    ctx.press('right', ms=40)
    out['frame_selection'] = frames.wait(r'\[UiClipFrame\] published [^\n]*start=1 ', cap=6)
    out['selected'] = str(ctx.shot('replay-handle-redrawn'))
    preview = cov.diag(ctx)
    steps.press(ctx, 'x', 0.1)
    out['frame_preview'] = frames.wait(r'\[UiClipFrame\] published [^\n]*playing=true', cap=6)
    out['preview_started'] = str(ctx.shot('replay-preview-started'))
    out['preview_complete'] = preview.wait(r'\[ClipPreview\] inputs_complete revision=\d+ frames=\d+', cap=30)
    out['preview_done'] = str(ctx.shot('replay-preview-complete'))
    steps.press(ctx, 'b', 0.8)
    out['frame_hidden'] = frames.wait(r'\[UiClipFrame\] hidden seq=\d+', cap=6)
    out['closed'] = str(ctx.shot('replay-editor-closed'))
    (ctx.row_dir / 'replay-frame-publish.log').write_text(frames.text())
    return cov.save(ctx, 'replay-redraw', out)
