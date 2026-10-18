"""Normalize recorded native read traces into wire tokens (analysis helper)."""
import json
THING = {'0076a4b0': 'Thing', '0076a5a0': 'ThingB4', '0076a600': 'ThingA8', '0076a530': 'ThingB0',
         '0076a670': 'ThingAC', '0076a6e0': 'ThingBC', '0076a750': 'ThingC0'}
def tokens(reads, managers=None):
    out = []; i = 0
    while i < len(reads):
        e = reads[i]; fn = e['fn']; w = e['width']
        nxt = reads[i + 1] if i + 1 < len(reads) else None
        if w == 16 and nxt is not None and nxt['fn'] == fn and isinstance(e['value'], int) and nxt['width'] == 16 * e['value'] and fn not in THING:
            out.append(('Str', fn)); i += 2; continue
        if fn == '00411fa0':
            c = e['registry_count']
            name = managers.get(c, None) if managers else None
            out.append(('Def', name) if name else ('Bounded', c)); i += 1; continue
        if fn in THING and w == 16: out.append((THING[fn],)); i += 1; continue
        if fn == '00765f70' and w == 32: out.append(('Player',)); i += 1; continue
        if fn == '0076a810' and w == 32 and nxt is not None and nxt['fn'] == fn:
            out.append(('NetAddr',)); i += 2; continue
        out.append(('U', w, fn)); i += 1
    return out
def load():
    d = json.load(open('order-traces.json'))
    managers = {c + 1: g for g, c in d['managers'].items()}
    return d, managers
if __name__ == '__main__':
    import sys
    d, managers = load()
    for x in d['descriptors']:
        seqs = {}
        for r in x['runs']:
            t = tuple(tokens(r['reads'], managers)); seqs.setdefault(t, 0); seqs[t] += 1
        print(x['id'], x['name'], len(seqs))
        for t in sorted(seqs, key=len):
            print('   ', ' '.join(':'.join(str(p) for p in k) for k in t))
