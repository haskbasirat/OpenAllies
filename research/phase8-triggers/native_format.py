"""Oracle for the original VC7.1 CRT wide %f formatting (_vswprintf 96C7EC).

Trigger camera/video/banner commands are built with CString::Format and parsed
back by the UI command interpreter, so the exact text is gameplay-visible.
This executes the ORIGINAL _vswprintf on generated binary64 inputs and records
the produced text.  Writes native-format.json.
"""
from pathlib import Path
import hashlib, json, random, struct, sys
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import harness as h  # noqa: E402
from unicorn.x86_const import *  # noqa: E402,F403

FMTS = ['%f', '%d', '%.2f']


def values():
    rnd = random.Random(20261014)
    out = set()
    specials = [0.0, -0.0, 0.5, 1.0, -1.0, 0.1, 0.0000005, 0.0000015, 0.0000025, 0.00000049999999,
                123.4567895, 1e6, 1e7, 1e9, 1e10, 123456789.125, 16777216.0, 4294967296.0,
                2.5e-7, 1.5e-6, 9.9999995, 0.9999995, 99999.9999995, 1e15, 1e16, 1e17, 1e20, 1e22, 3e30,
                float.fromhex('0x1.fffffffffffffp+52'), 0.30000000000000004]
    for v in specials:
        out.add(v)
        out.add(-v)
    for _ in range(3000):           # binary32 values (camera fields, durations)
        bits = rnd.getrandbits(32)
        e = (bits >> 23) & 0xff
        if e == 0xff:
            continue
        if not 90 <= e <= 160:
            bits = (bits & 0x807fffff) | (rnd.randint(110, 150) << 23)
        out.add(struct.unpack('<f', struct.pack('<I', bits))[0])
    for _ in range(2000):           # binary32 sums carried in binary64 (times)
        a = struct.unpack('<f', struct.pack('<f', rnd.uniform(0, 5000)))[0]
        b = struct.unpack('<f', struct.pack('<f', rnd.uniform(0, 60)))[0]
        out.add(a + b)
    for _ in range(2000):           # decimal ties at 6 places
        k = rnd.randint(0, 10 ** 9)
        out.add((k + 0.5) / 1e6)
    for _ in range(1000):           # random binary64
        out.add(struct.unpack('<d', struct.pack('<Q', rnd.getrandbits(64) & 0x43ffffffffffffff))[0])
    return sorted(v for v in out if v == v and abs(v) != float('inf'))


class Formatter(h.Machine):
    def __init__(self):
        c = {'time_bits': 0, 'precision': 'Double53', 'variables': {}, 'triggers': [{'id': 1, 'enabled': 1, 'activation_bits': 0}]}
        super().__init__(c)

    def format(self, fmt, raw_args):
        va = self.alloc(64)
        self.u.mem_write(va, raw_args)
        f = self.string(fmt)
        self.w(h.core.STACK, h.core.STOP)
        self.w(h.core.STACK + 4, h.SCRATCH)
        self.w(h.core.STACK + 8, f)
        self.w(h.core.STACK + 12, va)
        self.u.reg_write(UC_X86_REG_ESP, h.core.STACK)
        self.u.emu_start(0x96c7ec, h.core.STOP, count=2000000)
        assert self.u.reg_read(UC_X86_REG_EIP) == h.core.STOP
        n = self.u.reg_read(UC_X86_REG_EAX)
        text = self.text(h.SCRATCH)
        assert len(text) == n, (text, n)
        return text


def main():
    m = Formatter()
    rows = []
    for v in values():
        raw = struct.pack('<d', v)
        rows.append({'fmt': '%f', 'bits': struct.unpack('<Q', raw)[0], 'text': m.format('%f', raw)})
        if abs(v) < 1e9:
            rows.append({'fmt': '%.2f', 'bits': struct.unpack('<Q', raw)[0], 'text': m.format('%.2f', raw)})
        m.bump = h.BASE + 0x200000          # reuse scratch allocations
        m.strings.clear()
    for i in [0, 1, -1, 2147483647, -2147483648, 7]:
        rows.append({'fmt': '%d', 'bits': i & 0xffffffff, 'text': m.format('%d', struct.pack('<i', i))})
    windows = []
    for a, n in sorted(m.visited.items()):
        raw = h.core.IMAGE.at(a, n)
        assert raw == h.core.ACCEPTED.at(a, n)
        windows.append({'address': hex(a), 'bytes': raw.hex()})
    doc = {'schema': 1, 'original_sha256': h.core.pe.ORIGINAL_SHA, 'v14_sha256': h.core.pe.ACCEPTED_SHA,
           'producer_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
           'scope': 'ORIGINAL VC7.1 _vswprintf (96C7EC) wide %f/%.2f/%d formatting; C locale via _getptd seam.',
           'cases': rows, 'visited_windows': windows}
    path = HERE / 'native-format.json'
    path.write_text(json.dumps(doc, indent=0) + '\n')
    print(json.dumps({'cases': len(rows), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}))


if __name__ == '__main__':
    main()
