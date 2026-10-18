"""Bake the main-menu globe cinematic (MiniMenuBackground,
Data/UI/Menus/menu_background.tgi: /Video/MainMenu/AAGlobeBig.bik) into a
JPEG flipbook in the same format the client's MenuVideo player reads.

    python research/phase8-ui/bake_globe.py

Source: the reconstruction's MP4 conversion of the original Bink file.
Output: assets-cache/video/MainMenuGlobe/AAGlobeBig/NNNN.jpg + index.json.
Needs PyAV and Pillow.
"""
import json
import os

import av

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.normpath(os.path.join(HERE, '..', '..', '..', 'reconstruction', 'assets', 'converted', 'video',
                                    'Video', 'MainMenu', 'AAGlobeBig.mp4'))
OUT = os.path.normpath(os.path.join(HERE, '..', '..', 'assets-cache', 'video', 'MainMenuGlobe'))


def main():
    dst = os.path.join(OUT, 'AAGlobeBig')
    os.makedirs(dst, exist_ok=True)
    with av.open(SRC) as c:
        stream = c.streams.video[0]
        fps = float(stream.average_rate or 30)
        n = 0
        for frame in c.decode(stream):
            frame.to_image().save(os.path.join(dst, f'{n:04d}.jpg'), quality=92)
            n += 1
        w, h = stream.codec_context.width, stream.codec_context.height
    with open(os.path.join(OUT, 'index.json'), 'w') as f:
        json.dump({'movies': [{'name': 'AAGlobeBig', 'fps': fps, 'frames': n, 'width': w, 'height': h}]}, f, indent=1)
    print(f'AAGlobeBig: {n} frames {w}x{h} @ {fps:g} fps -> {OUT}')


if __name__ == '__main__':
    main()
