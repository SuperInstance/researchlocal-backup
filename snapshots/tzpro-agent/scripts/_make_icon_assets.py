"""Generate the icon asset bundle from the source toolbar image.

Source: C:\\Users\\casey\\tzpro-agent-toolbar.jpg
Outputs (in C:\\Users\\casey\\tzpro-agent\\assets\\):
  icon-source.png         256x256 PNG (dashboard header / preview)
  icon-tray-16.png        16x16   PNG (legacy tray fallback)
  icon-tray-32.png        32x32   PNG (high-DPI tray)
  icon-tray-64.png        64x64   PNG (very high-DPI tray)
  icon-tray.ico           Multi-size .ico (16/32/48/64 in one file)
  icon-shortcut-256.png   256x256 PNG (Windows shortcut icon)

We crop to a square before resizing so the tray icon is centered
on whatever the focal subject of the source JPG is.
"""
from __future__ import annotations

from pathlib import Path

from PIL import Image

SRC = Path(r"C:\Users\casey\tzpro-agent-toolbar.jpg")
OUT = Path(r"C:\Users\casey\tzpro-agent\assets")
OUT.mkdir(parents=True, exist_ok=True)

TRAY_SIZES = [16, 32, 48, 64]


def center_crop_square(img: Image.Image) -> Image.Image:
    """Crop to a centered square covering the largest possible area."""
    w, h = img.size
    side = min(w, h)
    left = (w - side) // 2
    top = (h - side) // 2
    return img.crop((left, top, left + side, top + side))


def main() -> None:
    if not SRC.exists():
        raise SystemExit(f"source image missing: {SRC}")

    print(f"loading {SRC} ...")
    src = Image.open(SRC).convert("RGBA")
    print(f"  size={src.size} mode={src.mode}")

    square = center_crop_square(src)
    print(f"  cropped to square: {square.size}")

    # 256x256 master for everything that isn't a tray icon
    master = square.resize((256, 256), Image.LANCZOS)
    master.save(OUT / "icon-source.png", "PNG", optimize=True)
    print(f"  saved icon-source.png  (256x256)")

    # Per-size PNG renders for tray (some Windows contexts prefer PNG over ICO)
    for size in (16, 32, 64):
        out = square.resize((size, size), Image.LANCZOS)
        out.save(OUT / f"icon-tray-{size}.png", "PNG", optimize=True)
        print(f"  saved icon-tray-{size}.png  ({size}x{size})")

    # Multi-resolution ICO - this is what pystray + Windows shortcut consume
    ico = square.resize((256, 256), Image.LANCZOS)
    ico.save(
        OUT / "icon-tray.ico",
        format="ICO",
        sizes=[(s, s) for s in TRAY_SIZES],
    )
    print(f"  saved icon-tray.ico  (sizes={TRAY_SIZES})")

    # Windows shortcut icon (256 is overkill but Windows picks what it needs)
    master.save(OUT / "icon-shortcut-256.png", "PNG", optimize=True)
    print(f"  saved icon-shortcut-256.png  (256x256)")

    print(f"\nall assets written to {OUT}")
    for p in sorted(OUT.glob("icon-*")):
        size = p.stat().st_size
        print(f"  {p.name:24}  {size:>7,} bytes")


if __name__ == "__main__":
    main()
