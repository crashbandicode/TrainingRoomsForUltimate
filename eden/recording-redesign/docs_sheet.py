#!/usr/bin/env python3
"""Contact sheet for a docs GIF: N frames spread over the clip, each labelled with its time.

python3 -I docs_sheet.py <clip.gif> <sheet.png> [frames=12] [cell_width=640]
python3 -I docs_sheet.py <clip.gif> --frame <ms> <out.png>      one full-size frame at that time
"""
import sys

from PIL import Image, ImageDraw, ImageSequence


def frames_of(path):
    out, at = [], 0
    with Image.open(path) as im:
        for frame in ImageSequence.Iterator(im):
            out.append((at, frame.convert('RGB')))
            at += int(frame.info.get('duration') or 100)
    return out, at


def sheet(path, dest, count=12, cell=640, cols=3):
    frames, total = frames_of(path)
    count = min(count, len(frames))
    picks = [frames[round(i * (len(frames) - 1) / max(count - 1, 1))] for i in range(count)]
    w, h = picks[0][1].size
    ch = round(cell * h / w)
    rows = (count + cols - 1) // cols
    out = Image.new('RGB', (cols * cell, rows * (ch + 18)), (24, 24, 24))
    draw = ImageDraw.Draw(out)
    for i, (at, frame) in enumerate(picks):
        x, y = (i % cols) * cell, (i // cols) * (ch + 18)
        out.paste(frame.resize((cell, ch)), (x, y + 18))
        draw.text((x + 4, y + 3), '%d ms / %d' % (at, total), fill=(255, 255, 0))
    out.save(dest)
    print('%s: %d frames, %d ms, %dx%d -> %s' % (path, len(frames), total, w, h, dest))


def one(path, ms, dest):
    frames, _ = frames_of(path)
    best = min(frames, key=lambda item: abs(item[0] - ms))
    best[1].save(dest)
    print('%s @%d ms -> %s' % (path, best[0], dest))


if __name__ == '__main__':
    if len(sys.argv) > 2 and sys.argv[2] == '--frame':
        one(sys.argv[1], int(sys.argv[3]), sys.argv[4])
    else:
        sheet(sys.argv[1], sys.argv[2], *(int(a) for a in sys.argv[3:5]))
