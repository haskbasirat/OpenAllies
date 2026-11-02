"""Bake the scripted campaigns' movies (mission_type = CINEMATIC of
Data/Campaign/campaign_aa.tgi, and the trigger PlayVideo of the same folder) into
the JPEG flipbook the client's VideoPlayer reads (crates/aa-bevy/src/ui/video.rs).

    python research/phase8-ui/bake_campaign_video.py

Source: the reconstruction's MP4 conversions of the original Bink files.
Output: assets-cache/video/Campaign/<Name>/NNNN.jpg, <Name>.wav, index.json.
Needs PyAV and Pillow.
"""
import json
import os
import wave

import av

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.normpath(os.path.join(HERE, '..', '..', '..', 'reconstruction', 'assets', 'converted', 'video', 'Video', 'Campaign'))
OUT = os.path.normpath(os.path.join(HERE, '..', '..', 'assets-cache', 'video', 'Campaign'))
QUALITY = 85


def bake(path, name):
    dst = os.path.join(OUT, name)
    os.makedirs(dst, exist_ok=True)
    entry = {}
    with av.open(path) as c:
        stream = c.streams.video[0]
        stream.thread_type = 'AUTO'
        fps = float(stream.average_rate or 30)
        n = 0
        for frame in c.decode(stream):
            frame.to_image().save(os.path.join(dst, f'{n:04d}.jpg'), quality=QUALITY)
            n += 1
        entry = {'name': name, 'fps': fps, 'frames': n, 'width': stream.width, 'height': stream.height}
    with av.open(path) as c:
        if c.streams.audio:
            audio = c.streams.audio[0]
            rate = audio.rate or 44100
            resampler = av.AudioResampler(format='s16', layout='stereo', rate=rate)
            wav = os.path.join(OUT, f'{name}.wav')
            with wave.open(wav, 'wb') as w:
                w.setnchannels(2)
                w.setsampwidth(2)
                w.setframerate(rate)
                for frame in c.decode(audio):
                    for out in resampler.resample(frame):
                        w.writeframes(bytes(out.planes[0])[: out.samples * 4])
            entry['audio'] = f'{name}.wav'
    return entry


def main():
    os.makedirs(OUT, exist_ok=True)
    movies = []
    for f in sorted(os.listdir(SRC)):
        if f.lower().endswith('.mp4'):
            name = os.path.splitext(f)[0]
            print(name, flush=True)
            movies.append(bake(os.path.join(SRC, f), name))
    with open(os.path.join(OUT, 'index.json'), 'w') as o:
        json.dump({'movies': movies}, o, indent=1)
    print(sum(m['frames'] for m in movies), 'frames')


if __name__ == '__main__':
    main()
