"""Actual ObjectAnimation 49DA40->49C020->49D900 with POSITIVE 49D0A0 CreateVFX
and 49CF80 PlaySFX children (original and accepted v14, PC24/53/64).

Executed native bodies: 49BF20 Set ctor, 49C020 allocation (49B510/49B5A0
element ctors 79E130/49B4A0), 49D900 fields, 49D0A0 and 49CF80 loops with
their 877EA0/49B630 growth, 49DA40 overall/typed appends, 497220 enum lookup,
405D60 KF format, and 42C070's own empty-IDS early return.

Explicit recorded seams (never completed silently):
  5B4220 child count, 42BE10 child iterator {node,index,name,count},
  5B54D0/5B5380/5B4AE0 typed getters, 4763C0 source-relative path,
  5A80D0 Sequence factory, 416370 Audio Find (ECX must be [[B36530]+584]),
  4BDB70 VFX Thing Find (ECX must be [[B36530]+C]), nonempty 42C070 Bone
  Find (ECX must be [[B36530]+580]; result Data+C), 410C40 throw (formatted
  %s message recorded) and 855D30 assertion. The Audio manager +20 name is
  an explicit fixture CString; 483A40 is not executed here.
The DataManager registries/translations behind the three Finds are the
existing FX lookups (fx_graph Bound::find), not re-proved by this producer.
"""
from pathlib import Path
import sys, json, hashlib, gc, struct
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-definitions'))
import native_definition_registry as h
sys.path.insert(0, str(ROOT / 'research/skirmish-20261017-svar-field-order'))
import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP, UC_X86_REG_ESP, UC_X86_REG_ECX, UC_X86_REG_FPCW


class Required(Exception):
    pass


def signed(v):
    return v - (1 << 32) if v & 0x80000000 else v


def cases():
    base = dict(kf='%s%s.kf', prefix='unit_', folder='Art/Units', length=0x3fc00000, exclusive=0,
                start_min=0, start_max=0, resource=True, stop=None, type='IDLE',
                children={}, child_values={}, audio={}, audio_manager='Audio', vfx={}, bone={})

    def sfx(*rows):
        return [{'sfx_ids': ids, 'looping': looping} for ids, looping in rows]

    def vfx(*rows):
        return [{'vfx_ids': ids, 'location': location} for ids, location in rows]

    def case(ids, s=(), v=(), **kw):
        out = dict(base, ids=ids, **kw)
        values = {}
        if s:
            values['PlaySFX'] = sfx(*s)
        if v:
            values['CreateVFX'] = vfx(*v)
        out['child_values'] = values
        out['children'] = {k: len(rows) for k, rows in values.items()}
        out['children'].update(kw.get('children', {}))
        return out
    audio = {'snd_a': True, 'snd_b': True, 'snd_c': True}
    fx = {'fx_a': True, 'fx_b': True}
    bones = {'base': 0, 'turret': 3, 'neg': -5}
    yield case('sfx1', s=[('snd_a', 0)], audio=audio)
    yield case('sfx2', s=[('snd_a', 1), ('snd_b', 0)], audio=audio, type='RANDOM')
    yield case('sfx5', s=[('snd_a', 1), ('snd_b', 0), ('snd_c', 1), ('snd_a', 0), ('snd_b', 1)], audio=audio, type='SELECTED')
    yield case('sfx_empty', s=[('', 0)], audio=audio)
    yield case('sfx_unknown', s=[('missing', 1)], audio=audio)
    yield case('sfx_second_unknown', s=[('snd_a', 1), ('nope', 0)], audio=audio)
    yield case('sfx_stop_ids', s=[('snd_a', 1)], audio=audio, stop='read:PlaySFX[0]:sfx_ids')
    yield case('sfx_stop_looping', s=[('snd_a', 1), ('snd_b', 1)], audio=audio, stop='read:PlaySFX[1]:looping')
    yield case('sfx_lookup_stop', s=[('snd_a', 1)], audio=audio, stop='audio:snd_a')
    yield case('sfx_manager', s=[('ghost', 0)], audio=audio, audio_manager='Mgr')
    yield case('vfx1', v=[('fx_a', 'base')], vfx=fx, bone=bones)
    yield case('vfx2', v=[('fx_a', 'base'), ('fx_b', 'turret')], vfx=fx, bone=bones, type='FIDGET')
    yield case('vfx5', v=[('fx_a', 'base'), ('fx_b', 'turret'), ('fx_a', 'turret'), ('fx_b', 'base'), ('fx_a', 'base')], vfx=fx, bone=bones)
    yield case('vfx_unknown', v=[('nofx', 'base')], vfx=fx, bone=bones)
    yield case('vfx_empty', v=[('', 'base')], vfx=fx, bone=bones)
    yield case('vfx_location_unknown', v=[('fx_a', 'nobone')], vfx=fx, bone=bones)
    yield case('vfx_location_empty', v=[('fx_a', '')], vfx=fx, bone=bones)
    yield case('vfx_location_negative', v=[('fx_a', 'neg')], vfx=fx, bone=bones)
    yield case('vfx_stop_location', v=[('fx_a', 'base')], vfx=fx, bone=bones, stop='read:CreateVFX[0]:location')
    yield case('both', s=[('snd_a', 0), ('snd_b', 1)], v=[('fx_b', 'turret')], audio=audio, vfx=fx, bone=bones, type='CUSTOMPLAY')
    yield case('both_sfx_fail', s=[('snd_a', 0), ('none', 1)], v=[('fx_a', 'base')], audio=audio, vfx=fx, bone=bones)
    yield case('both_stop_count', s=[('snd_a', 0)], v=[('fx_a', 'base')], audio=audio, vfx=fx, bone=bones, stop='PlaySFX')
    yield case('nores', s=[('snd_a', 0)], v=[('fx_a', 'base')], audio=audio, vfx=fx, bone=bones, resource=False)
    yield case('after', s=[('snd_c', 1)], audio=audio, type='CONST')


def run(image, pc):
    full.order.T.P = image
    m = h.Machine()
    m.u.reg_write(UC_X86_REG_FPCW, full.order.PCS[pc])
    pins = {}
    seams = set()

    def capture(u, at, size, _):
        if image.base <= at < image.base + image.image_size:
            if u.reg_read(UC_X86_REG_EIP) == at:
                pins[at] = bytes(u.mem_read(at, size)).hex()
            else:
                seams.add(at)
    m.u.hook_add(UC_HOOK_CODE, capture)
    m.thiscall(0x4013e0, 0xb43b40, [])
    enum = []

    def registration(mm, u):
        s = u.reg_read(UC_X86_REG_ESP)
        enum.append({'name': mm.raw_wstr(mm.r(s + 8)), 'kind': mm.r(s + 4)})
        mm.ret(pop=12)
    m.stubs[0x4976b0] = registration
    m.thiscall(0x497f10, m.alloc(0x40), [])
    buckets = m.alloc(4)
    m.w(0xb3b138, buckets)
    m.w(0xb3b13c, 1)
    head = 0
    for e in reversed(enum):
        record = m.alloc(12)
        m.w(record, e['kind'])
        m.w(record + 4, m.cstring(e['name']))
        node = m.alloc(8)
        m.w(node, record)
        m.w(node + 4, head)
        head = node
    m.w(buckets, head)
    owner = m.alloc(0x6c)
    m.u.mem_write(owner, b'\xa5' * 0x6c)
    m.thiscall(0x49bf20, owner, [])
    # Actual GameData+C Thing, +580 Bone and +584 Audio manager slots.
    game = m.r(0xb36530)
    assert game
    managers = {'thing': m.alloc(0x40), 'bone': m.alloc(0x40), 'audio': m.alloc(0x40)}
    m.w(game + 0xc, managers['thing'])
    m.w(game + 0x580, managers['bone'])
    m.w(game + 0x584, managers['audio'])
    resource = m.alloc(0x40)
    m.w(resource + 0x1c, 0)
    inputs = {}
    trace = []
    state = {'node': None, 'children': {}, 'labels': {}}

    def esp_arg(u, i=1):
        return m.r(u.reg_read(UC_X86_REG_ESP) + 4 * i)

    def label(prefix, text):
        p = m.alloc(0x10)
        state['labels'][p] = text
        return p

    def child_of(ecx):
        return state['children'].get(ecx)

    def stop(entry):
        if inputs.get('stop') == entry:
            raise Required(entry)

    def string_get(mm, u):
        key = mm.raw_wstr(esp_arg(u))
        ecx = u.reg_read(UC_X86_REG_ECX)
        child = child_of(ecx)
        if child is None:
            assert ecx == state['node'], hex(ecx)
            entry = 'read:' + key
            trace.append(entry)
            stop(entry)
            value = inputs[key]
        else:
            name, index = child
            entry = 'read:%s[%d]:%s' % (name, index, key)
            trace.append(entry)
            stop(entry)
            value = inputs['child_values'][name][index][key]
        slot = mm.alloc(4)
        mm.w(slot, mm.cstring(value))
        mm.ret(slot, pop=4)
    m.stubs[0x5b54d0] = string_get

    def number_get(mm, u):
        key = mm.raw_wstr(esp_arg(u))
        assert u.reg_read(UC_X86_REG_ECX) == state['node']
        entry = 'read:' + key
        trace.append(entry)
        stop(entry)
        value = mm.alloc(4)
        mm.w(value, inputs[key])
        stub = mm.alloc(16)
        mm.u.mem_write(stub, b'\xd9\x05' + struct.pack('<I', value) + b'\xc2\x04\x00')
        u.reg_write(UC_X86_REG_EIP, stub)
    m.stubs[0x5b4ae0] = number_get

    def boolean_get(mm, u):
        key = mm.raw_wstr(esp_arg(u))
        ecx = u.reg_read(UC_X86_REG_ECX)
        child = child_of(ecx)
        if child is None:
            assert ecx == state['node'], hex(ecx)
            entry = 'read:' + key
            trace.append(entry)
            stop(entry)
            value = inputs[key]
        else:
            name, index = child
            entry = 'read:%s[%d]:%s' % (name, index, key)
            trace.append(entry)
            stop(entry)
            value = inputs['child_values'][name][index][key]
        mm.ret(value, pop=4)
    m.stubs[0x5b5380] = boolean_get

    def relative(mm, u):
        arg = esp_arg(u)
        name = mm.wstr(mm.r(arg))
        folder = inputs['folder']
        out = name.lstrip('/') if name.startswith('/') else folder + '/' + name
        trace.append('relative:' + out)
        mm.w(arg, mm.cstring(out))
        mm.ret(arg, pop=4)
    m.stubs[0x4763c0] = relative

    def factory(mm, u):
        name = mm.wstr(mm.r(esp_arg(u)))
        trace.append('sequence:' + name)
        stop('sequence')
        mm.ret(resource if inputs['resource'] else 0)
    m.stubs[0x5a80d0] = factory

    def assertion(mm, u):
        trace.append('assert')
        raise Required('assert')
    m.stubs[0x855d30] = assertion

    def thrown(mm, u):
        fmt = mm.raw_wstr(esp_arg(u))
        args = []
        at = 2
        out = ''
        i = 0
        while i < len(fmt):
            if fmt[i] == '%' and fmt[i + 1:i + 2] == 's':
                out += mm.raw_wstr(esp_arg(u, at))
                at += 1
                i += 2
            else:
                out += fmt[i]
                i += 1
        trace.append('throw')
        raise Required(out)
    m.stubs[0x410c40] = thrown

    def count(mm, u):
        key = mm.raw_wstr(esp_arg(u))
        assert u.reg_read(UC_X86_REG_ECX) == state['node']
        trace.append('children:' + key)
        stop(key)
        mm.ret(inputs['children'].get(key, 0), pop=4)
    m.stubs[0x5b4220] = count

    def iterator(mm, u):
        it = u.reg_read(UC_X86_REG_ECX)
        node, index, name, total = mm.r(it), mm.r(it + 4), mm.raw_wstr(mm.r(it + 8)), mm.r(it + 12)
        assert node == state['node'] and total == inputs['children'][name], (hex(node), total)
        trace.append('child:%s:%d' % (name, index))
        handle = mm.alloc(0x10)
        state['children'][handle] = (name, index)
        mm.ret(handle)
    m.stubs[0x42be10] = iterator

    def audio_find(mm, u):
        assert u.reg_read(UC_X86_REG_ECX) == managers['audio']
        ids = mm.raw_wstr(esp_arg(u))
        entry = 'audio:' + ids
        trace.append(entry)
        stop(entry)
        mm.ret(label('audio', ids) if inputs['audio'].get(ids) else 0, pop=4)
    m.stubs[0x416370] = audio_find

    def vfx_find(mm, u):
        assert u.reg_read(UC_X86_REG_ECX) == managers['thing']
        ids = mm.raw_wstr(esp_arg(u))
        trace.append('vfx:' + ids)
        mm.ret(label('vfx', ids) if inputs['vfx'].get(ids) else 0, pop=4)
    m.stubs[0x4bdb70] = vfx_find

    def bone_find(mm, u):
        assert u.reg_read(UC_X86_REG_ECX) == managers['bone']
        ids = mm.raw_wstr(esp_arg(u))
        trace.append('bone:' + ids)
        if ids == '':
            # Execute actual 42C070 entry: empty IDS returns null before any
            # manager/translation/registry access.
            return
        value = inputs['bone'].get(ids)
        if value is None:
            mm.ret(0, pop=4)
            return
        data = mm.alloc(0x10)
        mm.w(data + 0xc, value & 0xffffffff)
        mm.ret(data, pop=4)
    m.stubs[0x42c070] = bone_find

    def vector(p, offset):
        return m.r(p + offset), m.r(p + offset + 4), m.r(p + offset + 8)

    def fields(p):
        if p is None:
            return None
        vdata, vcount, _ = vector(p, 0x20)
        sdata, scount, _ = vector(p, 0x2c)
        name = lambda q: state['labels'][q] if q else None
        vfx = [{'vfx': name(m.r(vdata + 8 * i)), 'location': signed(m.r(vdata + 8 * i + 4))} for i in range(vcount)]
        sfx = [{'audio': name(m.r(sdata + 8 * i)), 'looping': bytes(m.u.mem_read(sdata + 8 * i + 4, 1))[0]} for i in range(scount)]
        return dict(ordinal=m.r(p), ids=m.wstr(m.r(p + 4)), kind=signed(m.r(p + 8)), length=m.r(p + 12),
                    exclusive=bytes(m.u.mem_read(p + 24, 1))[0], start_min=m.r(p + 16), start_max=m.r(p + 20),
                    same_resource=m.r(p + 28) == resource, vfx=vfx, sfx=sfx)
    loaded = []
    for inputs in cases():
        trace = []
        state['children'] = {}
        before = m.r(owner + 4)
        arg = m.alloc(4)
        m.w(arg, m.cstring(inputs['prefix']))
        node = m.alloc(4)
        state['node'] = node
        m.w(managers['audio'] + 0x20, m.cstring(inputs['audio_manager']))
        status = 'ok'
        pending = None

        def pending_(u, at, size, _):
            nonlocal pending
            if at == 0x49c020:
                pending = u.reg_read(UC_X86_REG_ECX)
        hook = m.u.hook_add(UC_HOOK_CODE, pending_)
        try:
            m.thiscall(0x49da40, owner, [node, arg])
        except Required as e:
            status = 'required:' + str(e)
        m.u.hook_del(hook)
        after = m.r(owner + 4)
        last = m.r(m.r(owner) + 4 * (after - 1)) if status == 'ok' and after > before else None
        assert status != 'ok' or pending is None or last == pending
        loaded.append({'input': inputs, 'status': status, 'trace': trace, 'before': before, 'count': after,
                       'pending': fields(pending),
                       'typed_counts': [m.r(owner + 0x10 + 12 * i) for i in range(8)]})
    return {'enum': enum, 'loads': loaded}, pins, seams


def main():
    rows = []
    pins = {}
    seams = set()
    for pc in full.order.PCS:
        a, pa, sa = run(h.P, pc)
        gc.collect()
        b, pb, sb = run(h.V, pc)
        gc.collect()
        assert a == b and pa == pb
        rows.append({'precision': pc, 'output': a})
        pins.update(pa)
        seams |= sa | sb
    for a, b in pins.items():
        assert bytes.fromhex(b) == h.P.range(a, len(bytes.fromhex(b))) == h.V.range(a, len(bytes.fromhex(b)))
    for required in (0x49cf80, 0x49d0a0, 0x49b630, 0x877ea0, 0x49d900, 0x49da40, 0x49c020, 0x42c08a):
        assert required in pins, hex(required)
    doc = {'images': {'original': hashlib.sha256((ROOT.parent / 'AA.exe').read_bytes()).hexdigest(),
                      'v14': hashlib.sha256((ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes()).hexdigest()},
           'scope': __doc__, 'rows': rows,
           'executed_pins': {hex(a): b for a, b in sorted(pins.items())},
           'intercepted_not_executed': [hex(a) for a in sorted(seams)],
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    path = HERE / 'native-object-animation-children.json'
    if '--write' in sys.argv:
        path.write_text(json.dumps(doc, indent=1) + '\n', encoding='utf-8', newline='\n')
    else:
        assert json.loads(path.read_text()) == doc
    print(json.dumps({'rows': len(rows), 'machines': 2 * len(rows),
                      'loads': 2 * sum(len(r['output']['loads']) for r in rows), 'genuine_pins': len(pins)}))


if __name__ == '__main__':
    main()
