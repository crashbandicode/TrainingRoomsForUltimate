import hashlib,json,subprocess,sys,time
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
import cov,gna,gna_p2_rec as rec,steps

def macro_resume(ctx):
    root=cov.card(ctx)
    source_bytes=(ctx.row_dir/'source-macro.json').read_bytes()
    source=json.loads(source_bytes)
    source_path=root/'macros'/'macro-1.json'
    assert source_path.read_bytes()==source_bytes
    frames=cov.diag(ctx)
    # The first probe held each direction for150ms, triggering the editor's
    # documented held acceleration. Reset physically, then use short taps.
    ctx.press('l',ms=40)
    ctx.call('input.button',{'button':'zl','state':'press','lease_ms':5000})
    for _ in range(15):
        ctx.press('left',ms=40)
        time.sleep(.12)
    ctx.call('input.button',{'button':'zl','state':'release'})
    frames.wait(r'\[UiClipFrame\] published [^\n]*start=0 ',cap=6)
    out={'source_id':'macro-1','source_sha256':hashlib.sha256(source_bytes).hexdigest(),
         'before':str(ctx.shot('macro-resume-full-range')),'correction':'150ms probe taps repeated;40ms direction taps retain exact20-frame coarse movement'}
    camera_dir=ctx.row_dir/'crop-capture2'
    with (ctx.row_dir/'capture2.log').open('w') as log:
        camera=subprocess.Popen([sys.executable,str(Path(__file__).with_name('capture_eden.py')),
            '--socket',ctx.instance['socket'],'--output-dir',str(camera_dir),'--seconds','30','--fps','5','--trigger','none'],stdout=log,stderr=subprocess.STDOUT)
        time.sleep(.7)
        try:
            ctx.call('input.button',{'button':'zl','state':'press','lease_ms':5000})
            for n in range(1,10):
                ctx.press('right',ms=40)
                time.sleep(.12)
                frames.wait(r'\[UiClipFrame\] published [^\n]*start=%d ' % (n*20),cap=3)
            ctx.call('input.button',{'button':'zl','state':'release'})
            out['selection']=str(ctx.shot('macro-resume-kept-range'))
            preview=cov.diag(ctx)
            ctx.press('x',ms=80)
            out['frame_preview']=frames.wait(r'\[UiClipFrame\] published [^\n]*playing=true',cap=6)
            out['preview_started']=str(ctx.shot('macro-resume-preview-started'))
            out['preview_complete']=preview.wait(r'\[ClipPreview\] inputs_complete revision=\d+ frames=\d+',cap=18)
            out['preview']=str(ctx.shot('macro-resume-preview-complete'))
            cov.real_keyboard(ctx,True)
            prior=rec._macro_ids(ctx)
            ctx.press('a',ms=80)
            time.sleep(.5)
            out['keyboard']=cov.keyboard_type(ctx,'Kept jab 2')
            assert out['keyboard']['path']=='eden-keyboard'
            created=cov.wait_until(lambda:sorted(rec._macro_ids(ctx)-prior),cap=6)
            assert len(created or [])==1,created
            new_id=created[0]
            path=root/'macros'/(new_id+'.json')
            kept=json.loads(path.read_text())
            assert kept['frames']==source['frames'][180:],(len(kept['frames']),len(source['frames']))
            assert source_path.read_bytes()==source_bytes
            (ctx.row_dir/'kept-macro.json').write_bytes(path.read_bytes())
            out.update(new_id=new_id,new_frames=len(kept['frames']),cutoff=180,original_unchanged=True,
                saved=str(ctx.shot('macro-resume-saved-new')))
        finally:
            camera.wait(timeout=40)
            cov.real_keyboard(ctx,False)
    ctx.press('b',ms=80)
    time.sleep(.8)
    out['frame_hidden']=frames.wait(r'\[UiClipFrame\] hidden seq=\d+',cap=6)
    out['closed']=str(ctx.shot('macro-resume-editor-closed'))
    (ctx.row_dir/'macro-frame-publish.log').write_text(frames.text())
    offset,title=steps.open_library_item(ctx,True)
    steps.down_to_file_row(ctx,offset,title,'Play on CPU')
    play=cov.diag(ctx)
    steps.press(ctx,'a',.4)
    out['play_new']=play.wait(r'screen=playback role=cpu',cap=8)
    gna._close(ctx)
    time.sleep(3)
    out['play_result']=str(ctx.shot('new-macro-played'))
    assert source_path.read_bytes()==source_bytes
    return cov.save(ctx,'macro-crop',out)
