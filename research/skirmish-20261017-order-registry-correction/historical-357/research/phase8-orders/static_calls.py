"""Static call sequence of each original order handler (capstone, read-only)."""
from pathlib import Path
import json, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from natdis import insns, wstr
from native_order_traces import DESCRIPTORS, extent
LOG = {0x4012c0, 0x40f8d0, 0x77da70, 0x96cb83, 0x401560, 0x40f950, 0x40c970, 0x404f90, 0x404f30, 0x77da70}
def calls(handler):
    lo, hi = extent(handler)
    out = []
    for i in insns(lo, size=hi - lo):
        if i.mnemonic == 'call':
            try: tgt = int(i.op_str, 16)
            except ValueError: tgt = i.op_str
            out.append((i.address, tgt))
    return out
def analyse(handler):
    cs = calls(handler); tg = [t for _, t in cs]
    info = {'calls': [f'{t:#x}' if isinstance(t, int) else t for t in tg]}
    request = 0x766c30 in tg and 0x766c40 in tg
    info['template_request'] = request
    first_log = next((k for k, t in enumerate(tg) if t == 0x40f8d0), None)
    if request:
        a = tg.index(0x766c70) if 0x766c70 in tg else tg.index(0x766c40)
        b = tg.index(0x766ca0)
        log = tg.index(0x404f90)
        getter = tg[log - 1] if log > 0 else None
        info['name_getter'] = f'{getter:#x}' if isinstance(getter, int) else getter
        info['validity'] = [f'{t:#x}' for t in tg[a + 1:b] if isinstance(t, int) and t not in (getter, 0x404f30)]
        c = tg.index(0x766d00, b)
        w = tg[c + 1]
        info['writer'] = 'none (empty payload)' if w == 0x4012c0 else ('inline' if w in (0x4b2830, 0x40d420) else (f'{w:#x}' if isinstance(w, int) else w))
        first_log = next(k for k in range(c, len(tg)) if tg[k] == 0x40f8d0)
        info['decode'] = [f'{t:#x}' for t in tg[:log - 1] if isinstance(t, int)]
    else:
        info['decode'] = [f'{t:#x}' for t in tg[:first_log] if isinstance(t, int) and t not in LOG] if first_log is not None else None
    if first_log is not None:
        rest = [t for t in tg[first_log:] if t not in LOG]
        info['process'] = f'{rest[0]:#x}' if rest and isinstance(rest[0], int) else (rest[0] if rest else None)
    return info
if __name__ == '__main__':
    rows = []
    for ident, name, handler in DESCRIPTORS:
        try: a = analyse(handler)
        except Exception as e: a = {'error': repr(e)}
        rows.append({'id': ident, 'name': name, 'handler': f'{handler:#x}', **a})
    (HERE / 'static-handler-calls.json').write_text(json.dumps(rows, indent=1) + '\n')
    for r in rows: print(r['id'], r['name'], r.get('template_request'), 'valid', r.get('validity'), 'writer', r.get('writer'), 'process', r.get('process'), r.get('error', ''))
