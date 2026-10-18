"""Original4FAE80 post-construction dispatch, not complete record loading.

Executes4FB2D5 through the record-tail cleanup boundary4FB539. Real class
vtables select Load/Init/PostCreate. Those bodies and publication/queue/FX
helpers are explicit recorded leaves. Init=false stops at4FB582 before the
diagnostic/destruction path; no native SEH/destructor claim is made.
"""
from pathlib import Path
import sys, json, hashlib, struct
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
from native_machine import Machine, original, modified, STACK, instructions
from unicorn.x86_const import UC_X86_REG_EBP, UC_X86_REG_EBX, UC_X86_REG_EDI, UC_X86_REG_ESP, UC_X86_REG_EIP

sha = lambda b: hashlib.sha256(b).hexdigest()
CLASSES = {'Kingdom': 0xa299d8, 'Team': 0xa29780, 'Thing': 0xa2a680}

def run(case, image_name, precision):
    m = Machine(precision)
    image = original if image_name == 'original' else modified
    old = (original.image_size + 4095) & ~4095
    new = (image.image_size + 4095) & ~4095
    if new > old: m.u.mem_map(image.base + old, new - old)
    for _, rva, size, offset in image.sections:
        m.u.mem_write(image.base + rva, image.b[offset:offset + size])
    m.u.mem_unmap(0, 4096)
    obj, world, frame, slot, stream, definition = [m.alloc(n) for n in [0x400, 0x400, 0x200, 4, 0x40, 0x200]]
    frame += 0x100
    vt = CLASSES[case['class']]
    m.w(obj, vt); m.w(obj + 4, definition); m.w(obj + 8, case['initial_flags'])
    m.w(slot, obj); m.w(world + 0x2c, 23); m.w(0xb36534, world)
    m.w(frame - 0x20, world); m.w(frame + 0x10, 0x13579bdf)
    m.u.reg_write(UC_X86_REG_EBP, frame)
    m.u.reg_write(UC_X86_REG_EBX, stream)
    m.u.reg_write(UC_X86_REG_EDI, slot)
    m.u.reg_write(UC_X86_REG_ESP, STACK)
    events = []; terminal = ['cleanup_boundary']
    def target(off): return struct.unpack('<I', original.range(vt + off, 4))[0]
    def load(_):
        assert m.ecx() == obj and m.arg(1) == stream
        events.append(['Load', hex(target(0xd8))])
        m.w(obj + 8, case['loaded_flags']); m.ret(pop=4)
    def init(_):
        assert m.ecx() == obj and m.arg(1) == 0
        events.append(['Init', hex(target(0xe8)), 0])
        if case['init_sets_bit7']: m.w(obj + 8, m.r(obj + 8) | 128)
        m.ret(case['init_return'], pop=4)
    def post(_):
        assert m.ecx() == obj and m.arg(1) == 0 and m.arg(2) == 1
        events.append(['PostCreate', hex(target(0xec)), 0, 1]); m.ret(pop=8)
    def queue(_):
        assert m.ecx() == world + 0x110 and m.arg(1) == slot
        events.append(['deferred_queue', '0x542a00', m.r(obj + 8)])
        if case['queue_clears_bit7']: m.w(obj + 8, m.r(obj + 8) & ~128)
        m.ret(pop=4)
    def register(_):
        assert m.ecx() == world and m.arg(1) == obj
        events.append(['register', '0x4f9510']); m.ret(pop=4)
    def created(_):
        assert m.ecx() == definition + 0xbc and m.arg(1) == obj and m.arg(2) == 1
        events.append(['definition_created', '0x4296b0', 1]); m.ret(pop=8)
    def failed(_):
        terminal[0] = 'Init_false_diagnostic_and_destroy_branch'
        m.u.emu_stop()
    for address, callback in [(target(0xd8), load), (target(0xe8), init), (target(0xec), post),
                              (0x542a00, queue), (0x4f9510, register), (0x4296b0, created)]:
        m.seam(address, callback)
    m.observe(0x4fb582, failed)
    m.u.emu_start(0x4fb2d5, 0x4fb539, count=2000)
    assert m.u.reg_read(UC_X86_REG_EIP) in (0x4fb539, 0x4fb582)
    assert m.u.reg_read(UC_X86_REG_ESP) == STACK
    m.verify_v14()
    return {'events': events, 'terminal': terminal[0], 'thing_flags_at_boundary': m.r(obj + 8),
            'actor_138': m.r(obj + 0x138)}, m.visited

def main():
    rows = []; visited = {}
    base = dict(initial_flags=0, loaded_flags=0, init_return=1, init_sets_bit7=False, queue_clears_bit7=False)
    cases = [dict(base, name=kind+'_normal', **{'class': kind}) for kind in CLASSES]
    cases += [dict(base, name=name, **{'class':'Kingdom'}, **change) for name, change in [
        ('init_false', {'init_return':0}), ('init_AL_zero', {'init_return':256}),
        ('init_AL_nonzero', {'init_return':255}),
        ('loaded_bit7', {'loaded_flags':0x80000080}),
        ('Load_clears_bit7', {'initial_flags':128}),
        ('Init_sets_bit7', {'init_sets_bit7':True}),
        ('queue_clears_bit7_probe', {'loaded_flags':0x80000080, 'queue_clears_bit7':True})]]
    for case in cases:
        for precision in ['Single24','Double53','Extended64']:
            for image in ['original','v14']:
                result, code = run(case, image, precision); visited.update(code)
                rows.append({'input':case,'precision':precision,'image':image,'output':result})
    table = []
    for name, vt in CLASSES.items():
        values = {}
        for off in [0xb0,0xb4,0xd8,0xdc,0xe8,0xec,0xf0]:
            b = original.range(vt+off,4); assert b == modified.range(vt+off,4)
            values[hex(off)] = hex(struct.unpack('<I', b)[0])
        table.append({'class':name,'vtable':hex(vt),'entries':values})
    report = {'scope':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),
              'producer_sha256':sha(Path(__file__).read_bytes()),'cases':rows,'vtables':table,
              'instructions':instructions(visited),
              'limits':['After-construction record-tail dispatch only; virtual bodies and queue/publication/FX helpers are leaves.',
                        'World version23 bypasses the older-footprint compatibility block. No pre23 shape conversion claim.',
                        'Init=false branch stops before diagnostics/destruction. Full Load throw/unwind cleanup not executed.',
                        'Leaf-induced bit7 changes are explicit ordering probes, not claimed normal behavior of those leaves.',
                        '4F9510 is a recorded leaf, so final flags do not model its internal bit31 clearing or world membership effects.']}
    out = HERE/'native-loadthing-lifecycle.json'
    out.write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'cases':len(rows),'sha256':sha(out.read_bytes()),'instruction_sites':len(visited)}))

if __name__ == '__main__': main()
