"""Where the Fighters & Stages cursor is, from a 1280x674 frame of the open page: the highlighted group (left column bars at
y 143/178/213/248) and the highlighted row in the right column (rows every 33.8 px from y 143). Returns the focusable
index (groups' rows in order: 5, 5, 5, 6) or None."""
import sys
from PIL import Image

GROUP_Y = [143, 178, 213, 248]
GROUP_ROWS = [5, 5, 5, 6]
ROW_Y = [143 + round(33.8 * i) for i in range(12)]


def lum(px):
    return sum(px[:3]) / 3


def cursor(path):
    im = Image.open(path).convert("RGB")
    left = [lum(im.getpixel((300, y))) for y in GROUP_Y]
    right = [lum(im.getpixel((1000, y))) for y in ROW_Y]
    g = max(range(4), key=lambda i: left[i])
    r = max(range(len(ROW_Y)), key=lambda i: right[i])
    bg_l = sorted(left)[0]
    bg_r = sorted(right)[0]
    if left[g] - bg_l < 12 or right[r] - bg_r < 12 or r >= GROUP_ROWS[g]:
        return None, left, right
    return sum(GROUP_ROWS[:g]) + r, left, right


if __name__ == "__main__":
    print(cursor(sys.argv[1]))
