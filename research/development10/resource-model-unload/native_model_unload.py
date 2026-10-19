"""Original/v14 5AE0A0 -> 5ACA50 over the actual Resource class vtables, plus the
5ABFF0 KKC_ResourceManager::UnloadResource body.

Executed native code: 5AE0A0, 96CB83, 5ACA50, the 4245A0 hash iterator, every
class vf4 getter reached through the ACTUAL constructor vtables (Texture
A454A8/5A60E0, Model A45C98/5A7DB0, Sequence A45290/5A5FB0, Sound A42C38/
59B400, Map A44008, Film A44130), the descriptor-parent walk, and (stage
'unload') the 5ABFF0 body.  Descriptor+8 parents are produced by executing the
real 409750 slice 40977F..4097A8 with the immediates decoded from each CRT
registration function (9C7FE0, 9D0340, 9D0370, 9D0680, 9D06B0, 9D06E0,
9D0710, 9D02F0); the CRT bytes are pinned.

Named seams: 405B80 CString assignment (transport), and
  stage 'select': 5ABFF0 is a RECORDING stop (manager, resource) + ret 4, so
                  every eligible entry of one traversal is listed in native
                  hash order; it is NOT a successful unload.
  stage 'unload': 5ABFF0 executes; its children 857E10/857DE0 (manager lock
                  enter/leave), Model vf40 5A6560 / vf54 5A7CF0, the 404F90
                  log and the 59BFA0 cache removal are recording stops. The
                  flags+8 store at 5AC014 executes natively.
A read guard proves non-NiAVObj entries' +4/+8 are never read by 5ACA50.
"""
from pathlib import Path
import hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402
from unicorn import UC_HOOK_MEM_READ  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ESI, UC_X86_REG_ECX  # noqa: E402
from capstone import Cs, CS_ARCH_X86, CS_MODE_32  # noqa: E402

CS = Cs(CS_ARCH_X86, CS_MODE_32)
# Actual constructor vtable stores (5A6109, 5A7DE3, 5A5FFB, 59B44A) and the
# existing Map/Film vtables; each is checked against its constructor bytes.
CONSTRUCTORS = {'Texture': (0x5a6109, 0xa454a8), 'Model': (0x5a7de3, 0xa45c98),
                'Sequence': (0x5a5ffb, 0xa45290), 'Sound': (0x59b44a, 0xa42c38)}
VT = {'Texture': 0xa454a8, 'Model': 0xa45c98, 'Sequence': 0xa45290, 'Sound': 0xa42c38,
      'Map': 0xa44008, 'Film': 0xa44130}
CRT = [0x9c7fe0, 0x9d0340, 0x9d0370, 0x9d0680, 0x9d06b0, 0x9d06e0, 0x9d0710, 0x9d02f0]
NI_AVOBJ = 0xb3d990


class Required(Exception):
    pass


def crt_registration(address):
    """push factory; push manager; push parent; push name; mov ecx,descriptor; call 409750."""
    ins = list(CS.disasm(h.P.range(address, 0x2a), address))[:6]
    assert [i.mnemonic for i in ins] == ['push'] * 4 + ['mov', 'call'], hex(address)
    assert ins[4].op_str.startswith('ecx, ') and ins[5].op_str == '0x409750', hex(address)
    factory, manager, parent, name = (int(i.op_str, 16) for i in ins[:4])
    descriptor = int(ins[4].op_str.split(', ')[1], 16)
    assert manager == 0xb38e08
    return dict(crt=address, descriptor=descriptor, parent=parent, name=name, factory=factory)


REGISTRATIONS = [crt_registration(a) for a in CRT]


def wide(image, address):
    out = []
    while True:
        v = struct.unpack('<H', image.range(address + 2 * len(out), 2))[0]
        if v == 0:
            return ''.join(map(chr, out))
        out.append(v)


class Machine(h.Machine):
    def __init__(self, pc, image):
        super().__init__(pc)
        if image is not h.P:
            original_end = (h.P.image_size + 4095) & ~4095
            image_end = (image.image_size + 4095) & ~4095
            if image_end > original_end:
                self.u.mem_map(image.base + original_end, image_end - original_end)
            self.u.mem_write(image.base, image.b[:4096])
            for vs, r, raw, off in image.sections:
                if raw:
                    self.u.mem_write(image.base + r, image.b[off:off + min(raw, max(vs, raw))])
        self.image = image
        self.events = []
        self.guards = {}
        self.labels = {}
        self.running = False
        self.u.hook_add(UC_HOOK_MEM_READ, self.read_guard)
        for r in REGISTRATIONS:
            self.w32(h.STACK + 0x28, r['parent'])
            self.w32(h.STACK + 0x2c, 0xb38e08)
            self.w32(h.STACK + 0x30, r['factory'])
            self.u.reg_write(UC_X86_REG_ESP, h.STACK)
            self.u.reg_write(UC_X86_REG_ESI, r['descriptor'])
            self.u.emu_start(0x40977f, 0x4097a8, count=100)
            expected = 0 if r['descriptor'] == r['parent'] else r['parent']
            assert self.r32(r['descriptor'] + 8) == expected
            assert self.r32(r['descriptor'] + 0x10) == r['factory']
        self.seams[0x405b80] = self.assign

    def chain(self, descriptor):
        out = []
        while descriptor:
            out.append(descriptor)
            descriptor = self.r32(descriptor + 8)
        return out

    def text(self, units):
        data = self.alloc(16 + 2 * (len(units) + 1))
        self.w32(data, 0, len(units), len(units), 1)
        self.u.mem_write(data + 16, struct.pack('<' + 'H' * (len(units) + 1), *units, 0))
        return data + 16

    def units(self, p):
        out = []
        while True:
            v = struct.unpack('<H', self.u.mem_read(p + 2 * len(out), 2))[0]
            if v == 0:
                return out
            out.append(v)

    def assign(self, _):
        source = self.arg()
        dest = self.ecx()
        self.w32(dest, self.r32(source))
        self.events.append(['assign_counted_string'])
        self.ret(4, dest)

    def read_guard(self, u, access, a, size, value, _):
        if not self.running:
            return
        for p, kind in self.guards.items():
            if kind != 'Model' and p + 4 <= a < p + 12:
                raise AssertionError(f'non-NiAVObj {kind} count/flags were read')

    # ---- stage seams --------------------------------------------------
    def select_stop(self, _):
        manager = self.ecx()
        resource = self.arg()
        assert manager == self.manager
        self.events.append(['5ABFF0', self.labels[resource]])
        self.ret(4)

    def install_unload(self):
        def lock(name):
            def seam(_):
                offset = self.ecx() - self.manager
                self.events.append([name, hex(offset)])
                self.ret()
            return seam
        self.seams[0x857e10] = lock('lock_857e10')
        self.seams[0x857de0] = lock('unlock_857de0')

        def virtual(name):
            def seam(_):
                p = self.ecx()
                self.events.append([name, self.labels[p], self.r32(p + 8)])
                self.ret()
            return seam
        self.seams[0x5a6560] = virtual('vf40_5A6560')
        self.seams[0x5a7cf0] = virtual('vf54_5A7CF0')

        def log(_):
            channel, file, line, function, fmt, name = self.r32(self.esp() + 4, 6)
            self.events.append(['log_404f90', hex(channel), wide(self.image, file), line,
                                wide(self.image, function), wide(self.image, fmt), self.units(name)])
            self.ret()
        self.seams[0x404f90] = log

        def cache_remove(_):
            offset = self.ecx() - self.manager
            p = self.arg()
            self.events.append(['cache_remove_59bfa0', hex(offset), self.labels[p], self.r32(p + 8)])
            self.ret(4, 1)
        self.seams[0x59bfa0] = cache_remove

    def run(self, case):
        manager = self.manager = self.alloc(0x130)
        arg = self.alloc(4)
        self.w32(manager + 0x80, self.text(case['current']))
        self.w32(arg, self.text(case['next']))
        buckets_count = case.get('buckets', 5)
        buckets = self.alloc(buckets_count * 4)
        self.w32(manager + 0x60, buckets, buckets_count, len(case['resources']))
        # Native nodes {value,next}; insertion at chain head as 5AC080/5ACF60.
        for index, item in enumerate(case['resources']):
            p = self.alloc(0xb4)
            self.w32(p, VT[item['class']], item['count'], item['flags'])
            name = item.get('name', f"{item['class']}{index}")
            self.w32(p + 0x38, self.text(list(map(ord, name))))
            self.guards[p] = item['class']
            self.labels[p] = index
            bucket = item['bucket'] % buckets_count
            node = self.alloc(8)
            self.w32(node, p, self.r32(buckets + bucket * 4))
            self.w32(buckets + bucket * 4, node)
        if case['stage'] == 'select':
            self.seams[0x5abff0] = self.select_stop
        else:
            self.install_unload()
        classified = []

        def classify(_):
            p = self.ecx()
            classified.append(self.labels[p])
        self.seams[0x5aca8b] = classify  # call [eax+4]: class getter dispatch
        self.running = True
        self.call(0x5ae0a0, manager, [arg])
        self.running = False
        flags = [self.r32(p + 8) for p in self.guards]
        counts = [self.r32(p + 4) for p in self.guards]
        return dict(current=self.units(self.r32(manager + 0x80)), events=self.events,
                    classified=classified, flags=flags, counts=counts)


def cases():
    def r(kind, flags, count, bucket, name=None):
        out = {'class': kind, 'flags': flags & 0xffffffff, 'count': count & 0xffffffff, 'bucket': bucket}
        if name:
            out['name'] = name
        return out
    tropical = list(map(ord, 'Tropical'))
    # 1. The exact Model/Texture/Sequence/Sound flag/count states the current
    #    Rust port can construct before 43F0F1 (constructor 0x410, attach
    #    clears 0x16, Aspect/Data/VFX/load-kernel option bits, loaded 1|4|6,
    #    requested counts 0..2). None has bit5.
    port_states = [0x410, 0x400, 0x8400, 0x80400, 0x1d00, 0x100400, 0x407, 0x4407, 0x804607, 0x20407, 0x447, 0x7]
    for i, flags in enumerate(port_states):
        for count in (0, 1, 2):
            yield dict(stage='select', current=[], next=tropical, resources=[
                r('Map', 0, 0, 0), r('Model', flags, count, i), r('Texture', 0x200000, count, 1),
                r('Sequence', 0x0, count, 2), r('Sound', 0x0, count, 3), r('Film', 0, 0, 4)])
    # 2. Predicate edges on Model (bit5 = 0x20 tested via shr 5/test 1; signed jg).
    for flags in (0x20, 0x21, 0x427, 0x0, 0xffffffdf, 0xffffffff, 0x1f):
        for count in (0, 1, -1, 0x7fffffff, -0x80000000):
            yield dict(stage='select', current=[], next=tropical,
                       resources=[r('Model', flags, count, 0)])
    # 3. Same cached/unrequested states on every other class: skipped by class.
    for kind in ('Texture', 'Sequence', 'Sound', 'Map', 'Film'):
        for flags, count in ((0x21, 0), (0x20, -1), (0xffffffff, 0x80000000)):
            yield dict(stage='select', current=[], next=tropical,
                       resources=[r(kind, flags, count, 0), r('Model', 0x410, 0, 1)])
    # 4. Several eligible entries over nontrivial buckets/chains: native order.
    yield dict(stage='select', current=[65], next=tropical, buckets=7, resources=[
        r('Model', 0x21, 0, 3), r('Model', 0x21, 1, 3), r('Texture', 0x21, 0, 0), r('Model', 0x20, -1, 0),
        r('Model', 0x421, 0, 6), r('Sequence', 0x21, 0, 6), r('Model', 0x21, 0, 3), r('Map', 0, 0, 2)])
    # 5. Equal C-string prefix: no assignment, no traversal at all.
    yield dict(stage='select', current=tropical, next=tropical + [0, 88], resources=[r('Model', 0x21, 0, 0)])
    # 6. The 5ABFF0 body for cached unrequested NiAVObj, loaded and not loaded.
    for flags in (0x21, 0x20, 0x8427, 0x420):
        yield dict(stage='unload', current=[], next=tropical,
                   resources=[r('Model', flags, 0, 0, 'Units/Infantry/inf.nif'), r('Texture', 0x21, 0, 1)])


def main():
    rows = []
    for case in cases():
        for pc in h.PRECISION:
            outputs = [Machine(pc, image).run(case) for image in (h.P, h.V)]
            assert outputs[0] == outputs[1], case
            rows.append(dict(input=case, precision=pc, output=outputs[0]))
    m = Machine('Double53', h.P)
    classes = {}
    for kind, vt in VT.items():
        getter = m.r32(vt + 4)
        ins = list(CS.disasm(h.P.range(getter, 6), getter))
        assert ins[0].mnemonic == 'mov' and ins[0].op_str.startswith('eax, ') and ins[1].mnemonic == 'ret'
        descriptor = int(ins[0].op_str.split(', ')[1], 16)
        chain = m.chain(descriptor)
        classes[kind] = dict(vtable=hex(vt), vf4=hex(getter), descriptor=hex(descriptor),
                             chain=[hex(v) for v in chain], ni_avobj=NI_AVOBJ in chain,
                             vf40=hex(m.r32(vt + 0x40)), vf54=hex(m.r32(vt + 0x54)))
    for kind, (store, vt) in CONSTRUCTORS.items():
        ins = next(CS.disasm(h.P.range(store, 6), store))
        assert ins.mnemonic == 'mov' and ins.op_str == f'dword ptr [esi], {hex(vt)}', (kind, ins.op_str)
    static = []
    for address, size in [(a, 0x2a) for a in CRT] + [(0x409750, 0x60), (0x5ae0a0, 0x3a), (0x5aca50, 0x9b),
                                                      (0x5abff0, 0x87), (0x5a7eb0, 6), (0x5a6180, 6), (0x5a5610, 6), (0x59af90, 6)]:
        raw = h.P.range(address, size)
        assert raw == h.V.range(address, size)
        static.append(dict(address=hex(address), size=size, sha256=hashlib.sha256(raw).hexdigest()))
    seams = ['0x405b80', '0x5abff0 (select stage only)', '0x857e10', '0x857de0', '0x5a6560', '0x5a7cf0', '0x404f90', '0x59bfa0']
    executed = [dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())]
    out = dict(schema=1, scope=__doc__, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               registrations=[{k: hex(v) for k, v in r.items()} for r in REGISTRATIONS], classes=classes,
               cases=rows, static_pins=static, seams=seams, executed_address_byte_pins=executed)
    raw = (json.dumps(out, indent=1) + '\n').encode()
    path = HERE / 'native-model-unload.json'
    if '--write' in sys.argv:
        path.write_bytes(raw)
    else:
        assert path.read_bytes() == raw, 'fixture differs from fresh native replay'
    print(json.dumps(dict(rows=len(rows), machines=2 * len(rows), executed_pins=len(executed),
                          sha256=hashlib.sha256(raw).hexdigest())))


if __name__ == '__main__':
    main()
