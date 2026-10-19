"""Data vf18 release / vf1C ready family under Unicorn, ORIGINAL AA.exe code only.

Coordinators: ObjectData 49BB00 (vf18) / 49BBB0 (vf1C) and ActorData 45DF40
(vf18, A05540+18) / 45EF20 (vf1C, A05540+1C). Their children are recorded
seams or virtual stubs; ready children answer the case's bool. 4BB050/4BB060
(Data+20) run natively. Mutating callbacks prove which counts are reread.
Kernels, each executed natively with only Resource vf14 (5AA550) recorded and
Resource+8 flags as memory: FX lists 428900/428950 (cells {node,next}; node
vf24/vf2C class answers; +3C VFX / +44 Audio vf18/vf1C stubs), Ability
4BE450/4BE480 (Definition+70+8i lists, 428900/428950 recorded), VFX
494390/494450/4943A0/494810, Audio 46B260/46B060, Aspect 469BB0/469C50 with
real 49B820/49B860 and 5A4ED0, Banner 46C7D0/46C800 with the real Plane
(A0A57C) and Decal (A0A570) vtables, Actor Animation 45EAC0 (vf4, A055D8+4)
and 45EB10. Default compares the fixture; --write regenerates it.
"""
from pathlib import Path
import sys, json, hashlib, itertools
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
import native_machine as nm
from unicorn.x86_const import UC_X86_REG_ECX

PCS = list(nm.PRECISIONS)


class Stop(Exception):
    pass


class Lab:
    def __init__(self, pc, image):
        m = self.m = nm.Machine(pc)
        if image == 'v14':
            old = (nm.original.image_size + 4095) & ~4095
            new = (nm.modified.image_size + 4095) & ~4095
            if new > old:
                m.u.mem_map(nm.original.base + old, new - old)
            for vs, r, size, off in nm.modified.sections:
                if size:
                    m.u.mem_write(nm.modified.base + r, nm.modified.b[off:off + size])
        self.trace = []
        self.names = {}

    def obj(self, name, size=0x60):
        p = self.m.alloc(size)
        self.names[p] = name
        return p

    def virtual(self, name, slots, size=0x60):
        """Object whose vtable slots are recording stubs. slots: {offset: (nargs, result)}."""
        m = self.m
        p = self.obj(name, size)
        vt = m.alloc(0x40)
        m.w(p, vt)
        for off, (nargs, result) in slots.items():
            m.w(vt + off, m.stub(lambda mm, off=off, nargs=nargs, result=result: self.child(mm, name, off, nargs, result)))
        return p

    def child(self, mm, name, off, nargs, result):
        self.trace.append({'call': name, 'args': [mm.arg(i + 1) for i in range(nargs)]})
        self.on_child(name)
        if self.stop == name:
            raise Stop(name)
        value = result() if callable(result) else result
        mm.ret(value, 4 * nargs)

    on_child = staticmethod(lambda name: None)
    stop = None

    def resource(self, name, loaded):
        p = self.virtual(name, {0x14: (1, 0)})
        self.m.w(p + 8, 0x200010 | (1 if loaded else 0))
        return p

    def run(self, entry, ecx, args):
        status = 'ok'
        try:
            value = self.m.call(entry, ecx, args)
        except Stop as e:
            status = 'stop_' + str(e)
            value = None
        #Release entries take delta and are void; only ready entries return AL.
        return status, None if value is None or args else value & 0xff


# ---------------------------------------------------------------- coordinators
LABELS_OBJECT = ['portrait', 'banner', 'aspect0', 'aspect1', 'texture_ec', 'texture_f0', 'texture_138'] + \
    ['viewfx%d' % i for i in range(6)]
ACTOR_FX = [0x194, 0x19c, 0x24c, 0x254, 0x294, 0x29c, 0x598, 0x5a0]
LABELS_ACTOR = ['animation0', 'animation1', 'animation2'] + ['actorfx%d' % o for o in ACTOR_FX] + \
    ['ability0', 'ability1', 'ability2', 'ability3', 'extra', 'deployment']


def coordinator_cases():
    base = dict(actor=True, ready=False, initial=5, delta=3, aspects=2, portrait=True, banner=True,
                textures=True, unloaded=None, animations=True, abilities=True, extra=True,
                deployment=True, mask=0x80004, false_at=None, mutation=None, stop=None)
    for actor, ready in itertools.product([False, True], [False, True]):
        yield dict(base, actor=actor, ready=ready)
        for k in ['portrait', 'banner', 'textures', 'animations', 'abilities', 'extra', 'deployment']:
            yield dict(base, actor=actor, ready=ready, **{k: False})
        yield dict(base, actor=actor, ready=ready, aspects=0)
        yield dict(base, actor=actor, ready=ready, aspects=-1)
        yield dict(base, actor=actor, ready=ready, mask=0x4)
        for m in ['aspect_count', 'animation_inner', 'animation_outer', 'ability_count', 'mask_clear', 'mask_set']:
            yield dict(base, actor=actor, ready=ready, mutation=m, mask=0x4 if m == 'mask_set' else 0x80004)
    for ready in [False, True]:
        yield dict(base, actor=True, ready=ready, initial=0, delta=0xffffffff)
        yield dict(base, actor=True, ready=ready, initial=0x80000000, delta=1)
    for label in LABELS_OBJECT + LABELS_ACTOR:
        if label.startswith('texture'):
            yield dict(base, ready=True, unloaded=label)
            yield dict(base, ready=True, actor=False, unloaded=label)
        else:
            yield dict(base, ready=True, false_at=label)
            if label in LABELS_OBJECT:
                yield dict(base, ready=True, actor=False, false_at=label)
        yield dict(base, ready=False, stop=label)
    for initial in [0, 1, 0xffffffff]:
        yield dict(base, ready=True, initial=initial)


def coordinator(c, pc, image):
    L = Lab(pc, image)
    m = L.m
    data = L.obj('data', 0x5b0)
    m.w(data + 0x20, c['initial'])
    m.w(data + 0x174, c['mask'])
    ready = c['ready']
    slot = 0x1c if ready else 0x18
    answer = lambda name: (lambda: 0 if c['false_at'] == name else 1)
    nargs = 0 if ready else 1
    m.w(data + 0x104, L.virtual('portrait', {slot: (nargs, answer('portrait'))}) if c['portrait'] else 0)
    m.w(data + 0x534, L.virtual('deployment', {slot: (nargs, answer('deployment'))}) if c['deployment'] else 0)
    banner = L.obj('banner') if c['banner'] else 0
    m.w(data + 0x134, banner)
    aspects = [L.obj('aspect%d' % i) for i in range(2)]
    arr = m.array(aspects)
    m.w(data + 0x10c, arr)
    m.w(data + 0x110, c['aspects'] & 0xffffffff)
    for off, name in [(0xec, 'texture_ec'), (0xf0, 'texture_f0'), (0x138, 'texture_138')]:
        m.w(data + off, L.resource(name, c['unloaded'] != name) if c['textures'] else 0)
    anims = [L.virtual('animation%d' % i, {4: (1, 0)}) for i in range(3)]
    sets = [0, m.alloc(8), m.alloc(8)]
    m.w(sets[1], m.array(anims[:2]))
    m.w(sets[1] + 4, 2)
    m.w(sets[2], m.array(anims[2:]))
    m.w(sets[2] + 4, 1)
    m.w(data + 0x1bc, m.array(sets))
    m.w(data + 0x1c0, 3 if c['animations'] else 0)
    abil = [L.obj('ability%d' % i) for i in range(4)]
    extra = L.obj('extra')
    m.w(data + 0x1ec, m.array(abil[:2]))
    m.w(data + 0x1f0, 2 if c['abilities'] else 0)
    m.w(data + 0x21c, m.array(abil[2:]))
    m.w(data + 0x220, 2 if c['abilities'] else 0)
    m.w(data + 0x2a4, extra if c['extra'] else 0)

    def seam_child(label_of, nargs):
        def seam(mm):
            name = label_of(mm)
            L.trace.append({'call': name, 'args': [mm.arg(i + 1) for i in range(nargs)]})
            mutate(name)
            if c['stop'] == name:
                raise Stop(name)
            mm.ret(0 if c['false_at'] == name else 1, 4 * nargs)
        return seam

    def mutate(name):
        change = c['mutation']
        if name == 'aspect0' and change == 'aspect_count':
            m.w(data + 0x110, 1)
        if name == 'animation0' and change == 'animation_inner':
            m.w(sets[1] + 4, 1)
        if name == 'animation0' and change == 'animation_outer':
            m.w(data + 0x1c0, 2)
        if name == 'ability0' and change == 'ability_count':
            m.w(data + 0x1f0, 0)
            m.w(data + 0x220, 1)
        if name == 'actorfx404' and change == 'mask_clear':
            m.w(data + 0x174, 0)
        if name == 'actorfx404' and change == 'mask_set':
            m.w(data + 0x174, 0x80000)
    L.on_child = mutate
    L.stop = c['stop']
    ecx = lambda mm: mm.u.reg_read(UC_X86_REG_ECX)
    name_of = lambda mm: L.names[ecx(mm)]

    def fx_name(mm):
        off = ecx(mm) - data
        return ('viewfx%d' % ((off - 0xbc) // 8)) if 0xbc <= off < 0xec else 'actorfx%d' % off
    if ready:
        m.seam(0x46c800, seam_child(name_of, 0))
        m.seam(0x469c50, seam_child(name_of, 0))
        m.seam(0x428950, seam_child(fx_name, 0))
        m.seam(0x45eb10, seam_child(name_of, 0))
        m.seam(0x4be480, seam_child(name_of, 0))
        entry = 0x45ef20 if c['actor'] else 0x49bbb0
        status, value = L.run(entry, data, [])
    else:
        m.seam(0x46c7d0, seam_child(name_of, 1))
        m.seam(0x469bb0, seam_child(name_of, 1))
        m.seam(0x428900, seam_child(fx_name, 1))
        m.seam(0x4be450, seam_child(name_of, 1))
        entry = 0x45df40 if c['actor'] else 0x49bb00
        status, value = L.run(entry, data, [c['delta']])
    return dict(status=status, value=value, count=m.r(data + 0x20), trace=L.trace), m.visited


# --------------------------------------------------------------------- kernels
def fx_cases():
    for ready in [False, True]:
        for nodes in [[], ['vfx'], ['sfx'], ['other'], ['vfx', 'sfx', 'other', 'vfx'], ['both']]:
            yield dict(kernel='fx_list', ready=ready, nodes=nodes, false_at=None, delta=2)
            if ready:
                for i, n in enumerate(nodes):
                    if n in ('vfx', 'sfx', 'both'):
                        for side in (['vfx', 'sfx'] if n == 'both' else [n]):
                            yield dict(kernel='fx_list', ready=True, nodes=nodes, false_at=[i, side], delta=0)


def fx_kernel(c, L):
    m = L.m
    head = 0
    cells = []
    for i, kind in reversed(list(enumerate(c['nodes']))):
        node = m.alloc(0x60)
        vt = m.alloc(0x40)
        m.w(node, vt)
        v24 = kind in ('vfx', 'both')
        v2c = kind in ('sfx', 'both')
        m.w(vt + 0x24, m.stub(lambda mm, v=v24: mm.ret(int(v), 0)))
        m.w(vt + 0x2c, m.stub(lambda mm, v=v2c: mm.ret(int(v), 0)))
        for off, side in [(0x3c, 'vfx'), (0x44, 'sfx')]:
            fl = c['false_at'] == [i, side]
            slot = 0x1c if c['ready'] else 0x18
            target = L.virtual('node%d_%s' % (i, side), {slot: (0 if c['ready'] else 1, 0 if fl else 1)})
            m.w(node + off, target)
        cell = m.alloc(8)
        m.w(cell, node)
        m.w(cell + 4, head)
        head = cell
    lst = m.alloc(8)
    m.w(lst, head)
    if c['ready']:
        return L.run(0x428950, lst, [])
    return L.run(0x428900, lst, [c['delta']])


def ability_cases():
    for ready in [False, True]:
        yield dict(kernel='ability', ready=ready, false_at=None, delta=7)
        if ready:
            for i in range(5):
                yield dict(kernel='ability', ready=True, false_at=i, delta=0)


def ability_kernel(c, L):
    m = L.m
    definition = L.obj('definition', 0xa0)
    ability = m.alloc(8)
    m.w(ability, definition)

    def seam(mm):
        off = mm.u.reg_read(UC_X86_REG_ECX) - definition
        i = (off - 0x70) // 8
        L.trace.append({'call': 'list%d' % i, 'offset': off, 'args': [] if c['ready'] else [mm.arg(1)]})
        mm.ret(0 if c['false_at'] == i else 1, 0 if c['ready'] else 4)
    m.seam(0x428950 if c['ready'] else 0x428900, seam)
    if c['ready']:
        return L.run(0x4be480, ability, [])
    return L.run(0x4be450, ability, [c['delta']])


def vfx_cases():
    for entry in ['494390', '494450', '4943a0', '494810']:
        for initial in [0, 1, 0x80000000, 0x7fffffff, 0xffffffff]:
            for resource in [None, 'loaded', 'unloaded']:
                if entry in ('494390', '4943a0') and resource == 'unloaded':
                    continue
                yield dict(kernel='vfx', entry=entry, initial=initial, resource=resource, delta=3)


def vfx_kernel(c, L):
    m = L.m
    d = L.obj('vfx', 0x60)
    m.w(d + 0x20, c['initial'])
    m.w(d + 0x40, L.resource('resource40', c['resource'] == 'loaded') if c['resource'] else 0)
    entry = int(c['entry'], 16)
    out = L.run(entry, d, [] if entry in (0x4943a0, 0x494810) else [c['delta']])
    L.extra = {'count': m.r(d + 0x20)}
    return out


def audio_cases():
    for ready in [False, True]:
        for counts in [[0, 0, 0], [1, 0, 0], [2, 3, 1], [0, 1, 2]]:
            yield dict(kernel='audio', ready=ready, counts=counts, unloaded=None, delta=4)
            if ready:
                for li, n in enumerate(counts):
                    for j in range(n):
                        yield dict(kernel='audio', ready=True, counts=counts, unloaded=[li, j], delta=0)


def audio_kernel(c, L):
    m = L.m
    a = L.obj('audio', 0x60)
    for li, (off, n) in enumerate(zip([0x34, 0x40, 0x4c], c['counts'])):
        items = [L.resource('sound%d_%d' % (li, j), c['unloaded'] != [li, j]) for j in range(n)]
        m.w(a + off, m.array(items))
        m.w(a + off + 4, n)
    if c['ready']:
        return L.run(0x46b060, a, [])
    return L.run(0x46b260, a, [c['delta']])


def aspect_cases():
    base = dict(kernel='aspect', count=5, delta=2, model='loaded', set=[True, False, True], skins=[True, True],
                extra=False, extra_count=2, unloaded=None, ready=False)
    for ready in [False, True]:
        yield dict(base, ready=ready)
        yield dict(base, ready=ready, set=None)
        yield dict(base, ready=ready, set=[])
        yield dict(base, ready=ready, skins=[False, True])
        yield dict(base, ready=ready, skins=[True, False])
        yield dict(base, ready=ready, extra=True)
        yield dict(base, ready=ready, extra=True, extra_count=0)
        yield dict(base, ready=ready, model='unloaded')
    for count, delta in [(5, 2), (2, 2), (1, 2), (0, 1), (0x80000000, 1), (3, 0xffffffff)]:
        yield dict(base, count=count, delta=delta)
        yield dict(base, count=count, delta=delta, extra=True)
    for u in ['set0', 'set2', 'skin0', 'skin1', 'extra0', 'extra1']:
        yield dict(base, ready=True, extra=True, unloaded=u)


def aspect_kernel(c, L):
    m = L.m
    a = L.obj('aspect', 0x60)
    m.w(a + 0x18, c['count'])
    model = L.resource('model', c['model'] == 'loaded')
    m.w(model + 0x7c, c['extra_count'])
    m.w(a + 0x28, model)
    if c['set'] is None:
        m.w(a + 0x38, 0)
    else:
        entries = []
        for i, present in enumerate(c['set']):
            e = m.alloc(0x20)
            m.w(e + 0x1c, L.resource('set%d' % i, c['unloaded'] != 'set%d' % i) if present else 0)
            entries.append(e)
        s = m.alloc(8)
        m.w(s, m.array(entries))
        m.w(s + 4, len(entries))
        m.w(a + 0x38, s)
    for i, present in enumerate(c['skins']):
        m.w(a + 0x30 + 4 * i, L.resource('skin%d' % i, c['unloaded'] != 'skin%d' % i) if present else 0)
    m.wb(a + 0x54, int(c['extra']))
    pairs = m.alloc(8 * max(c['extra_count'], 1))
    for i in range(c['extra_count']):
        m.w(pairs + 8 * i, L.resource('extra%d' % i, c['unloaded'] != 'extra%d' % i))
        m.w(pairs + 8 * i + 4, 0)
    m.w(a + 0x48, pairs)
    m.w(a + 0x4c, c['extra_count'])
    out = L.run(0x469c50, a, []) if c['ready'] else L.run(0x469bb0, a, [c['delta']])
    L.extra = {'count': m.r(a + 0x18), 'extra': m.rb(a + 0x54)}
    return out


def banner_cases():
    for ready in [False, True]:
        for shape in [[], [[]], [[0]], [[2, 1], [], [3]]]:
            yield dict(kernel='banner', ready=ready, shape=shape, unloaded=None, delta=5)
            if ready:
                for pi, plane in enumerate(shape):
                    for di, n in enumerate(plane):
                        for t in range(n):
                            yield dict(kernel='banner', ready=True, shape=shape, unloaded=[pi, di, t], delta=0)


def banner_kernel(c, L):
    m = L.m
    planes = []
    for pi, plane in enumerate(c['shape']):
        decals = []
        for di, n in enumerate(plane):
            d = m.alloc(0x20)
            m.w(d, 0xa0a570)
            tex = [L.resource('texture%d_%d_%d' % (pi, di, t), c['unloaded'] != [pi, di, t]) for t in range(n)]
            m.w(d + 0xc, m.array(tex))
            m.w(d + 0x10, n)
            decals.append(d)
        p = m.alloc(0x20)
        m.w(p, 0xa0a57c)
        m.w(p + 0x10, m.array(decals))
        m.w(p + 0x14, len(decals))
        planes.append(p)
    b = L.obj('banner', 0x40)
    m.w(b + 0x24, m.array(planes))
    m.w(b + 0x28, len(planes))
    if c['ready']:
        return L.run(0x46c800, b, [])
    return L.run(0x46c7d0, b, [c['delta']])


def animation_cases():
    for ready in [False, True]:
        for resource in [None, 'loaded', 'unloaded']:
            for timed in [0, 2]:
                yield dict(kernel='animation', ready=ready, resource=resource, timed=timed, false_at=None, delta=6)
        if ready:
            for f in ['list0', 'list1', 'list2', 'timed0', 'timed1']:
                yield dict(kernel='animation', ready=True, resource='loaded', timed=2, false_at=f, delta=0)


def animation_kernel(c, L):
    m = L.m
    a = L.obj('animation', 0x60)
    m.w(a, 0xa055d8)
    m.w(a + 0x34, L.resource('sequence', c['resource'] == 'loaded') if c['resource'] else 0)
    head = 0
    for i in reversed(range(c['timed'])):
        n = m.alloc(0x10)
        m.w(n + 0xc, head)
        L.names[n + 4] = 'timed%d' % i
        head = n
    m.w(a + 0x50, head)
    for i in range(3):
        L.names[a + 0x38 + 8 * i] = 'list%d' % i

    def seam(mm):
        name = L.names[mm.u.reg_read(UC_X86_REG_ECX)]
        L.trace.append({'call': name, 'args': [] if c['ready'] else [mm.arg(1)]})
        mm.ret(0 if c['false_at'] == name else 1, 0 if c['ready'] else 4)
    m.seam(0x428950 if c['ready'] else 0x428900, seam)
    if c['ready']:
        return L.run(0x45eb10, a, [])
    return L.run(0x45eac0, a, [c['delta']])


KERNELS = [(fx_cases, fx_kernel), (ability_cases, ability_kernel), (vfx_cases, vfx_kernel),
           (audio_cases, audio_kernel), (aspect_cases, aspect_kernel), (banner_cases, banner_kernel),
           (animation_cases, animation_kernel)]


def kernel(c, run, pc, image):
    L = Lab(pc, image)
    L.extra = {}
    status, value = run(c, L)
    return dict(status=status, value=value, trace=L.trace, **L.extra), L.m.visited


def main():
    rows, pins = [], {}
    for pc in PCS:
        jobs = [(c, coordinator) for c in coordinator_cases()]
        for cases, run in KERNELS:
            jobs += [(c, lambda c, pc, image, run=run: kernel(c, run, pc, image)) for c in cases()]
        for c, run in jobs:
            pair = []
            for image in ['original', 'v14']:
                out, visited = run(c, pc, image)
                pair.append(out)
                pins.update(visited)
            assert pair[0] == pair[1]
            rows.append(dict(precision=pc, input=c, output=pair[0]))
    for a, b in pins.items():
        assert b == nm.original.range(a, len(b)) == nm.modified.range(a, len(b))
    doc = {**nm.header(__file__, __doc__), 'rows': rows, 'instructions': nm.instructions(pins),
           'harness_sha256': hashlib.sha256((ROOT / 'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
    path = HERE / 'native-data-release-ready.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1))
    else:
        assert json.loads(path.read_text()) == doc, 'fixture differs'
    print(json.dumps(dict(rows=len(rows), pins=len(pins), sha256=hashlib.sha256(path.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
