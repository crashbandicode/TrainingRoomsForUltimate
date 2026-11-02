"""Side by side: a mockup screen (docs/menu-mockup/NN-name.png, left) and the in-game screenshot of the same screen (right),
each scaled to 960 x 540, with a caption bar. Review material only (the mockup holds game captures; nothing here is committed).

    python3 -I eden/compare_s5.py <out.png> <mockup.png> <ingame.png> [caption]
"""
import sys
from pathlib import Path

from PIL import Image, ImageDraw

W, H = 960, 540


def main():
    out, mock, shot = (Path(p) for p in sys.argv[1:4])
    caption = sys.argv[4] if len(sys.argv) > 4 else out.stem
    canvas = Image.new("RGB", (W * 2 + 6, H + 22), (30, 30, 30))
    for i, path in enumerate((mock, shot)):
        image = Image.open(path).convert("RGB").resize((W, H), Image.LANCZOS)
        canvas.paste(image, (i * (W + 6), 22))
    draw = ImageDraw.Draw(canvas)
    draw.text((6, 5), f"mockup: {mock.name}", fill=(220, 220, 120))
    draw.text((W + 12, 5), f"in game (Ally Eden): {shot.name}   {caption}", fill=(120, 220, 220))
    out.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(out)
    print(out)


main()
