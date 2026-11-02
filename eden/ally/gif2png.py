"""Write one frame of an Eden GIF as a PNG for review (read-only on the GIF).
  python3 -I gif2png.py <in.gif> <out.png> [frame index, default last] [scale, default 0.5]"""
import sys
from PIL import Image

src, out = sys.argv[1:3]
im = Image.open(src)
idx = int(sys.argv[3]) if len(sys.argv) > 3 else im.n_frames - 1
scale = float(sys.argv[4]) if len(sys.argv) > 4 else 0.5
im.seek(min(idx, im.n_frames - 1))
f = im.convert('RGB')
f = f.resize((int(f.width * scale), int(f.height * scale)))
f.save(out)
print(out, 'frames', im.n_frames, 'size', im.size)
