"""World+130 broadcast array: every producer, the 50C750/50C600 walk and slot reuse.

Runs the UNCHANGED original instructions (research/phase8-movement/harness.py:
every executed in-image byte is compared with the accepted v14 image; page-0
null guard; the engine allocator is a named seam) of:

  50CE00  manager constructor (4 slots built by 50CA40 through the CRT
          vector constructor iterator).
  50D120  type-1 script text (one-time WaitWorld under the +1C byte).
  50D0B0  type-0 thing record (World+D8 != 6, [[B39940]+88] byte, actor+C
          definition +54 bit 1, 4CF1B0 = actor+14).
  50D230  type-4 attached VFX (World+D8 != 6, definition and thing non-null;
          +24 definition, +28 thing+14, +44 World+E0).
  50D280  type-4 detached VFX (World+D8 != 6, definition non-null; +24,
          +2C..+34 position, +38/+3C direction, +40 = 0, +44 World+E0; +28 is
          NOT written).
  50D330  type 3 (no payload).  50D370  type 2 (no payload).
  50D070/50CF30  slot append and growth (50CA40 builds new slots: +4..+44
          zero except +20 = 0xA, +48 empty CString; old slots are copied
          word for word, so a reused slot keeps its stale words).
  50C750/50C600  the walk: index over the reread count, then count = 0.

Seams (recorded): 6F7810/6F7430 allocator (allocations are filled with 0xCD,
so a slot +0 never written by a producer reads 0xCDCDCDCD and is reported as
null); 856370 string manager and its vt+C empty-string allocation; 401B00
CString literal; 405B80 CString assignment; 4047A0 CString release (4010E0 via
50CAA0); 432E40 (handle word -> registered thing, the word read is recorded);
43B270, 427950, 440190, 4408A0, 43CD00, 43CC50 (return a fresh VFX object or
null as the case says; result+50 is read back).

Output per case: the walk traces, then count (+14), capacity (+18), +1C and
every slot up to the capacity (+0 kind, +4..+44 words, +48 text).
Usage: python native_world_broadcasts.py [--write]
"""
from pathlib import Path
import hashlib, itertools, json, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402

PCS = list(h.PRECISION)
STRING_VT_C = 0x2e7f000  # stub address inside the mapped harness region


def fb(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def run(c):
    m = h.Machine(c['precision'])
    trace = []

    def alloc(mm):
        n = mm.arg(0)
        p = m.alloc(n)
        m.u.mem_write(p, b'\xcd' * n)
        trace.append(['6F7810', n])
        mm.ret(0, p)

    def free(mm):
        trace.append(['6F7430'])
        mm.ret(0)

    def string(text):
        data = text.encode('utf-16le')
        base = m.alloc(16 + len(data) + 2)
        m.w32(base, 0x1234, len(data) // 2, len(data) // 2, 1)
        m.u.mem_write(base + 16, data + b'\0\0')
        return base + 16

    def text_of(ptr):
        return bytes(m.u.mem_read(ptr, m.r32(ptr - 12) * 2)).decode('utf-16le')

    manager_obj = m.alloc(0x20)

    def string_manager(mm):
        assert mm.arg(0) == 2
        mm.ret(0, manager_obj)

    def empty_string(mm):
        assert mm.ecx() == manager_obj
        mm.ret(0, string('') - 16)

    def literal(mm):
        raw = bytearray()
        p = mm.arg()
        while True:
            pair = bytes(mm.u.mem_read(p, 2))
            p += 2
            if pair == b'\0\0':
                break
            raw.extend(pair)
        target = mm.ecx()
        mm.w32(target, string(raw.decode('utf-16le')))
        mm.ret(4, target)

    def assign(mm):
        target = mm.ecx()
        value = mm.r32(mm.arg())
        mm.w32(target, value)
        mm.w32(value - 4, mm.r32(value - 4) + 1)
        mm.ret(4, target)

    def release(mm):
        mm.ret(0)

    vtable = m.alloc(0x20)
    m.w32(manager_obj, vtable)
    m.w32(vtable + 0xc, STRING_VT_C)
    m.u.mem_write(STRING_VT_C, b'\xc3')

    world, uvars, show, vworld, script = m.alloc(0x200), m.alloc(0x200), m.alloc(4), m.alloc(0x300), m.alloc(0x40)
    m.w32(0xb36534, world)
    m.w32(0xb39940, uvars)
    m.w32(uvars + 0x88, show)
    m.w32(0xb36538, vworld)
    m.w32(0xb36500, script)
    manager = m.alloc(0x20)

    things = {}  # handle word -> thing object
    labels = {}

    def lookup(mm):
        word = m.r32(mm.ecx())
        trace.append(['432E40', word])
        mm.ret(0, things.get(word, 0))

    def label(v):
        return labels.get(v, v)

    def record_view(mm):
        trace.append(['43B270', mm.ecx() == vworld, label(mm.arg(0)), m.r32(mm.arg(1), 7)])
        mm.ret(8)

    def script_eval(mm):
        assert mm.ecx() == script
        trace.append(['427950', text_of(m.r32(mm.arg(0))), mm.arg(1)])
        mm.ret(8)

    def plain(name):
        def seam(mm):
            trace.append([name, mm.ecx() == vworld])
            mm.ret(0)
        return seam

    vfx_objects = []
    results = iter(c['vfx_results'])

    def vfx_result():
        if next(results):
            v = m.alloc(0x60)
            m.w32(v + 0x50, 0x77777777)
            vfx_objects.append(v)
            return v
        return 0

    def attached(mm):
        trace.append(['43CD00', mm.ecx() == vworld, mm.arg(0), mm.arg(1), mm.arg(2), mm.arg(3), mm.arg(4), mm.arg(5)])
        mm.ret(24, vfx_result())

    def detached(mm):
        trace.append(['43CC50', mm.ecx() == vworld, mm.arg(0), m.r32(mm.arg(1), 3), m.r32(mm.arg(2), 3), mm.arg(3), mm.arg(4)])
        mm.ret(20, vfx_result())

    m.seams.update({0x6f7810: alloc, 0x6f7430: free, 0x856370: string_manager, STRING_VT_C: empty_string,
                    0x401b00: literal, 0x405b80: assign, 0x4047a0: release, 0x432e40: lookup,
                    0x43b270: record_view, 0x427950: script_eval, 0x440190: plain('440190'),
                    0x4408a0: plain('4408A0'), 0x43cd00: attached, 0x43cc50: detached})

    m.call(0x50ce00, manager, [])
    walks = []
    for op in c['ops']:
        kind = op[0]
        if kind == 'world':
            m.w32(world + 0xd8, op[1])
            m.w32(world + 0xe0, op[2])
        elif kind == 'show':
            m.w8(show, op[1])
        elif kind == 'view':
            m.w32(manager + 4, vworld if op[1] else 0)
        elif kind == 'thing':  # register handle word op[1] (op[2]: its +C word)
            t = m.alloc(0x20)
            m.w32(t + 0xc, op[2])
            things[op[1]] = t
            labels[t] = f'thing{op[1]:#x}'
        elif kind == 'forget':
            things.pop(op[1], None)
        elif kind == 'text':
            word = m.alloc(4)
            m.w32(word, string(op[1]))
            m.call(0x50d120, manager, [word])
        elif kind == 'damage':  # actor: definition flags (None = null definition), handle, record
            actor = m.alloc(0x20)
            if op[1] is not None:
                definition = m.alloc(0x60)
                m.w32(definition + 0x54, op[1])
                m.w32(actor + 0xc, definition)
            m.w32(actor + 0x14, op[2])
            record = m.alloc(0x1c)
            m.w32(record, *op[3])
            m.call(0x50d0b0, manager, [actor, record])
        elif kind == 'attached':  # definition word, thing handle (None = null thing)
            thing = 0
            if op[2] is not None:
                thing = m.alloc(0x20)
                m.w32(thing + 0x14, op[2])
            m.call(0x50d230, manager, [op[1], thing])
        elif kind == 'detached':  # definition word, position[3], direction[2]
            pos, dirn = m.alloc(12), m.alloc(12)
            m.w32(pos, *op[2])
            m.w32(dirn, *op[3], 0x99999999)
            m.call(0x50d280, manager, [op[1], pos, dirn])
        elif kind == 'kind2':
            m.call(0x50d370, manager, [])
        elif kind == 'kind3':
            m.call(0x50d330, manager, [])
        elif kind == 'walk':
            trace.clear()
            m.call(0x50c750, manager, [])
            walks.append(dict(trace=list(trace), vfx_50=[m.r32(v + 0x50) for v in vfx_objects]))
            vfx_objects.clear()
        else:
            raise ValueError(op)
    base, count, capacity = m.r32(manager + 0x10, 3)
    slots = []
    for i in range(capacity):
        p = base + i * 0x4c
        k = m.r32(p)
        slots.append(dict(kind=None if k == 0xcdcdcdcd else k, words=m.r32(p + 4, 17), text=text_of(m.r32(p + 0x48))))
    return dict(walks=walks, count=count, capacity=capacity, enabled=m.r8(manager + 0x1c), slots=slots)


R1 = [1, 2, 3, fb(4.5), fb(0.25), 6, 7]
R2 = [0x11, 0x22, 0x33, fb(-1.0), fb(9.0), 0x66, 0x77]
P1, D1 = [fb(10.0), fb(20.0), fb(30.0)], [fb(0.5), fb(-0.5)]
P2, D2 = [fb(1.5), fb(2.5), fb(3.5)], [fb(1.0), fb(0.0)]
ON = ['world', 0, fb(12.5)]
SEQUENCES = [
    ('constructor', [], []),
    ('texts_reuse', [ON, ['text', 'a'], ['walk'], ['text', 'b'], ['walk']], []),
    ('wait_world_and_trigger_list', [ON, ['text', 'TriggerListUpdate'], ['walk']], []),
    ('attached_and_detached', [ON, ['thing', 0x10005, 0xc001], ['attached', 0x2400, 0x10005],
                               ['detached', 0x2500, P1, D1], ['walk']], [True, True]),
    ('stale_target_reattaches', [ON, ['thing', 0x10005, 0xc001], ['attached', 0x2400, 0x10005], ['walk'],
                                 ['detached', 0x2500, P1, D1], ['walk']], [True, True]),
    ('stale_target_gone_drops', [ON, ['thing', 0x10005, 0xc001], ['attached', 0x2400, 0x10005], ['walk'],
                                 ['forget', 0x10005], ['detached', 0x2500, P1, D1], ['walk']], [True]),
    ('attached_thing_gone', [ON, ['attached', 0x2400, 0x20007], ['walk']], []),
    ('null_results', [ON, ['thing', 0x10005, 0xc001], ['attached', 0x2400, 0x10005],
                      ['detached', 0x2500, P2, D2], ['walk']], [False, False]),
    ('gates', [['world', 6, fb(1.0)], ['attached', 0x2400, 0x10005], ['detached', 0x2500, P1, D1],
               ['show', 1], ['damage', 2, 0x10009, R1],
               ['world', 0, fb(2.0)], ['attached', 0, 0x10005], ['attached', 0x2400, None],
               ['detached', 0, P1, D1], ['show', 0], ['damage', 2, 0x10009, R1],
               ['show', 1], ['damage', None, 0x10009, R1], ['damage', 0xfffffffd, 0x10009, R1],
               ['kind2'], ['walk']], []),
    ('damage_view', [ON, ['show', 1], ['view', 1], ['thing', 0x10009, 0xc009], ['damage', 2, 0x10009, R1],
                     ['damage', 3, 0x1000a, R2], ['walk']], []),
    ('damage_no_view', [ON, ['show', 1], ['view', 0], ['thing', 0x10009, 0xc009], ['damage', 2, 0x10009, R1],
                        ['walk']], []),
    ('mission_and_victory', [ON, ['kind2'], ['kind3'], ['text', 'after'], ['walk']], []),
    ('growth_copies_stale', [ON, ['thing', 0x10001, 0xc101], ['thing', 0x10002, 0xc102],
                             ['attached', 0x2401, 0x10001], ['attached', 0x2402, 0x10002],
                             ['attached', 0x2403, 0x10003], ['detached', 0x2404, P2, D2], ['walk'],
                             ['text', 't1'], ['text', 't2'], ['text', 't3'], ['text', 't4'], ['text', 't5'],
                             ['walk'],
                             ['detached', 0x2501, P1, D1], ['detached', 0x2502, P1, D1], ['detached', 0x2503, P1, D1],
                             ['detached', 0x2504, P1, D1], ['detached', 0x2505, P1, D1], ['detached', 0x2506, P1, D1],
                             ['detached', 0x2507, P1, D1], ['detached', 0x2508, P1, D1], ['detached', 0x2509, P1, D1],
                             ['walk']], [True] * 12),
    ('mixed_order', [ON, ['show', 1], ['view', 1], ['thing', 0x10009, 0xc009], ['text', 'x'],
                     ['detached', 0x2500, P1, D1], ['kind3'], ['damage', 2, 0x10009, R2], ['kind2'],
                     ['text', 'y'], ['walk'], ['walk']], [True]),
]


def cases():
    rows = []
    for pc, (name, ops, results) in itertools.product(PCS, SEQUENCES):
        c = dict(precision=pc, name=name, ops=ops, vfx_results=results)
        rows.append(dict(input=c, output=run(c)))
    return rows


def main():
    rows = cases()
    out = dict(schema=1, scope=__doc__, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA, cases=rows,
               instructions=[dict(va=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())])
    path = HERE / 'native-world-broadcasts.json'
    text = json.dumps(out, indent=1) + '\n'
    if '--write' in sys.argv:
        path.write_text(text)
    else:
        assert json.loads(path.read_text()) == json.loads(text), 'native replay differs from the fixture'
    print(json.dumps(dict(cases=len(rows), instructions=len(h.VISITED),
                          sha256=hashlib.sha256(path.read_bytes()).hexdigest())))


if __name__ == '__main__':
    main()
