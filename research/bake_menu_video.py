"""Bake the original main-menu intro movies into JPEG frame sequences.

The original front end loops five Bink movies (USA, JPN, GER, RUS, GBR
parades) behind the menu column. The reconstruction already converted them
to MP4; the Rust client cannot decode MP4 without a system codec, so this
writes plain JPEG frames plus an index the client streams as a flipbook.

    python research/bake_menu_video.py [--src DIR] [--out DIR] [--quality 85]

Needs PyAV and Pillow. Output: assets-cache/video/MainMenu/<NAME>/NNNN.jpg
and assets-cache/video/MainMenu/index.json.
"""
import argparse
import json
import os
import sys

import av

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_SRC = os.path.normpath(os.path.join(HERE, "..", "..", "reconstruction", "assets", "converted", "video", "Video", "MainMenu"))
DEFAULT_OUT = os.path.normpath(os.path.join(HERE, "..", "assets-cache", "video", "MainMenu"))
# Order of [VideoResource] entries in Data/UI/Menus/menu_background.tgi.
MOVIES = ["USAIntroMovie", "JPNIntroMovie", "GERIntroMovie", "RUSIntroMovie", "GBRIntroMovie"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default=DEFAULT_SRC)
    ap.add_argument("--out", default=DEFAULT_OUT)
    ap.add_argument("--quality", type=int, default=85)
    args = ap.parse_args()
    index = {"movies": []}
    for name in MOVIES:
        src = os.path.join(args.src, name + ".mp4")
        if not os.path.isfile(src):
            print(f"missing {src}", file=sys.stderr)
            continue
        dst = os.path.join(args.out, name)
        os.makedirs(dst, exist_ok=True)
        with av.open(src) as c:
            stream = c.streams.video[0]
            fps = float(stream.average_rate or 24)
            n = 0
            for frame in c.decode(stream):
                frame.to_image().save(os.path.join(dst, f"{n:04d}.jpg"), quality=args.quality)
                n += 1
            w, h = stream.codec_context.width, stream.codec_context.height
        index["movies"].append({"name": name, "fps": fps, "frames": n, "width": w, "height": h})
        print(f"{name}: {n} frames {w}x{h} @ {fps:g} fps")
    with open(os.path.join(args.out, "index.json"), "w") as f:
        json.dump(index, f, indent=1)


if __name__ == "__main__":
    main()
