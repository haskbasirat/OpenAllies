"""Bake the original intro movies (AVars_version.tgi IntroVideo1..4:
/Video/Intro/ESRB, TimeGate, Atari, ATI) into JPEG frame sequences and
16-bit PCM WAV sound tracks for the Bevy client.

The original plays them one after another at startup (0x5C4910 collects
IntroVideo1..N, 0x5C4AF0 plays each in the FullscreenVideoWidget of
UI/Shared/video.tgi, then shows the main menu). The reconstruction already
converted the Bink files to MP4; this writes plain frames plus an index the
client streams as a flipbook (same scheme as bake_menu_video.py).

    python research/bake_intro_video.py [--src DIR] [--out DIR] [--quality 85]

Needs PyAV. Output: assets-cache/video/Intro/<NAME>/NNNN.jpg,
assets-cache/video/Intro/<NAME>.wav (when the movie has sound) and
assets-cache/video/Intro/index.json.
"""
import argparse
import json
import os
import sys
import wave

import av

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_SRC = os.path.normpath(os.path.join(HERE, "..", "..", "reconstruction", "assets", "converted", "video", "Video", "Intro"))
DEFAULT_OUT = os.path.normpath(os.path.join(HERE, "..", "assets-cache", "video", "Intro"))
# AVars_version.tgi IntroVideo1..4.
MOVIES = ["ESRB", "TimeGate", "Atari", "ATI"]


def bake_audio(src, dst):
    """The movie's first audio stream as 16-bit PCM at its own rate."""
    with av.open(src) as c:
        streams = [s for s in c.streams if s.type == "audio"]
        if not streams:
            return None
        stream = streams[0]
        rate = stream.codec_context.sample_rate
        channels = stream.codec_context.channels
        layout = "stereo" if channels >= 2 else "mono"
        resampler = av.AudioResampler(format="s16", layout=layout, rate=rate)
        pcm = bytearray()
        for frame in c.decode(stream):
            for out in resampler.resample(frame):
                pcm += bytes(out.planes[0])[: out.samples * (2 if layout == "stereo" else 1) * 2]
        for out in resampler.resample(None):
            pcm += bytes(out.planes[0])[: out.samples * (2 if layout == "stereo" else 1) * 2]
    with wave.open(dst, "wb") as w:
        w.setnchannels(2 if layout == "stereo" else 1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(bytes(pcm))
    return {"rate": rate, "channels": 2 if layout == "stereo" else 1}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", default=DEFAULT_SRC)
    ap.add_argument("--out", default=DEFAULT_OUT)
    ap.add_argument("--quality", type=int, default=85)
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)
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
            fps = float(stream.average_rate or 30)
            n = 0
            for frame in c.decode(stream):
                frame.to_image().save(os.path.join(dst, f"{n:04d}.jpg"), quality=args.quality)
                n += 1
            w, h = stream.codec_context.width, stream.codec_context.height
        audio = bake_audio(src, os.path.join(args.out, name + ".wav"))
        entry = {"name": name, "fps": fps, "frames": n, "width": w, "height": h}
        if audio:
            entry["audio"] = name + ".wav"
        index["movies"].append(entry)
        print(f"{name}: {n} frames {w}x{h} @ {fps:g} fps, audio {audio}")
    with open(os.path.join(args.out, "index.json"), "w") as f:
        json.dump(index, f, indent=1)


if __name__ == "__main__":
    main()
