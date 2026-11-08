"""rc.5 Fighter Mismatch proof (TASK-OPUS-rc5-mismatch): the question is drawn, Keep and Switch each work, then the GIF.

Each row copies the `mismatch` fixture (a CPU chain whose macro 1 was recorded on Pikachu; the CPU is Mario), presses
Play on CPU, and checks the gates log for the question and for the answer line. The shots are the visual proof.
"""
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import accept  # noqa: E402
import docsgif  # noqa: E402
import steps  # noqa: E402

ASK = r'screen=mismatch ask=macro1 rows='


def _ask(ctx):
    docsgif.setup_mismatch(ctx)
    offset = steps._gate_offset(ctx)
    ctx.play(steps.PLAY_CPU)
    line = steps.diag(ctx, ASK, cap=10)
    time.sleep(1.2)
    return offset, line, str(ctx.shot('prompt'))


def _after(ctx, word):
    answer = steps.diag(ctx, r'screen=mismatch ask=macro1 answer=%s' % word, cap=10)
    time.sleep(0.6)
    shots = [str(ctx.shot('after-%s-1' % word))]
    time.sleep(1.5)
    shots.append(str(ctx.shot('after-%s-2' % word)))
    return answer, shots


def keep(ctx):
    _, line, prompt = _ask(ctx)
    steps.press(ctx, 'a', 0.2)
    answer, shots = _after(ctx, 'keep')
    time.sleep(2.0)
    path = Path(steps.gate_source(ctx))
    asks = path.read_text(errors='replace').count('screen=mismatch ask=macro1 rows=')
    return steps.save(ctx, 'keep', {'ask': line, 'prompt_shot': prompt, 'answer': answer, 'after': shots,
                                    'asks_in_log': asks})


def switch(ctx):
    _, line, prompt = _ask(ctx)
    steps.press(ctx, 'down', 0.4)
    shot_down = str(ctx.shot('prompt-switch-row'))
    steps.press(ctx, 'a', 0.2)
    answer, shots = _after(ctx, 'switch')
    time.sleep(4.0)
    shots.append(str(ctx.shot('after-switch-3')))
    return steps.save(ctx, 'switch', {'ask': line, 'prompt_shot': prompt, 'switch_row_shot': shot_down,
                                      'answer': answer, 'after': shots})


def gif_setup(ctx):
    return docsgif.setup_mismatch(ctx)


def gif_clip(ctx):
    time.sleep(0.6)
    ctx.play(steps.PLAY_CPU)
    time.sleep(1.6)
    steps.press(ctx, 'a', 0.2)
    time.sleep(3.6)


def cpu_back_to_mario(ctx):
    accept.prep(ctx, 'empty')
    return steps.cpu_mario(ctx)
