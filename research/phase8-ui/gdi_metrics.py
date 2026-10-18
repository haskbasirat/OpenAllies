"""Bake the GDI glyph cells of the system faces the original UI fonts use
(Arial, Arial Bold) into crates/aa-ui/data/gdi-metrics.json.

The original builds its font atlas with these calls for non-TTF faces
(KKC_RFont::AddGlyphs 0x7072F0 / SetupFont 0x708B30, research by the phase-8
text subagent, scratch-text2/gdip_metrics.py):
  hdc = CreateCompatibleDC(NULL)
  CreateFontW(-px, 0, 0, 0, bold ? 700 : 400, italic, 0, 0, DEFAULT_CHARSET 1,
              OUT_TT_ONLY_PRECIS 7, 0, antialias ? 4 : 3, VARIABLE_PITCH 2, face)
  per covered character: GetTextExtentPoint32W(hdc, &ch, 1, &sz)
  cell = (sz.cx, sz.cy) (+2 outline, else +1 shadow, added by the caller)
Coverage is the face's cmap (the original reads it with GetFontData,
0x704DF0/0x7050A0): uncovered characters get no glyph entry.

These are GDI's HINTED metrics of the Arial installed on THIS machine (the
original ran on whatever Arial the player's Windows had). Output per
(face, bold, px): cy, the widest cx over every covered code point (for the
fixed-width advance font+0xC), and cx for the covered code points 0..0x24F.
The process is DPI-unaware like AA.exe; quality ANTIALIASED (antialias is
the template default).

usage: python research/phase8-ui/gdi_metrics.py
"""
import ctypes, json, struct
from ctypes import wintypes as W
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE.parents[1] / 'crates' / 'aa-ui' / 'data' / 'gdi-metrics.json'
FONTS = Path('C:/Windows/Fonts')
PX = range(4, 65)
TABLE_MAX = 0x24F

user32 = ctypes.WinDLL('user32')
gdi32 = ctypes.WinDLL('gdi32')
try:
    user32.SetThreadDpiAwarenessContext.restype = ctypes.c_void_p
    user32.SetThreadDpiAwarenessContext(ctypes.c_void_p(-1))
except AttributeError:
    pass
gdi32.CreateCompatibleDC.restype = ctypes.c_void_p
gdi32.CreateFontW.restype = ctypes.c_void_p
gdi32.SelectObject.restype = ctypes.c_void_p
gdi32.SelectObject.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
gdi32.DeleteObject.argtypes = [ctypes.c_void_p]
gdi32.GetTextExtentPoint32W.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p, ctypes.c_int, ctypes.POINTER(W.SIZE)]
hdc = gdi32.CreateCompatibleDC(None)


def cmap_codepoints(path):
    b = path.read_bytes()
    n = struct.unpack_from('>H', b, 4)[0]
    tabs = {}
    for i in range(n):
        tag, _, off, ln = struct.unpack_from('>4sIII', b, 12 + 16 * i)
        tabs[tag] = off
    o = tabs[b'cmap']
    sub = None
    for i in range(struct.unpack_from('>H', b, o + 2)[0]):
        pid, eid, off = struct.unpack_from('>HHI', b, o + 4 + 8 * i)
        if pid == 3 and eid in (1, 0):
            sub = o + off
    seg = struct.unpack_from('>H', b, sub + 6)[0] // 2
    ends = struct.unpack_from('>%dH' % seg, b, sub + 14)
    starts = struct.unpack_from('>%dH' % seg, b, sub + 16 + 2 * seg)
    deltas = struct.unpack_from('>%dh' % seg, b, sub + 16 + 4 * seg)
    rp = sub + 16 + 6 * seg
    roffs = struct.unpack_from('>%dH' % seg, b, rp)
    out = []
    for i in range(seg):
        for c in range(starts[i], ends[i] + 1):
            if c > 0xFFFA:
                continue
            if roffs[i] == 0:
                g = (c + deltas[i]) & 0xFFFF
            else:
                p = rp + 2 * i + roffs[i] + 2 * (c - starts[i])
                g = struct.unpack_from('>H', b, p)[0]
                g = (g + deltas[i]) & 0xFFFF if g else 0
            if g:
                out.append(c)
    return out


def main():
    data = {'note': __doc__, 'faces': []}
    for face, bold, file in (('Arial', False, 'arial.ttf'), ('Arial', True, 'arialbd.ttf')):
        cps = cmap_codepoints(FONTS / file)
        for px in PX:
            hf = gdi32.CreateFontW(-px, 0, 0, 0, 700 if bold else 400, 0, 0, 0, 1, 7, 0, 4, 2, face)
            old = gdi32.SelectObject(hdc, hf)
            cx = {}
            cy = 0
            widest = 0
            for c in cps:
                sz = W.SIZE()
                gdi32.GetTextExtentPoint32W(hdc, chr(c), 1, ctypes.byref(sz))
                widest = max(widest, sz.cx)
                cy = sz.cy
                if c <= TABLE_MAX:
                    cx[c] = sz.cx
            gdi32.SelectObject(hdc, old)
            gdi32.DeleteObject(hf)
            data['faces'].append({'face': face, 'bold': bold, 'px': px, 'cy': cy, 'widest': widest,
                                  'cx': [[c, w] for c, w in sorted(cx.items())]})
        print(face, bold, len(cps), 'code points')
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(data, separators=(',', ':')) + '\n')
    print(OUT, OUT.stat().st_size)


if __name__ == '__main__':
    main()
