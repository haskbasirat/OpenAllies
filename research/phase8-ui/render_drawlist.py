"""Composite an aa-ui draw list into a PNG for visual comparison.

usage: render_drawlist.py <drawlist.json> <out.png> [background.png]
Quads are drawn with point sampling in draw-list order (alpha blend for
'Alpha', alpha-test >0 otherwise); text commands are drawn glyph quad by
glyph quad (aa_ui::rdraw positions and clipping) with the A&A TrueType face
or the Windows Arial files via PIL, in draw-list order (glyph shapes are an
approximation; the original rasterises with GDI+ / GDI into an atlas);
video/external rects are outlined.
"""
import json, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFont
sys.path.insert(0, str(Path(__file__).resolve().parent))
from screenshot_match import converted_texture, rasterise

ROOT = Path(__file__).resolve().parents[2]
FONT_DIR = ROOT.parent / 'reconstruction' / 'assets' / 'original' / 'Data' / 'Fonts' / 'TrueType'


def main():
    dl = json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'))
    W, H = dl['screen']
    if len(sys.argv) > 3:
        canvas = np.asarray(Image.open(sys.argv[3]).convert('RGB').resize((W, H))).astype(np.float32) * 0.35
    else:
        canvas = np.zeros((H, W, 3), np.float32)
    outlines = []
    texts = []
    cache = {}
    for c in dl['commands']:
        if c['kind'] == 'quad':
            if not c.get('texture'):
                # vertex colour only (status bars, colour decals)
                r = c['rect']
                x0, y0 = int(round(r['l'])), int(round(r['t']))
                x1, y1 = int(round(r['r'])), int(round(r['b']))
                col = [x if x is not None else 0.0 for x in c['color']]
                if x1 > x0 and y1 > y0:
                    xa, ya, xb, yb = max(x0, 0), max(y0, 0), min(x1, W), min(y1, H)
                    if xb > xa and yb > ya:
                        a = col[3] if c.get('blend') == 'Alpha' else 1.0
                        canvas[ya:yb, xa:xb] = canvas[ya:yb, xa:xb] * (1 - a) + np.array(col[:3], np.float32) * 255 * a
                continue
            png = converted_texture(c['texture'], dl.get('skin'))
            if png is None:
                r = c['rect']
                outlines.append(((r['l'], r['t'], r['r'], r['b']), (255, 0, 255)))
                continue
            if png not in cache:
                t = np.asarray(Image.open(png).convert('RGBA')).astype(np.float32)
                t[..., 3] /= 255.0
                cache[png] = t
            r = c['rect']
            rr = rasterise(cache[png], [r['l'], r['t'], r['r'], r['b']], c['uv'], None)
            if rr is None:
                continue
            x0, y0, img = rr
            hh, ww = img.shape[:2]
            xa, ya = max(x0, 0), max(y0, 0)
            xb, yb = min(x0 + ww, W), min(y0 + hh, H)
            if xb <= xa or yb <= ya:
                continue
            sub = img[ya - y0:yb - y0, xa - x0:xb - x0]
            col = sub[..., :3] * np.array(c['color'][:3], np.float32)
            a = sub[..., 3:4] * c['color'][3]
            if c['blend'] != 'Alpha':
                a = (a > 0).astype(np.float32)
            dst = canvas[ya:yb, xa:xb]
            canvas[ya:yb, xa:xb] = dst * (1 - a) + col * a
        elif c['kind'] == 'text':
            draw_text(canvas, c)
        else:
            r = c['rect']
            outlines.append(((r['l'], r['t'], r['r'], r['b']), (0, 200, 255)))
    im = Image.fromarray(np.clip(canvas, 0, 255).astype(np.uint8))
    d = ImageDraw.Draw(im)
    for (l, t, r, b), col in outlines:
        d.rectangle([l, t, r - 1, b - 1], outline=col, width=2)
    im.save(sys.argv[2])


FONTS = {}
WINFONTS = Path('C:/Windows/Fonts')


def pil_font(face):
    em = int(round(face.get('em_px', 16)))
    if face.get('gdiplus', True):
        path = FONT_DIR / 'A&A.ttf'
    else:
        stem = face.get('face', 'arial').lower().replace(' ', '')
        path = WINFONTS / (stem + ('bd' if face.get('bold') else '') + '.ttf')
    key = (str(path), em)
    if key not in FONTS:
        try:
            FONTS[key] = ImageFont.truetype(str(path), em)
        except OSError:
            FONTS[key] = ImageFont.load_default()
    return FONTS[key]


def blend(canvas, x, y, rgba):
    H, W = canvas.shape[:2]
    h, w = rgba.shape[:2]
    xa, ya, xb, yb = max(x, 0), max(y, 0), min(x + w, W), min(y + h, H)
    if xb <= xa or yb <= ya:
        return
    sub = rgba[ya - y:yb - y, xa - x:xb - x]
    al = sub[..., 3:4]
    canvas[ya:yb, xa:xb] = canvas[ya:yb, xa:xb] * (1 - al) + sub[..., :3] * al


def draw_text(canvas, c):
    """Glyph quads of the native rebuild (aa_ui::rdraw): the cell top-left
    is rect - cell; a quad shows only its part of the cell (the shadow is
    drawn at +1,+1 into the cell, as the atlas bakes it)."""
    faces = c.get('faces') or []
    for g in c['glyphs']:
        r = g['rect']
        x0, y0 = int(r['l']), int(r['t'])
        w, h = int(r['r'] - r['l']), int(r['b'] - r['t'])
        if w <= 0 or h <= 0:
            continue
        col = g['color']
        if g['highlight']:
            blend(canvas, x0, y0, np.concatenate([np.full((h, w, 3), 255 * np.array(col[:3]), np.float32),
                                                 np.full((h, w, 1), col[3], np.float32)], axis=2))
            continue
        cx, cy = int(g['cell'][0]), int(g['cell'][1])
        if g.get('image'):
            png = converted_texture(g['image'], None)
            if png is None:
                continue
            img = np.asarray(Image.open(png).convert('RGBA')).astype(np.float32)
            sub = img[cy:cy + h, cx:cx + w].copy()
            sub[..., :3] *= np.array(col[:3], np.float32)
            sub[..., 3] = sub[..., 3] / 255.0 * col[3]
            blend(canvas, x0, y0, sub)
            continue
        ch = chr(g['c'])
        if ch.isspace():
            continue
        face = (faces[g['face']].get('face') if g['face'] < len(faces) else None) or {}
        f = pil_font(face)
        cw, chh = cx + w + 2, cy + h + 2
        layer = Image.new('RGBA', (cw, chh), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        if face.get('shadow'):
            d.text((1, 1), ch, font=f, fill=(0, 0, 0, int(255 * face.get('shadow_opacity', 1.0))))
        d.text((0, 0), ch, font=f, fill=(255, 255, 255, 255))
        a = np.asarray(layer).astype(np.float32)[cy:cy + h, cx:cx + w]
        a[..., :3] *= np.array(col[:3], np.float32)
        a[..., 3] = a[..., 3] / 255.0 * col[3]
        blend(canvas, x0, y0, a)


if __name__ == '__main__':
    main()
