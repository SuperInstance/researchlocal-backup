"""One-shot image inspection utility for the tzpro-agent toolbar icon."""
from PIL import Image
from collections import Counter

SRC = r"C:\Users\casey\tzpro-agent-toolbar.jpg"
im = Image.open(SRC).convert("RGB")
w, h = im.size
gs = im.convert("L")
print(f"Source: {SRC}")
print(f"Size:   {w}x{h}\n")

# Brightness distribution
hist = gs.histogram()
total = sum(hist)
bands = [
    ("pure black 0-15", 0, 16),
    ("very dark 16-64", 16, 64),
    ("dark 64-128", 64, 128),
    ("mid 128-192", 128, 192),
    ("bright 192-255", 192, 256),
]
print("=== Brightness distribution ===")
for name, lo, hi in bands:
    pct = sum(hist[lo:hi]) * 100 / total
    print(f"  {name:24} {pct:5.1f}%")

# Color regions
print("\n=== Color regions by quadrant ===")
for name, box in [
    ("top-left",     (0, 0, w//2, h//2)),
    ("top-right",    (w//2, 0, w, h//2)),
    ("bottom-left",  (0, h//2, w//2, h)),
    ("bottom-right", (w//2, h//2, w, h)),
    ("center band",  (w//6, h//3, 5*w//6, 2*h//3)),
]:
    crop = im.crop(box).resize((20, 20))
    pixels = list(crop.getdata())
    rs = [p[0] for p in pixels]
    gs_ = [p[1] for p in pixels]
    bs = [p[2] for p in pixels]
    avg_r = sum(rs) // len(rs)
    avg_g = sum(gs_) // len(gs_)
    avg_b = sum(bs) // len(bs)
    saturated = sum(1 for p in pixels if max(p) - min(p) > 30)
    print(f"  {name:14} avg_rgb=({avg_r:3},{avg_g:3},{avg_b:3})  saturated_pixels={saturated}/{len(pixels)}")

# Distinctive saturated colors
print("\n=== Distinctive saturated colors (sat > 60, count >= 100) ===")
sample = im.resize((200, 170))
ctr = Counter(sample.getdata())
shown = 0
for color, n in ctr.most_common(200):
    r, g, b = color
    sat = max(r, g, b) - min(r, g, b)
    if sat > 60 and n > 100:
        hexcode = f"#{r:02x}{g:02x}{b:02x}"
        print(f"  rgb{color}  hex={hexcode}  count={n}")
        shown += 1
        if shown >= 20:
            break
