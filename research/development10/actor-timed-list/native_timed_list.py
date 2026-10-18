"""Whole 4D37E0 with actual 4F1430/5AC280 list children, both pinned PEs.

The only successful seam is allocator free; it records the freed allocation.
Synthetic typed actor/registry/list inputs do not attest a complete Actor ctor.
No native process is launched. Default invocation compares immutable evidence.
"""
from pathlib import Path
import hashlib, json, random, struct, sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *

HERE = Path(__file__).resolve().parent
DEST = HERE / 'native-timed-list.json'
PINS = {}
SELF = 0x10001


class Machine(h.Machine):
    def __init__(self, image, pc, data):
        super().__init__(pc)
        if image is h.V:
            old_end = (h.P.image_size + 0xfff) & ~0xfff
            new_end = (image.image_size + 0xfff) & ~0xfff
            self.u.mem_map(image.base + old_end, new_end - old_end)
            for vs, r, raw, off in image.sections:
                if raw:
                    self.u.mem_write(image.base + r, image.b[off:off + min(raw, max(vs, raw))])
        self.data = data
        self.trace, self.freed, self.labels = [], [], {}
        self.actor = self.alloc(0x200)
        self.world = self.alloc(0x200)
        self.registry = self.alloc(0x60010)
        self.w32(0xb364c0, self.registry)
        self.w32(0xb36534, self.world)
        self.w32(self.actor + 8, data['flags'])
        self.w32(self.actor + 0x14, SELF)
        self.w32(self.world + 0xe0, data['time'])
        self.targets = {1: self.actor}
        for index in [2, 3, 4]:
            self.targets[index] = self.alloc(0x200)
        for index, ptr in self.targets.items():
            generation, present = data['registry'][str(index)]
            self.u.mem_write(self.registry + 4 + 2 * index, struct.pack('<H', generation))
            self.w32(self.registry + 0x20004 + 4 * index, ptr if present else 0)
            self.w32(ptr + 0x14, 0x10000 | index)
            self.build_list(ptr + 0x128, data['reciprocal'][str(index)], 1, ('reciprocal', index))
        self.build_list(self.actor + 0x130, data['timed'], 3, ('timed', 1))
        self.seams[0x6f7430] = self.free
        self.u.hook_add(UC_HOOK_CODE, self.observe)

    def build_list(self, field, values, width, label):
        prior = 0
        for i, value in enumerate(values):
            ptr = self.alloc(4 * (width + 1))
            words = value if width > 1 else [value]
            self.w32(ptr, *words, 0)
            self.labels[ptr] = [*label, i]
            if prior:
                self.w32(prior + 4 * width, ptr)
            else:
                self.w32(field, ptr)
            prior = ptr
        self.w32(field + 4, prior)

    def free(self, m):
        ptr = self.arg()
        assert ptr in self.labels and self.labels[ptr] not in self.freed
        self.freed.append(self.labels[ptr])
        self.trace.append(['free', self.labels[ptr]])
        self.ret()

    def observe(self, u, address, size, _):
        # Hooks run before execution. Platform free and explicit cutoffs are
        # observed entry boundaries, never included as executed body pins.
        if address == 0x6f7430 or address == h.STOP:
            return
        if self.data.get('cutoff') == hex(address):
            self.trace.append(['required', hex(address)])
            self.u.emu_stop()
            return
        if h.P.base <= address < h.P.base + h.P.image_size:
            raw = bytes(u.mem_read(address, size))
            assert raw == h.P.range(address, size) == h.V.range(address, size), hex(address)
            PINS[address] = raw.hex()
        node = u.reg_read(UC_X86_REG_ESI)
        if address == 0x4d3801:
            self.trace.append(['flags', self.r32(self.actor + 8)])
        elif address in [0x4d3814, 0x4d3852, 0x4d386f]:
            self.trace.append(['resolve', self.r32(node)])
        elif address == 0x4d3831:
            self.trace.append(['time', self.r32(self.world + 0xe0)])
        elif address == 0x4f1430:
            target = next(k for k, p in self.targets.items() if p + 0x128 == self.ecx())
            self.trace.append(['reciprocal', target, self.r32(self.arg())])
        elif address == 0x5ac280:
            self.trace.append(['remove', self.labels[self.arg()][2]])

    def collect_list(self, field, width):
        node, out, seen = self.r32(field), [], set()
        last = 0
        while node:
            assert node not in seen
            seen.add(node)
            out.append({'index': self.labels[node][2], 'value': self.r32(node, width)})
            last, node = node, self.r32(node + 4 * width)
        assert self.r32(field + 4) == last
        return out

    def run(self):
        if self.data.get('cutoff'):
            self.w32(h.STACK, h.STOP)
            self.u.reg_write(UC_X86_REG_ESP, h.STACK)
            self.u.reg_write(UC_X86_REG_ECX, self.actor)
            self.u.emu_start(0x4d37e0, h.STOP, count=100000)
            assert hex(self.u.reg_read(UC_X86_REG_EIP)) == self.data['cutoff']
        else:
            self.call(0x4d37e0, self.actor)
            assert self.fpu_top() == 0
            assert self.final_esp == h.STACK + 4
        return {'timed': self.collect_list(self.actor + 0x130, 3),
                'reciprocal': {str(k): self.collect_list(p + 0x128, 1) for k, p in self.targets.items()},
                'freed': self.freed, 'trace': self.trace}


def base(flags=0, time=0x3f800000):
    return {'flags': flags, 'time': time,
            'registry': {str(i): [1, True] for i in range(1, 5)},
            'reciprocal': {str(i): [SELF, 0x10003, SELF] for i in range(1, 5)},
            'timed': []}


def inputs():
    result = []
    for flags in [0, 1, 0x80000000]:
        case = base(flags)
        result.append(case)
    triples = [
        (0, 0, 0), (0x80000000, 0, 0x80000000),
        (0x3f800000, 0, 0x3f800000), (0x3f800000, 1, 0x3f800000),
        (0x3f800001, 0, 0x3f800000), (0x3f7fffff, 0, 0x3f800000),
        (0x3f800000, 0xbf800000, 0), (1, 1, 1),
        (0x80000001, 0, 0), (0x7f7fffff, 0x7f7fffff, 0x7f800000),
        (0x7f800000, 0, 0), (0xff800000, 0, 0),
        (0x7f800000, 0xff800000, 0), (0x7f800000, 0, 0x7f800000),
        (0xff800000, 0, 0xff800000), (0, 0, 0xff800000),
        (0, 0, 0x7fc00001), (0x7fc00001, 0, 0), (0, 0x7f800001, 0),
        (0x3f800000, 0x33800000, 0x3f800000),
        (0x3f800001, 0xb3800000, 0x3f800000),
    ]
    for flags in [0, 1]:
        for status in ['live', 'stale', 'null']:
            for start, duration, time in triples:
                case = base(flags, time)
                case['registry']['2'] = {'live': [1, True], 'stale': [2, True], 'null': [1, False]}[status]
                case['timed'] = [[0x10002, start, duration]]
                result.append(case)
    # Real child-list edge domains: head/interior/tail/absent reciprocal match,
    # duplicate timed GTIDs, self targets, consecutive removals and survivors.
    for reciprocal in [[], [SELF], [0x10003, SELF], [SELF, SELF], [0x10003, SELF, 0x10004], [0x10003]]:
        case = base()
        case['reciprocal']['2'] = reciprocal
        case['timed'] = [[0x10002, 0, 0], [SELF, 0x40000000, 0], [0x10002, 0, 0], [0x10003, 0, 0]]
        result.append(case)
    rng = random.Random(0x4d37e0)
    finite = [0, 1, 0x80000000, 0x3f000000, 0x3f800000, 0xbf800000, 0x40000000, 0x40400000]
    for _ in range(16):
        case = base(rng.choice([0, 1]), rng.choice(finite))
        case['timed'] = [[rng.choice([SELF, 0x10002, 0x20003, 0x10004]), rng.choice(finite), rng.choice(finite)] for _ in range(rng.randrange(2, 8))]
        for i in [1, 2, 3, 4]:
            case['reciprocal'][str(i)] = [rng.choice([SELF, 0x10002, 0x20003]) for _ in range(rng.randrange(6))]
        result.append(case)
    for cutoff in [0x4f1430, 0x5ac280]:
        case = base()
        case['timed'] = [[0x10002, 0, 0], [0x10003, 0, 0]]
        case['cutoff'] = hex(cutoff)
        result.append(case)
    return result


def main():
    cases = []
    for i, data in enumerate(inputs()):
        for pc in h.PRECISION:
            original = Machine(h.P, pc, data).run()
            expanded = Machine(h.V, pc, data).run()
            assert original == expanded, (i, pc)
            cases.append({'input': data, 'precision': pc, 'output': original})
    document = {'scope': __doc__, 'original_sha256': h.ORIGINAL_SHA, 'v14_sha256': h.V14_SHA,
                'paired_cases': len(cases), 'native_executions': len(cases) * 2,
                'cases': cases, 'executed_pins': [{'va': hex(k), 'bytes': v} for k, v in sorted(PINS.items())],
                'successful_seams': ['6F7430 allocation free only; actual list unlink/copy/GTID/time instructions execute'],
                'observed_cutoffs': ['4F1430 required entry', '5AC280 required entry']}
    encoded = (json.dumps(document, indent=1) + '\n').encode()
    if '--capture' in sys.argv:
        assert not DEST.exists(), 'immutable fixture already exists'
        DEST.write_bytes(encoded)
    else:
        assert DEST.read_bytes() == encoded, 'native fixture changed'
    print(json.dumps({'paired_cases': len(cases), 'native_executions': len(cases) * 2,
                      'executed_pins': len(PINS), 'sha256': hashlib.sha256(encoded).hexdigest()}))


if __name__ == '__main__':
    main()
