"""Actual 461F70 death-Animation damage_filter lookup (4620A6..4620EC) in the
DamageType DataManager [B36530]+3A4, whose registry is built by ORIGINAL code:
5BD080 parses Templates/template_damage_type.tgi and then the DamageType file,
481410 constructs the manager (capacity 0xB, mode 0, as pushed at 48AA71),
then per root 483A40 -> 481570 -> 481630 (register at +24/+28, index at +C)
-> 4C0870 -> vtable A228D0[0] = 4C0A80 LoadData with the REAL typed getters
(5B54D0/5B4C30/5B4ED0/5B5620). 4C0A80 stores DamageType+2C = damage_filter.

Explicit seams (recorded): 48D500 on the Glyph manager [B36530]+3BC returns a
fixture handle for a glyph in GLYPHS and 0 otherwise; 4B9F40 (text table
GetDisplayString on [B36544]) returns the key itself. Neither value is read
by 461F70. A 4C0A80 throw stops the file (48D660 abandons the remaining roots).
4C0870 never writes DamageTypeData+2C: the fixture pre-fills it with POISON at
constructor entry so a never-stored +2C is visible (native: allocator bytes).

Animation sources go through actual 463B50 -> 463440 -> 4632D0 -> 461F70 with
the same typed node getter / Sequence transports as
research/skirmish-20261017-object-animation/native_actor_animation.py. The
miss diagnostic 410C40 is captured with its formatted arguments, then the
case stops ("required:diagnostic"), exactly like that producer.

This fixture does not certify the whole ActorData loader, the Glyph registry,
localized names, FX compilation or any later use of Animation+28.
"""
from pathlib import Path
import sys, json, hashlib, gc, re
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(ROOT / 'research/phase8-definitions')); import native_definition_registry as h
sys.path.insert(0, str(ROOT / 'research/skirmish-20261017-svar-field-order')); import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_FPCW
import struct

DATA = GAME / 'reconstruction/assets/original/Data'
GLYPHS = {'ch_gun', 'ch_fire', 'ch_explosive', 'ch_ap'}
POISON = 0xcdcdcdcd  # fixture pre-fill of the fresh DamageTypeData+2C (4C0870 never writes it)
FILTERS = [-1, 666, 0, 1, 2, 3, 7, -2, -5, 665, 667, 1000, 0x7fffffff, -0x80000000, POISON - (1 << 32)]


class Required(Exception):
    pass


def damage_type(ids, glyph, flt):
    return ('[DamageType template=DamageType]\r\n{\r\n\tIDS = %s\r\n\tname = "#x_%s"\r\n\tglyph = %s\r\n'
            '\tcolor = 1,0,0\r\n\tdamage_filter = %d\r\n}\r\n' % (ids, ids, glyph, flt))


def registries():
    real = (DATA / 'Game/damage_types.tgi').read_bytes()
    yield 'real', 'Game/damage_types.tgi', real
    yield 'empty', None, None
    yield 'absent', None, None
    yield 'duplicate_large', 'synthetic/dup.tgi', ''.join(
        damage_type(i, 'ch_gun', f) for i, f in [('a', 7), ('b', 7), ('c', 0x7fffffff), ('d', 1000)]).encode()
    # 4C0A80 stores the negative +2C, THEN throws: the entry stays registered,
    # later roots of the file are abandoned by 48D660.
    yield 'negative_registered', 'synthetic/neg.tgi', ''.join(
        damage_type(i, 'ch_gun', f) for i, f in [('a', 3), ('b', -5), ('c', 2)]).encode()
    # Unknown glyph throws before damage_filter is read: +2C is never stored.
    yield 'unknown_glyph', 'synthetic/glyph.tgi', ''.join(
        damage_type(i, g, f) for i, g, f in [('a', 'ch_gun', 1), ('b', 'not_a_glyph', 2), ('c', 'ch_gun', 0)]).encode()


def die_animations(path):
    """Authored [Animation Template=DieAnimation] IDS/damage_filter pairs."""
    text = (DATA / path).read_text(encoding='latin-1')
    blocks = re.split(r'\[Animation\s+Template\s*=\s*', text, flags=re.I)[1:]
    out = []
    for block in blocks:
        name = re.match(r'(\w+)\]', block).group(1)
        body = block
        ids = re.findall(r'^\s*IDS\s*=\s*(\S+)', body, flags=re.M | re.I)
        flt = re.findall(r'^\s*damage_filter\s*=\s*(-?\d+)', body, flags=re.M | re.I)
        if name.lower() != 'dieanimation':
            assert not flt, (path, name)
            continue
        assert len(ids) == 1 and len(flt) == 1, (path, ids, flt)
        out.append({'ids': ids[0], 'damage_filter': int(flt[0])})
    return out


def run(image, pc):
    full.order.T.P = image
    m = h.Machine()
    m.u.reg_write(UC_X86_REG_FPCW, full.order.PCS[pc])
    pins = {}; seams = set(); reads = []
    getters = {0x5b54d0: 4, 0x5b4ed0: 4, 0x5b5620: 4, 0x5b4c30: 8}

    def capture(u, at, size, _):
        if image.base <= at < image.base + image.image_size:
            if u.reg_read(UC_X86_REG_EIP) == at:
                pins[at] = bytes(u.mem_read(at, size)).hex()
            else:
                seams.add(at)
        if at == 0x4c0870:
            m.w(u.reg_read(UC_X86_REG_ECX) + 0x2c, POISON)
        if at in getters and reads is not None:
            try:
                reads.append(m.raw_wstr(m.r(u.reg_read(UC_X86_REG_ESP) + getters[at])))
            except Exception:
                reads.append(None)
    m.u.hook_add(UC_HOOK_CODE, capture)
    m.thiscall(0x4013e0, 0xb43b40, [])
    glyph_calls = []

    def glyph(mm, u):
        name = mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP) + 4)); glyph_calls.append(name)
        mm.ret(0x51ed0000 + len(glyph_calls) if name in GLYPHS else 0, pop=4)

    def text(mm, u):
        key = mm.wstr(mm.r(mm.r(u.reg_read(UC_X86_REG_ESP) + 4)))
        slot = mm.alloc(4); mm.w(slot, mm.cstring(key)); mm.ret(slot, pop=8)
    m.stubs[0x48d500] = glyph
    m.stubs[0x4b9f40] = text
    r = m.reparse((DATA / 'Templates/template_damage_type.tgi').read_bytes(), 'Templates/template_damage_type.tgi')
    assert r['error'] is None, r
    gd = m.r(0xb36530)
    built = {}
    for name, path, source in registries():
        if name == 'absent':
            built[name] = dict(manager=0, roots=[], entries=[]); continue
        mgr = m.alloc(0x4c); m.thiscall(0x481410, mgr, [0xb, 0])
        assert m.r(mgr) == 0xa0cfe4 and m.r(mgr + 8) == 0 and m.r(0xb3aaf8) == mgr
        roots = []
        if source is not None:
            r = m.reparse(source, path); assert r['error'] is None, r
            root = r['root']
            for i in range(m.r(root + 0x38)):
                node = m.r(m.r(root + 0x34) + 4 * i)
                del reads[:]; glyph_calls.clear()
                res = m.thiscall(0x483a40, mgr, [node])
                roots.append(dict(label=m.wstr(m.r(node)), reads=list(reads), glyph_lookups=list(glyph_calls),
                                  status='ok' if 'eax' in res else 'thrown', message=res.get('thrown')))
                if 'thrown' in res:
                    break  # 48D660: remaining roots of this file are not dispatched
        entries = []
        for i in range(m.r(mgr + 0x28)):
            e = m.r(m.r(mgr + 0x24) + 4 * i)
            stored = roots[i]['status'] == 'ok' or 'damage_filter' in roots[i]['reads']
            entries.append(dict(ids=m.wstr(m.r(e + 8)), vtable=m.r(e), index=m.r(e + 0xc),
                                glyph_nonzero=m.r(e + 0x28) != 0,
                                stored=stored, raw_2c=struct.unpack('<i', struct.pack('<I', m.r(e + 0x2c)))[0]))
        built[name] = dict(manager=mgr, roots=roots, entries=entries)
    reads = None
    # ------------------------------------------------ 463B50 Animation sources
    m.thiscall(0x499ff0, 0xb3ae88, [])
    registry = m.alloc(12)
    for i, name in enumerate(['WhenStart', 'WhenDo', 'WhenEnd']):
        entry = m.alloc(8); m.w(entry + 4, m.cstring(name)); m.w(registry + 4 * i, entry)
    m.w(0xb3af58, registry)
    inputs = {}; trace = []; node = m.alloc(4); resource = m.alloc(0x40); diagnostics = []

    def key_of(mm, u):
        return mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP) + 4)).lower()

    def typed_string(mm, u):
        key = key_of(mm, u); trace.append('read:' + key)
        slot = mm.alloc(4); mm.w(slot, mm.cstring(inputs[key])); mm.ret(slot, pop=4)

    def fixed(mm, u):
        key = key_of(mm, u); trace.append('read:' + key)
        value = mm.alloc(4); mm.w(value, inputs[key]); stub = mm.alloc(16)
        mm.u.mem_write(stub, b'\xd9\x05' + struct.pack('<I', value) + b'\xc2\x04\x00'); u.reg_write(UC_X86_REG_EIP, stub)

    def integer(mm, u):
        key = key_of(mm, u); trace.append('read:' + key); mm.ret(inputs[key] & 0xffffffff, pop=4)

    def count(mm, u):
        key = mm.raw_wstr(mm.r(u.reg_read(UC_X86_REG_ESP) + 4)); trace.append('children:' + key)
        mm.ret(1 if key == 'Animation' else 0, pop=4)

    def relative(mm, u):
        arg = mm.r(u.reg_read(UC_X86_REG_ESP) + 4); value = inputs['folder'] + '/' + mm.wstr(mm.r(arg))
        trace.append('relative:' + value); mm.w(arg, mm.cstring(value)); mm.ret(arg, pop=4)

    def sequence(mm, u):
        trace.append('sequence:' + mm.wstr(mm.r(mm.r(u.reg_read(UC_X86_REG_ESP) + 4)))); mm.ret(resource)

    def diagnostic(mm, u):
        s = u.reg_read(UC_X86_REG_ESP); fmt = mm.raw_wstr(mm.r(s + 4))
        args = [mm.raw_wstr(mm.r(s + 8)), mm.raw_wstr(mm.r(s + 12))]
        if '%d' in fmt:
            args.append(struct.unpack('<i', struct.pack('<I', mm.r(s + 16)))[0])
        diagnostics.append(dict(format=fmt, args=args)); trace.append('diagnostic'); raise Required('diagnostic')
    for at, fn in [(0x5b54d0, typed_string), (0x5b5130, fixed), (0x5b4ed0, integer), (0x5b5380, integer),
                   (0x5b4220, count), (0x4763c0, relative), (0x5a80d0, sequence), (0x410c40, diagnostic)]:
        m.stubs[at] = fn
    m.stubs[0x42be10] = lambda mm, u: mm.ret(node)
    base = dict(ids='die0', type='die', kf='%s%s.kf', prefix='unit_', folder='Art/Units', length=0x3f800000,
                key_ratio=0x3f000000, walk_distance=0x3f800000, animation_blend=0, sister_anim_type='',
                weight=0x3f800000, damage_threshold=0, damage_filter=-1)

    def load(case, manager):
        nonlocal inputs, trace
        inputs = case; trace = []; del diagnostics[:]
        m.w(gd + 0x3a4, manager)
        actor = m.alloc(0x700); m.w(actor + 0xb0, m.cstring(case['prefix'])); m.w(actor + 8, m.cstring('actor_ids'))
        owner = m.alloc(0x6c); m.thiscall(0x45f3e0, owner, [])
        name = m.alloc(4); m.w(name, m.cstring('Animation')); status = 'ok'
        try:
            m.thiscall(0x463b50, actor, [node, name, 0, owner, 0])
        except Required as e:
            status = 'required:' + str(e)
        except AssertionError:
            status = 'fault:' + ','.join('%s@%s' % (pc_, a) for pc_, a, _ in m.null_accesses); m.null_accesses.clear()
        n = m.r(owner + 4); a = m.r(m.r(owner) + 4 * (n - 1)) if n else 0
        return dict(status=status, trace=trace, diagnostic=diagnostics[0] if diagnostics else None, count=n,
                    kind=m.r(a + 8) if a else None,
                    stored_filter=struct.unpack('<i', struct.pack('<I', m.r(a + 0x28)))[0] if a else None,
                    complete_weight=m.r(m.r(owner + 0x60) + 4 * 3))
    lookups = []
    for reg in ['real', 'empty', 'absent', 'duplicate_large', 'negative_registered', 'unknown_glyph']:
        for flt in FILTERS:
            lookups.append(dict(registry=reg, input=dict(base, damage_filter=flt), **load(dict(base, damage_filter=flt), built[reg]['manager'])))
    for typ, flt in [('idle', 0), ('idle', -1), ('IDLE', 2), ('die_water', 1), ('die_water', 3)]:
        case = dict(base, type=typ, ids=typ, damage_filter=flt)
        lookups.append(dict(registry='real', input=case, **load(case, built['real']['manager'])))
    real = []
    for path in ['Units/GER/ger_infantry_mg.tgi', 'Units/USA/usa_infantry_flame.tgi', 'SpecialOps/Units/saboteur.tgi']:
        anims = die_animations(path)
        rows = []
        for a in anims:
            case = dict(base, ids=a['ids'], damage_filter=a['damage_filter'])
            rows.append(dict(input=case, **load(case, built['real']['manager'])))
        real.append(dict(source=path, sha256=hashlib.sha256((DATA / path).read_bytes()).hexdigest(), animations=rows))
    for v in built.values():
        v.pop('manager')
    return dict(registries=built, lookups=lookups, real_definitions=real), pins, seams


def main():
    rows = []; pins = {}; seams = set()
    if '--probe' in sys.argv:
        a, _, _ = run(h.P, 'Double53')
        print(json.dumps(a['registries'], indent=1))
        for row in a['lookups']:
            print(row['registry'], row['input']['type'], row['input']['damage_filter'], row['status'], row['diagnostic'], row['count'], row['stored_filter'])
        for d in a['real_definitions']:
            print(d['source'], [(r['input']['ids'], r['input']['damage_filter'], r['status']) for r in d['animations']])
        return
    for pc in full.order.PCS:
        a, pa, sa = run(h.P, pc); gc.collect(); b, pb, sb = run(h.V, pc); gc.collect()
        assert a == b and pa == pb
        rows.append(dict(precision=pc, output=a)); pins.update(pa); seams |= sa | sb
    for a, b in pins.items():
        assert bytes.fromhex(b) == h.P.range(a, len(bytes.fromhex(b))) == h.V.range(a, len(bytes.fromhex(b)))
    for at in [0x4620a6, 0x4620ae, 0x4620c9, 0x4620d6, 0x4620e7, 0x4c0b4b, 0x4c0b52, 0x481630]:
        assert at in pins, hex(at)
    doc = dict(images={'original': hashlib.sha256((GAME / 'AA.exe').read_bytes()).hexdigest(),
                       'v14': hashlib.sha256((GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes()).hexdigest()},
               scope=__doc__, glyph_seam=sorted(GLYPHS), filters=FILTERS,
               template=dict(path='Templates/template_damage_type.tgi',
                             text=(DATA / 'Templates/template_damage_type.tgi').read_bytes().decode('latin-1'),
                             sha256=hashlib.sha256((DATA / 'Templates/template_damage_type.tgi').read_bytes()).hexdigest()),
               registry_sources={name: None if path is None else dict(path=path, text=source.decode('latin-1'),
                                                                     sha256=hashlib.sha256(source).hexdigest())
                                 for name, path, source in registries()},
               rows=rows, executed_pins={hex(a): b for a, b in sorted(pins.items())},
               intercepted_not_executed=[hex(a) for a in sorted(seams)],
               producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
    path = HERE / 'native-damage-filter.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8', newline='\n')
    else:
        assert json.loads(path.read_text(encoding='utf-8')) == doc
    print(json.dumps(dict(rows=len(rows), machines=2 * len(rows), lookups=2 * sum(len(r['output']['lookups']) for r in rows),
                          real=2 * sum(len(d['animations']) for r in rows for d in r['output']['real_definitions']), pins=len(pins))))


if __name__ == '__main__':
    main()
