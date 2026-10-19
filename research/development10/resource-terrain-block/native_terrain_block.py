"""Original/v14 dynamic TerrainBlock Resource facts used by the port.

Executed: the 409750 descriptor slice for 9E4130's registration (B4040C ->
B38E24), the vtable A88DB0 vf0/vf4 getters and its vf4C 4F0500, 5ACEC0 over
a resource with flags 0x18 and 0x08 (seams: manager+20 lock object, 5AC1C0
hash insert and 59CB00 class accounting, both recorded), 405030 executes, and
the 723CE1..723CF0 slice (5AA270 memory words, then `or [esi+8],8`) from a
5AA890-constructed flags value 0x10 with +C8 = n.
Pinned bytes: the 723D30 format push (A88E08 "Dynamic/Terrain x%d y%d"), the
723960 5AA890 call, 721180/7211F0/7212D0 entries.
"""
from pathlib import Path
import hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h  # noqa: E402
from unicorn.x86_const import UC_X86_REG_ESP, UC_X86_REG_ESI, UC_X86_REG_EAX, UC_X86_REG_ECX  # noqa: E402

VT = 0xa88db0


def wide(image, a):
    out = []
    while True:
        v = struct.unpack('<H', image.range(a + 2 * len(out), 2))[0]
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
        self.events = []
        self.stub_seq = h.STUBS + 0x400

    def stub(self, handler):
        a = self.stub_seq
        self.stub_seq += 16
        self.seams[a] = handler
        return a

    def registration(self):
        # 9E4130: push 0; push B38E08; push B38E24; push A89338; mov ecx,B4040C; call 409750
        for d, parent, factory in ((0xb38e24, 0xb38e24, 0), (0xb4040c, 0xb38e24, 0)):
            self.w32(h.STACK + 0x28, parent)
            self.w32(h.STACK + 0x2c, 0xb38e08)
            self.w32(h.STACK + 0x30, factory)
            self.u.reg_write(UC_X86_REG_ESP, h.STACK)
            self.u.reg_write(UC_X86_REG_ESI, d)
            self.u.emu_start(0x40977f, 0x4097a8, count=100)
        chain, d = [], self.call(h.P.u32(VT + 4))
        while d:
            chain.append(hex(d))
            d = self.r32(d + 8)
        return dict(vf4_chain=chain, vf4c=self.call(h.P.u32(VT + 0x4c), self.alloc(0x150)))

    def publish(self, flags):
        manager = self.alloc(0x200)
        vt = self.alloc(12)
        self.w32(vt + 4, self.stub(lambda _: (self.events.append(['lock20']), self.ret())))
        self.w32(vt + 8, self.stub(lambda _: (self.events.append(['unlock20']), self.ret())))
        self.w32(manager + 0x20, vt)
        self.w32(0xb36540, manager)
        buckets = self.alloc(4 * 7)
        self.w32(manager + 0x70, buckets, 7)
        resource = self.alloc(0x150)
        self.w32(resource, VT, 0, flags)
        name = self.alloc(64)
        units = list(map(ord, 'Dynamic/Terrain x3 y7'))
        self.u.mem_write(name, struct.pack('<' + 'H' * (len(units) + 1), *units, 0))
        self.w32(resource + 0x38, name)
        self.seams[0x5ac1c0] = lambda _: (self.events.append(['5AC1C0_insert', self.arg(1)]), self.ret(8))
        self.seams[0x59cb00] = lambda _: (self.events.append(['59CB00_class', self.ecx() - manager]), self.ret(4, 0))
        self.call(0x5acec0, manager, [resource])
        return self.events

    def memory_and_flag(self, n):
        resource = self.alloc(0x150)
        self.w32(resource + 8, 0x10)
        self.w32(resource + 0xc8, n)
        self.u.reg_write(UC_X86_REG_ESP, h.STACK)
        self.w32(h.STACK + 0x10, 0)
        self.u.reg_write(UC_X86_REG_ESI, resource)
        self.u.emu_start(0x723cce, 0x723cf3, count=100)
        return dict(memory=self.r32(resource + 0xc, 2), flags=self.r32(resource + 8))


def main():
    rows = []
    for pc in h.PRECISION:
        out = []
        for image in (h.P, h.V):
            m = Machine(pc, image)
            row = dict(class_=m.registration(), publish=[Machine(pc, image).publish(f) for f in (0x18, 0x08)],
                       memory=[Machine(pc, image).memory_and_flag(n) for n in (17, 33, 0, 0x10000)])
            out.append(row)
        assert out[0] == out[1]
        rows.append(dict(precision=pc, output=out[0]))
    vtable = [hex(h.P.u32(VT + 4 * i)) for i in range(22)]
    static = []
    for a, n in ((0x723d71, 5), (0x72399a, 5), (0x723cce, 0x25), (0x721180, 0x68), (0x7211f0, 0xd8), (0x7212d0, 0x53), (0x9e4130, 0x2a)):
        raw = h.P.range(a, n)
        assert raw == h.V.range(a, n)
        static.append(dict(address=hex(a), size=n, sha256=hashlib.sha256(raw).hexdigest()))
    executed = [dict(address=hex(a), bytes=b) for a, b in sorted(h.VISITED.items())]
    out = dict(schema=1, scope=__doc__, original_sha256=h.ORIGINAL_SHA, v14_sha256=h.V14_SHA,
               format=wide(h.P, 0xa88e08), class_name=h.P.range(0xa89338, 25).decode('latin1'), vtable=vtable, rows=rows,
               static_pins=static, executed_address_byte_pins=executed)
    raw = (json.dumps(out, indent=1) + '\n').encode()
    path = HERE / 'native-terrain-block.json'
    if '--write' in sys.argv:
        path.write_bytes(raw)
    else:
        assert path.read_bytes() == raw, 'fixture differs from fresh native replay'
    print(json.dumps(dict(rows=len(rows), executed_pins=len(executed), sha256=hashlib.sha256(raw).hexdigest())))


if __name__ == '__main__':
    main()
