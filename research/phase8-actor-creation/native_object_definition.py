"""ObjectData LoadData 49DC40 (the fields fresh creation reads from an
Object / DynamicDoodad / ActorData definition), run NATIVELY from its entry
to the end of its Size block (49E097) on fixture TGI nodes.

Proves which TGI field lands at which definition offset:
  +24 Size.height, +28 / +2C Size.scale_min / scale_max (fixed getter
  5B5130 + fstp dword), +34 needs_space_for_settlement_zone (bool getter
  5B5380, byte store), +38.. shape A ("blocking": blocking_shape with
  blocking_radius / square_size / rect_width, rect_height), +50.. shape B
  ("soft"), +68.. target shape, +80 explicit-target byte.
Seams: ThingData LoadData 4BB310 (base, recorded), node getters 5B5380 /
5B5620 / 5B4410 / 5B5130 / 5B54D0 on fixture nodes, the shape enum lookup
497220 (POINT 0, CIRCLE 1, SQUARE 2, RECT 3 as registered by 46F5F6;
research/phase7-definitions/native-shapes.json), data errors 410C40 (end the
case), log 404ED0. The shape setters 46D8D0 / 46D8E0 / 46EB60 / 46E0E0 run
natively. PC24 / PC53 / PC64.
Usage: python native_object_definition.py -> native-object-definition.json
"""
import json, struct, sys, time
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from actor_machine import ActorMachine, VISITED, H, f32, nm, Stop, Throw
from unicorn.x86_const import *

SHAPES = {'POINT': 0, 'CIRCLE': 1, 'SQUARE': 2, 'RECT': 3}
NATIVE = {0x46e060, 0x49dc40, 0x46d8d0, 0x46d8e0, 0x46eb60, 0x46e0e0, 0x405b80, 0x4012c0, 0x4047a0, 0x401560}


def run(case):
    m = ActorMachine(case['pc'])
    import actor_machine
    actor_machine.NATIVE |= NATIVE
    root = m.alloc(0x40, 0)
    size = m.alloc(0x40, 0)
    m.labels[root] = 'root'
    m.labels[size] = 'size'
    values = {root: case['root'], size: case.get('size') or {}}
    d = m.alloc(0x200, 0xa5)
    m.w(d + 8, m.cstring('def'))
    # 49C760's effects on the fields the Size block touches: +30 empty
    # CString, the three shapes through the native zero constructor 46E060.
    m.w(d + 0x30, m.cstring(''))
    for base in (0x38, 0x50, 0x68):
        m.call(0x46e060, d + base, [], 0)

    def name_arg(m, i=0):
        return m.read_wide(m.arg(i))

    def getter(kind):
        def h(m):
            node, name = m.ecx(), name_arg(m)
            v = values.get(node, {}).get(name)
            m.record(kind, node=m.label(node), field=name)
            if kind == 'string' or kind == 'localized':
                p = m.alloc(4, 0)
                m.w(p, m.cstring(v if v is not None else ''))
                m.ret(4, p)
            elif kind == 'bool':
                m.ret(4, 1 if v else 0)
            else:
                m.ret_float(v if v is not None else 0, 4)
        return h
    m.seam(0x5b54d0, getter('string'))
    m.seam(0x5b5620, getter('localized'))
    m.seam(0x5b5380, getter('bool'))
    m.seam(0x5b5130, getter('fixed'))

    def child(m):  # 5B4410 thiscall(node; name, required) ret 8
        name = name_arg(m)
        m.record('child', field=name)
        m.ret(8, size if name == 'size' and case.get('size') is not None else 0)
    m.seam(0x5b4410, child)

    def enum(m):  # 497220 thiscall(map; &CString) ret 4 -> &&value or 0
        text = m.read_wide(m.r(m.arg(0)))
        v = SHAPES.get(text.upper())
        m.record('enum', text=text)
        if v is None:
            m.ret(4, 0)
        else:
            cell = m.alloc(8, 0)
            m.w(cell, cell + 4)
            m.w(cell + 4, v)
            m.ret(4, cell)
    m.seam(0x497220, enum)
    m.leaf(0x4bb310, '4BB310', 4, 1, extra=lambda m, e, a: e.update(args=[m.label(a[0])]))
    m.seam(0x410c40, lambda m: m.throw('DataError', format=m.read_wide(m.arg(0))))

    def log(m):
        m.record('404ED0', line=m.arg(2), format=m.read_wide(m.arg(4)))
        m.ret(0)
    m.seam(0x404ed0, log)
    m.calls, m.outcome = [], None
    m.null_accesses.clear()
    m.guard = True
    # run 49DC40(this=def, node) to 49E097
    from native_machine import STACK
    m.w(STACK, 0x2f0fff0)
    m.w(STACK + 4, root)
    m.u.reg_write(UC_X86_REG_ESP, STACK)
    m.u.reg_write(UC_X86_REG_ECX, d)
    m.run(0x49dc40, 0x49e097)
    m.guard = False
    out = {'outcome': m.outcome, 'calls': m.calls, 'unexpected': m.unexpected}
    if not m.outcome:
        assert m.u.reg_read(UC_X86_REG_EIP) == 0x49e097
        out['words'] = {hex(o): m.r(d + o) for o in range(0x24, 0x84, 4)}
        out['byte_34'] = m.rb(d + 0x34)
        out['byte_80'] = m.rb(d + 0x80)
        out['description'] = m.read_wide(m.r(d + 0x30))
    return out


def cases():
    out = []
    shapes = [('CIRCLE', {'blocking_radius': f32(1.25)}), ('SQUARE', {'square_size': f32(3.0)}),
              ('RECT', {'rect_width': f32(4.0), 'rect_height': f32(2.5)}), ('POINT', {})]
    k = 0
    for pc in ('Single24', 'Double53', 'Extended64'):
        for bname, bfields in shapes:
            for soft in ('', 'CIRCLE'):
                for target in ('', 'RECT'):
                    for needs in (0, 1):
                        size = {'height': f32(1.5 + k * 0.125), 'scale_min': f32(0.8), 'scale_max': f32(1.2 + k * 0.01),
                                'blocking_shape': bname, 'soft_shape': soft, 'target_shape': target,
                                'soft_radius': f32(2.75), 'target_rect_width': f32(5.5), 'target_rect_height': f32(1.75),
                                **bfields}
                        out.append(('%s_%s_%s_%s_%d' % (pc, bname, soft or 'nosoft', target or 'notarget', needs),
                                    {'pc': pc, 'root': {'needs_space_for_settlement_zone': needs,
                                                         'description': 'desc%d' % k}, 'size': size}))
                        k += 1
    out.append(('no_size', {'pc': 'Double53', 'root': {'needs_space_for_settlement_zone': 1}, 'size': None}))
    out.append(('odd_rect', {'pc': 'Single24', 'root': {}, 'size': {
        'height': f32(0.1), 'scale_min': f32(1.0), 'scale_max': f32(1.0), 'blocking_shape': 'rect',
        'rect_width': f32(0.3), 'rect_height': f32(0.7), 'soft_shape': '', 'target_shape': ''}}))
    out.append(('scale_inverted', {'pc': 'Double53', 'root': {}, 'size': {
        'height': f32(1.0), 'scale_min': f32(2.0), 'scale_max': f32(1.0), 'blocking_shape': 'CIRCLE',
        'blocking_radius': f32(1.0), 'soft_shape': '', 'target_shape': ''}}))
    return out


def main():
    rows = []
    t = time.time()
    for label, c in cases():
        r = run(c)
        assert not r['unexpected'], (label, r)
        rows.append({'label': label, 'case': c, 'result': r})
    doc = {'image': 'AA.exe (executed windows identical in runtime-expanded-v14/AA.exe)',
           'harness_sha256': {n: H((HERE / n).read_bytes()) for n in ('actor_machine.py', 'native_object_definition.py')},
           'base_machine_sha256': H(Path(nm.__file__).read_bytes()), 'cases': rows,
           'doodad_component_update_time': 'research/phase7-definitions/native-doodads.json: component +10 = update_time '
                                           '(4C8040 / 4C7CC0 loaders, both slots)',
           'visited_instructions': len(VISITED)}
    (HERE / 'native-object-definition.json').write_text(json.dumps(doc, indent=1))
    print(len(rows), 'cases %.1fs' % (time.time() - t), [r['label'] for r in rows if r['result']['outcome']])


if __name__ == '__main__':
    main()
