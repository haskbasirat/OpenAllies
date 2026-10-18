"""Compare aa-ui draw lists with screenshots of the running original.

Input: draw-list JSON files written by the aa-ui example `dump_screens`
(research/phase8-ui/drawlists/<screen>.json) and the reference screenshots
qa-output/original-ui/*.png (2560x1440, captured from AA.exe).

For every textured quad whose source texture is opaque somewhere, the quad is
rasterised (bilinear, the texture stretched over the quad's rect and UV
range) and the integer pixel offset (dx, dy) in [-R, R]^2 that minimises the
mean absolute RGB error over the quad's fully opaque, unoccluded texels is
searched. A quad that lands exactly where the original drew it has its best
offset at (0, 0). Pixels covered by later opaque quads are excluded, as are
quads with fewer than MIN_PIXELS usable pixels.

Output: research/phase8-ui/screenshot-match.json (per quad: rect, best
offset, error at the best offset and at (0,0)).
"""
from pathlib import Path
import json, sys, hashlib
import numpy as np
from PIL import Image

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
SHOTS = ROOT / 'qa-output' / 'original-ui'
CONVERTED = ROOT.parent / 'reconstruction' / 'assets' / 'converted' / 'textures'
SKINS = ROOT.parent / 'reconstruction' / 'assets' / 'converted' / 'skins'
ORIGINAL_DATA = ROOT.parent / 'reconstruction' / 'assets' / 'original' / 'Data'
ORIGINAL_SKINS = ROOT.parent / 'reconstruction' / 'assets' / 'original' / 'Skins'
R = 6
MIN_PIXELS = 200


def converted_texture(data_path, skin=None):
    """Map an original Data path to the converted PNG (same folders,
    case-insensitive, .png extension); a skin's converted files first."""
    rel = data_path.strip('/').replace('\\', '/')
    stem = rel.rsplit('.', 1)[0].lower()
    index = converted_index()
    # a PNG original is used as is (the converted index is keyed by stem
    # and would return a same-named TGA's conversion)
    if rel.lower().endswith('.png'):
        for base in ([ORIGINAL_SKINS / skin] if skin else []) + [ORIGINAL_DATA]:
            p = find_ci(base, rel)
            if p:
                return p
    if skin:
        hit = index.get(('skins/' + skin + '/' + stem).lower())
        if hit:
            return hit
    hit = index.get(stem)
    if hit:
        return hit
    # PNG originals were not converted: use the original file
    if rel.lower().endswith('.png'):
        for base in ([ORIGINAL_SKINS / skin] if skin else []) + [ORIGINAL_DATA]:
            p = find_ci(base, rel)
            if p:
                return p
    return None


def find_ci(base, rel):
    at = base
    for part in rel.split('/'):
        if not at.is_dir():
            return None
        names = {c.name.lower(): c for c in at.iterdir()}
        at = names.get(part.lower())
        if at is None:
            return None
    return at if at.is_file() else None


_INDEX = None


def converted_index():
    global _INDEX
    if _INDEX is None:
        _INDEX = {}
        for p in CONVERTED.rglob('*.png'):
            rel = p.relative_to(CONVERTED).as_posix()
            _INDEX[rel.rsplit('.', 1)[0].lower()] = p
        for p in SKINS.rglob('*.png'):
            rel = 'skins/' + p.relative_to(SKINS).as_posix()
            _INDEX[rel.rsplit('.', 1)[0].lower()] = p
    return _INDEX


def rasterise(tex, rect, uv, clip):
    """Return (x0, y0, rgba) of the texture drawn into rect, sampled at pixel
    centres (i + 0.5) with POINT filtering and clamp addressing, as the
    original's samplers (0x742200). Draw-list rects already use the i + 0.5
    convention (sprite.rs `raster`)."""
    l, t, r, b = rect
    x0, y0 = int(np.floor(l)), int(np.floor(t))
    x1, y1 = int(np.ceil(r)), int(np.ceil(b))
    if x1 <= x0 or y1 <= y0:
        return None
    h, w = tex.shape[:2]
    xs = np.arange(x0, x1) + 0.5
    ys = np.arange(y0, y1) + 0.5
    u0, v0, u1, v1 = uv
    fu = (xs - l) / (r - l)
    fv = (ys - t) / (b - t)
    u = (u0 + (u1 - u0) * fu) * w
    v = (v0 + (v1 - v0) * fv) * h
    inside_x = (xs >= l) & (xs < r)
    inside_y = (ys >= t) & (ys < b)
    if clip is not None:
        cl, ct, cr, cb = clip
        inside_x &= (xs >= cl) & (xs < cr)
        inside_y &= (ys >= ct) & (ys < cb)
    ui = np.clip(np.floor(u).astype(int), 0, w - 1)
    vi = np.clip(np.floor(v).astype(int), 0, h - 1)
    out = tex[vi][:, ui].copy()
    mask = inside_y[:, None] & inside_x[None, :]
    out[..., 3] = np.where(mask, out[..., 3], 0)
    return x0, y0, out


def match_screen(drawlist_path, shot_path):
    dl = json.loads(Path(drawlist_path).read_text(encoding='utf-8'))
    shot = np.asarray(Image.open(shot_path).convert('RGB')).astype(np.float32)
    H, W = shot.shape[:2]
    quads = [c for c in dl['commands'] if c['kind'] == 'quad' and c.get('texture')]
    # occlusion: later opaque quads hide earlier ones
    rendered = []
    for q in quads:
        png = converted_texture(q['texture'], dl.get('skin'))
        if png is None:
            rendered.append(None)
            continue
        tex = np.asarray(Image.open(png).convert('RGBA')).astype(np.float32)
        tex[..., 3] /= 255.0
        rect = q['rect']
        rr = rasterise(tex, [rect['l'], rect['t'], rect['r'], rect['b']], q['uv'],
                       None if q.get('clip') is None else [q['clip'][k] for k in 'ltrb'])
        rendered.append((png, rr))
    cover = np.zeros((H, W), np.int32) - 1
    for i, item in enumerate(rendered):
        if item is None or item[1] is None:
            continue
        x0, y0, img = item[1]
        hh, ww = img.shape[:2]
        sub = img[..., 3] >= 0.999
        ys, xs = np.nonzero(sub)
        ys = ys + y0
        xs = xs + x0
        ok = (ys >= 0) & (ys < H) & (xs >= 0) & (xs < W)
        cover[ys[ok], xs[ok]] = i
    rows = []
    for i, (q, item) in enumerate(zip(quads, rendered)):
        row = {'widget': q.get('widget'), 've': q.get('ve'), 'texture': q['texture'],
               'rect': [q['rect'][k] for k in 'ltrb']}
        if item is None or item[1] is None:
            row['status'] = 'no converted texture'
            rows.append(row)
            continue
        x0, y0, img = item[1]
        hh, ww = img.shape[:2]
        opaque = img[..., 3] >= 0.999
        # exclude pixels covered by later opaque quads and the screen border
        ys, xs = np.nonzero(opaque)
        gy, gx = ys + y0, xs + x0
        ok = (gy >= R) & (gy < H - R) & (gx >= R) & (gx < W - R)
        ys, xs, gy, gx = ys[ok], xs[ok], gy[ok], gx[ok]
        own = cover[gy, gx] == i
        ys, xs, gy, gx = ys[own], xs[own], gy[own], gx[own]
        if len(ys) < MIN_PIXELS:
            row['status'] = 'too few unoccluded opaque pixels'
            row['pixels'] = int(len(ys))
            rows.append(row)
            continue
        colour = img[ys, xs, :3] * np.array(q.get('color', [1, 1, 1, 1])[:3], np.float32)
        best = None
        errors = {}
        for dy in range(-R, R + 1):
            for dx in range(-R, R + 1):
                e = float(np.abs(shot[gy + dy, gx + dx] - colour).mean())
                errors[(dx, dy)] = e
                if best is None or e < best[0]:
                    best = (e, dx, dy)
        row.update({'status': 'matched', 'pixels': int(len(ys)), 'best_offset': [best[1], best[2]],
                    'best_error': round(best[0], 3), 'error_at_zero': round(errors[(0, 0)], 3)})
        rows.append(row)
    texts = []
    for c in dl['commands']:
        glyphs = [g for g in c.get('glyphs', []) if not g['highlight']] if c['kind'] == 'text' else []
        if not glyphs:
            continue
        r = c['rect']
        l, t, rr, b = [int(round(r[k])) for k in 'ltrb']
        l, t = max(l, 0), max(t, 0)
        sub = shot[t:b, l:rr]
        if sub.size == 0:
            continue
        # white label text: all channels above 225
        ys, xs = np.nonzero(sub.min(axis=2) > 225)
        # first row of glyph quads: cell top-left of the first glyph = pen
        g0 = glyphs[0]
        px, py = g0['rect']['l'] - g0['cell'][0], g0['rect']['t'] - g0['cell'][1]
        row = [g for g in glyphs if g['rect']['t'] - g['cell'][1] == py and chr(g['c']) != ' ']
        entry = {'widget': c['widget'], 'text': c['text'], 'row_x': px, 'row_y': py,
                 'row_width': (row[-1]['rect']['r'] - px) if row else 0,
                 'line_height': g0['rect']['b'] - g0['rect']['t'] + g0['cell'][1]}
        if len(xs):
            entry['ink'] = [int(xs.min() + l), int(ys.min() + t), int(xs.max() + l + 1), int(ys.max() + t + 1)]
        texts.append(entry)
    return {'drawlist': Path(drawlist_path).name, 'screenshot': Path(shot_path).name, 'texts': texts,
            'screenshot_sha256': hashlib.sha256(Path(shot_path).read_bytes()).hexdigest(),
            'drawlist_sha256': hashlib.sha256(Path(drawlist_path).read_bytes()).hexdigest(),
            'quads': rows}


def main():
    pairs = json.loads((HERE / 'drawlists' / 'index.json').read_text(encoding='utf-8'))
    out = {'schema': 1, 'scope': __doc__, 'search_radius_px': R, 'screens': []}
    for p in pairs:
        res = match_screen(HERE / 'drawlists' / p['drawlist'], SHOTS / p['screenshot'])
        res['name'] = p['name']
        out['screens'].append(res)
        matched = [q for q in res['quads'] if q['status'] == 'matched']
        exact = [q for q in matched if q['best_offset'] == [0, 0]]
        print(p['name'], 'quads', len(res['quads']), 'matched', len(matched), 'offset(0,0)', len(exact))
        for q in matched:
            if q['best_offset'] != [0, 0]:
                print('   ', q['widget'], q['ve'], q['best_offset'], q['best_error'], q['error_at_zero'])
    (HERE / 'screenshot-match.json').write_text(json.dumps(out, indent=1) + '\n')


if __name__ == '__main__':
    main()
