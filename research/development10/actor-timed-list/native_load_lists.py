"""Actual ActorLoad +128/+130 append loops 4D95AC..4D96FC, both images.

The earlier +118/+120 loops and complete ActorLoad are separate evidence.
Start at the +128 count read, with restored frame/Actor/stream registers.
Allocation and exact read transport are explicit seams; append/link/zeroing,
signed count tests, item width and list order execute native instructions.
Default invocation verifies the immutable fixture; no native process starts.
"""
from pathlib import Path
import hashlib, json, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *

PINS = {}
DEST = HERE / 'native-load-timed-lists.json'


class Machine(h.Machine):
    def __init__(self, image, precision, data):
        super().__init__(precision)
        if image is h.V:
            old = (h.P.image_size + 0xfff) & ~0xfff
            new = (image.image_size + 0xfff) & ~0xfff
            self.u.mem_map(image.base + old, new - old)
            for vs, r, raw, off in image.sections:
                if raw:
                    self.u.mem_write(image.base + r, image.b[off:off + min(raw, max(vs, raw))])
        self.data = data
        self.actor = self.alloc(0x200)
        self.stream = self.alloc(8)
        self.vtable = self.alloc(8)
        self.w32(self.stream, self.vtable)
        self.w32(self.vtable, h.STUBS)
        self.trace, self.labels = [], {}
        self.current = 0
        self.payload = b''
        for name, width in [('reciprocal', 1), ('timed', 3)]:
            rows = data[name]
            count = data.get(name + '_count', len(rows))
            assert count >= 0x80000000 or count == len(rows)
            self.payload += struct.pack('<I', count)
            self.payload += b''.join(struct.pack('<' + 'I' * width, *row) for row in rows)
        self.build(0x128, data['existing_reciprocal'], 1)
        self.build(0x130, data['existing_timed'], 3)
        self.seams[0x6f7810] = self.allocate
        self.seams[h.STUBS] = self.read
        self.u.hook_add(UC_HOOK_CODE, self.observe)

    def build(self, offset, rows, width):
        last = 0
        for row in rows:
            node = self.alloc((width + 1) * 4)
            self.w32(node, *row, 0)
            if last:
                self.w32(last + width * 4, node)
            else:
                self.w32(self.actor + offset, node)
            last = node
        self.w32(self.actor + offset + 4, last)

    def allocate(self, m):
        size = self.arg()
        assert size in [8, 16]
        node = self.alloc(size)
        # Nonzero allocator fill verifies native payload/link initialization.
        self.u.mem_write(node, b'\xa5' * size)
        self.trace.append(['allocate', size])
        self.ret(value=node)

    def read(self, m):
        dst, size = self.arg(), self.arg(1)
        assert size in [4, 12]
        self.trace.append(['read', self.current, size])
        assert self.current + size <= len(self.payload)
        self.u.mem_write(dst, self.payload[self.current:self.current + size])
        self.current += size
        self.ret(pop=8, value=self.stream)

    def observe(self, u, address, size, _):
        if address == 0x6f7810:
            return
        if h.P.base <= address < h.P.base + h.P.image_size:
            raw = bytes(u.mem_read(address, size))
            assert raw == h.P.range(address, size) == h.V.range(address, size), hex(address)
            PINS[address] = raw.hex()

    def collect(self, offset, width):
        node, last, out, seen = self.r32(self.actor + offset), 0, [], set()
        while node:
            assert node not in seen
            seen.add(node)
            out.append(self.r32(node, width))
            last, node = node, self.r32(node + width * 4)
        assert self.r32(self.actor + offset + 4) == last
        return out

    def run(self):
        self.u.reg_write(UC_X86_REG_ESP, h.STACK)
        self.u.reg_write(UC_X86_REG_ESI, self.actor)
        self.u.reg_write(UC_X86_REG_EDI, self.stream)
        self.u.reg_write(UC_X86_REG_EBX, 0)
        self.u.emu_start(0x4d95ac, 0x4d96fc, count=100000)
        assert self.u.reg_read(UC_X86_REG_EIP) == 0x4d96fc
        assert self.u.reg_read(UC_X86_REG_ESP) == h.STACK
        assert self.current == len(self.payload)
        return {'reciprocal': self.collect(0x128, 1), 'timed': self.collect(0x130, 3),
                'consumed': self.current, 'trace': self.trace}


def main():
    cases = []
    for existing in [False, True]:
        for count in [0, 1, 3, 0xffffffff, 0x80000000]:
            data = {'existing_reciprocal': [[0x10005], [0x10006]] if existing else [],
                    'existing_timed': [[0x10005, 0x3f800000, 0x40000000]] if existing else [],
                    'reciprocal_count': count, 'timed_count': count,
                    'reciprocal': [[0x20001 + i] for i in range(count if count < 4 else 0)],
                    'timed': [[0x20001 + i, 0x80000000 + i, 0x7fc00000 + i] for i in range(count if count < 4 else 0)]}
            for pc in h.PRECISION:
                original = Machine(h.P, pc, data).run()
                expanded = Machine(h.V, pc, data).run()
                assert original == expanded
                assert original['reciprocal'] == data['existing_reciprocal'] + data['reciprocal']
                assert original['timed'] == data['existing_timed'] + data['timed']
                cases.append({'input': data, 'precision': pc, 'output': original})
    doc = {'scope': __doc__, 'original_sha256': h.ORIGINAL_SHA, 'v14_sha256': h.V14_SHA,
           'paired_cases': len(cases), 'native_executions': 2 * len(cases), 'cases': cases,
           'executed_pins': [{'va': hex(k), 'bytes': v} for k, v in sorted(PINS.items())],
           'observed_stop': '4D96FC not executed', 'successful_seams': ['6F7810 successful allocation', 'byte read transport only']}
    encoded = (json.dumps(doc, indent=1) + '\n').encode()
    if '--capture' in sys.argv:
        assert not DEST.exists()
        DEST.write_bytes(encoded)
    else:
        assert DEST.read_bytes() == encoded
    print(json.dumps({'paired_cases': len(cases), 'native_executions': 2 * len(cases),
                      'executed_pins': len(PINS), 'sha256': hashlib.sha256(encoded).hexdigest()}))


if __name__ == '__main__':
    main()
