import hashlib,json,sys,time
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
import cov,gna,steps

def confirm_play(ctx):
    root=cov.card(ctx)
    src=(ctx.row_dir/'source-macro.json').read_bytes()
    kept=json.loads((root/'macros/kept-jab-2.json').read_text())
    assert kept['frames']==json.loads(src)['frames'][180:]
    assert (root/'macros/macro-1.json').read_bytes()==src
    play=cov.diag(ctx)
    ctx.press('a',ms=80) # observed existing Fighter Mismatch: Keep current fighter
    played=play.wait(r'screen=playback role=cpu',cap=8)
    time.sleep(3)
    return cov.save(ctx,'macro-crop',{'cutoff':180,'source_frames':314,'new_frames':len(kept['frames']),
      'new_id':'kept-jab-2','original_unchanged':True,'play_new':played,
      'play_result':str(ctx.shot('new-macro-played')),'frame_log':'macro-frame-publish.log',
      'continued_after':'existing Fighter Mismatch prompt: Keep current fighter'})

def finish(ctx):
    root=cov.card(ctx)
    src=(ctx.row_dir/'source-macro.json').read_bytes()
    kept_path=root/'macros/kept-jab-2.json'
    kept=json.loads(kept_path.read_text())
    assert (root/'macros/macro-1.json').read_bytes()==src
    assert kept['frames']==json.loads(src)['frames'][180:]
    lines=(ctx.row_dir/'macro-frame-publish.log').read_text()
    assert 'start=180 end=314' in lines and 'playing=true' in lines and '[UiClipFrame] hidden seq=' in lines
    # Existing file menu offers Set active; playback is the configured chord.
    ctx.press('b',ms=80)
    time.sleep(.3)
    ctx.press('zr',ms=80)
    time.sleep(.3)
    ctx.press('a',ms=80)
    time.sleep(.4)
    ctx.press('down',ms=80)
    time.sleep(.2)
    ctx.press('a',ms=80)
    time.sleep(.5)
    index=json.loads((root/'recording-index.json').read_text())
    assert index['active']['cpu']=='kept-jab-2',index
    gna._close(ctx)
    play=cov.diag(ctx)
    ctx.play('chord-play-cpu')
    played=play.wait(r'screen=playback role=cpu',cap=8)
    time.sleep(3)
    assert (root/'macros/macro-1.json').read_bytes()==src
    return cov.save(ctx,'macro-crop',{'cutoff':180,'source_frames':314,'new_frames':len(kept['frames']),
       'new_id':'kept-jab-2','original_unchanged':True,'source_sha256':hashlib.sha256(src).hexdigest(),
       'selection_log':'macro-frame-publish.log','play_new':played,'play_result':str(ctx.shot('new-macro-played')),
       'continuations':'prior crop/save/B verified; Set active CPU plus existing playback chord replaces nonexistent Play on CPU file-row assumption'})
