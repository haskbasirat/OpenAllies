"""Original Element Mind (slot 4; factory 4CF480 ActorData+174 bit 8): constructor
535BA0 then saved Load 536340 on the same 0x1BC-byte object, under Unicorn.

Native code executed: 536340 (Element Load), 52C580 (base Mind Load: Updatable
50BCD0, command list, state-name stack resolved through Element virtual +19C =
531C30, _wcscmp 96CB83, current-command flag), 52D2C0 (owned path Load at +D4,
node constructor 52D000), 552CC0, 763B90, CString 401B00/405B80/401330.

Seams (explicit, none consume stream bytes):
  * stream byte source (base harness) and allocators 6F7810 / pool 85C680;
  * 530E50 (Element ability jitter: RNG draw and Performer ability writes) is
    skipped -- it is proven by aa-mind's native_constructors fixture and Load
    never reads or writes +1B8;
  * 8589B0 (version reader): i32, `v < 0 || v > max` reports "Bad data
    version" (41DFB0) -- recorded as outcome `bad_version`;
  * 41D290 (wide string reader: u16 unit count, units) -- base harness;
  * 5A0910 (KKC_MapResource::ThrowCancelException "Map Load Canceled") is the
    outcome `canceled`;
  * CRT _wcsicmp 96CAD6 C-locale (ASCII folding).
Load executes no x87 instruction (the jitter seam removes the only FPU code),
so results are precision independent; cases still run at PC24/PC53/PC64.
"""
from pathlib import Path
import sys, json, struct, random, hashlib
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from native_supply import Machine as Base, P, V, VISITED
from unicorn.x86_const import *

H = lambda b: hashlib.sha256(b).hexdigest()
ELEMENT_SIZE = 0x1BC
STATE_NAMES = {  # Element embedded state offset -> registered metadata name
    0x34: 'uai_unload', 0x48: 'uai_deploy', 0x68: 'uai_sleep', 0x7c: 'uai_move',
    0xf8: 'uai_kill', 0x10c: 'uai_bombard', 0x12c: 'uai_engage', 0x148: 'uai_repair',
    0x164: 'uai_join', 0x178: 'uai_reattach', 0x188: 'uai_unpack',
}
METADATA = {0x34: 0xb3d308, 0x48: 0xb3d320, 0x68: 0xb3d338, 0x7c: 0xb3d350, 0xf8: 0xb3d368,
            0x10c: 0xb3d380, 0x12c: 0xb3d3b0, 0x148: 0xb3d3c8, 0x164: 0xb3d3e0,
            0x178: 0xb3d3f8, 0x188: 0xb3d410}


class Outcome(Exception):
    pass


class Machine(Base):
    def hook(self, u, pc, size, arg):
        if pc == 0x530e50:
            self.ret()
            return
        if pc == 0x85c680:
            self.ret(value=self.alloc(0x28))
            return
        if pc == 0x8589b0:
            value = struct.unpack('<i', self.r.take(4))[0]
            if value < 0 or value > self.arg(1):
                raise Outcome('bad_version')
            ecx = u.reg_read(UC_X86_REG_ECX)
            self.write(ecx, value & 0xffffffff)
            self.ret(8, value=ecx)
            return
        if pc == 0x405b80:  # run the original CString assignment
            return
        if pc == 0x5a0910:
            raise Outcome('canceled')
        super().hook(u, pc, size, arg)

    def element(self):
        for offset, meta in METADATA.items():
            self.write(meta, self.string(STATE_NAMES[offset].encode('utf-16-le')))
        owner = self.alloc(0x200)
        self.write(owner + 4, self.alloc(0x800))
        obj = self.alloc(ELEMENT_SIZE + 4)
        self.u.mem_write(obj, b'\xcd' * ELEMENT_SIZE)
        self.call(0x535ba0, obj, [owner])
        before = bytes(self.u.mem_read(obj, ELEMENT_SIZE))
        try:
            self.call(0x536340, obj, [self.world])
            outcome = 'loaded'
        except Outcome as e:
            outcome = str(e)
        except Exception as e:  # stream exhaustion inside the seam
            if 'take' in repr(e) or isinstance(e, (EOFError, AssertionError, IndexError, struct.error)):
                outcome = 'short_read'
            else:
                raise
        result = {'outcome': outcome, 'consumed': self.r.at}
        if outcome != 'loaded':
            return result
        image = bytes(self.u.mem_read(obj, ELEMENT_SIZE))
        stack, node = [], self.read(obj + 0x10)
        while node:
            stack.append(self.read(node) - obj)
            node = self.read(node + 4)
        path, node = [], self.read(obj + 0xd8)
        while node:
            path.append([self.read(node + 4 * i) for i in range(8)])
            node = self.read(node + 0x24)
        # Only fields at known non-pointer offsets are meaningful for replay;
        # the full image is kept so the test can prove untouched words too.
        text = self.read(obj + 0x120)
        units = self.read(text - 12)
        result.update({'image_after_ctor': before.hex(), 'image': image.hex(),
                       'bombard_text_units': [int.from_bytes(self.u.mem_read(text + 2 * i, 2), 'little')
                                              for i in range(units)],
                       'stack_offsets': stack, 'path_head_first': path,
                       'path_tail_is_last': (self.read(obj + 0xdc) == 0) == (not path),
                       'commands_empty': self.read(obj + 0x18) == 0 and self.read(obj + 0x20) == 0})
        return result


W = lambda *v: struct.pack('<' + 'I' * len(v), *[x & 0xffffffff for x in v])


def wstr(units):
    return struct.pack('<H', len(units)) + b''.join(struct.pack('<H', u) for u in units)


def name(text):
    return [ord(c) for c in text]


def synthetic(rng, version, base_version=3, updatable=1, stack=('uai_sleep',), path=(), text=(),
              path_version=0, overrides=None):
    o = overrides or {}
    r = lambda: rng.getrandbits(32)
    out = W(o.get('version', version))
    if version == 0:
        return out
    out += W(updatable) + (W(rng.choice([0, 5, 11, 0x80000000, 0xffffffff])) if updatable >= 1 else b'')
    out += W(o.get('base_version', base_version)) + W(r(), r()) + W(0)
    if base_version >= 1:
        out += W(r()) + W(len(stack)) + b''.join(wstr(s if isinstance(s, list) else name(s)) for s in stack)
    if base_version >= 3:
        out += b'\0'
    if version < 2:
        return out
    out += W(rng.choice([0, 1, 2, r()]), rng.choice([0, 1, 2, 3]))
    if version >= 9:
        out += W(r())
    a8 = rng.choice([0, 1, 2, 2, 3])
    out += W(rng.choice([0, 1, 2]), r(), r(), r(), r(), r(), r(), r(), a8, r(), r(), r(), r())
    if version >= 12:
        out += W(r())
    out += W(path_version) + W(len(path))
    for words, discarded in path:
        out += W(*words[0:4]) + W(words[4], words[5], discarded) + W(words[6], words[7])
    out += W(r())
    if version >= 10:
        out += W(r(), r(), r(), r(), r(), r())
    if 5 <= version < 7:
        out += bytes([rng.getrandbits(8)])
    if version >= 7:
        out += W(r())
    out += W(rng.choice([0, 1, 2]), r())
    if version < 4:
        out += bytes([rng.getrandbits(8)]) + W(r(), r(), r(), r(), r(), r(), r())
    out += W(rng.choice([0, 1, 2]), r(), r(), r()) + bytes([rng.getrandbits(8)])
    out += W(rng.choice([0, 1, 2])) + bytes([rng.getrandbits(8)]) + W(r())
    if version >= 8:
        out += bytes([rng.getrandbits(8)])
    out += W(rng.choice([0, 1, 2]), r(), rng.choice([0, 1, 2]))
    if version >= 3:
        out += W(r())
    if version >= 4:
        out += W(rng.choice([0, 1, 2]))
        if version < 6:
            out += bytes([rng.getrandbits(8)])
        out += W(r(), r(), r()) + wstr(list(text))
    if version >= 9:
        out += W(rng.choice([0, 1, 2]), r(), r())
        if version >= 13:
            out += W(r(), r())
    if version >= 11:
        out += W(r(), r(), r(), r(), r())
    if version >= 14:
        out += W(rng.choice([0, 1, 2]), rng.choice([0, 1, 2]))
    return out


def path_records(rng, n):
    rows = []
    for _ in range(n):
        words = [rng.getrandbits(32) for _ in range(8)]
        words[4] = rng.choice([0, 1, 2, 2, 3, 0xffffffff])
        rows.append((words, rng.getrandbits(32)))
    return rows


def cases(corpus):
    rows = []
    seen = set()
    for line in corpus.read_text().splitlines():
        rec = json.loads(line)
        if rec['tail_hex'] in seen:
            continue
        seen.add(rec['tail_hex'])
        rows.append({'source': f"{rec['map']}#{rec['saved_id']}:{rec['definition']}", 'precision': 'Double53',
                     'payload': rec['tail_hex'], 'corpus': True})
    rng = random.Random(0x536340)
    syn = []
    listed = [n for o, n in STATE_NAMES.items() if o != 0x34]
    for version in range(15):
        for base_version in range(4):
            for updatable in (0, 1):
                stack = rng.sample(listed, rng.choice([0, 1, 2, 3]))
                syn.append(('versions', synthetic(rng, version, base_version, updatable, stack,
                                                 path_records(rng, rng.choice([0, 0, 1, 3])),
                                                 rng.choice([(), name('bombard'), [0x41, 0, 0x42]]))))
    for nm in listed:
        syn.append(('stack_' + nm, synthetic(rng, 14, stack=[nm])))
    syn += [
        ('stack_all_listed', synthetic(rng, 14, stack=listed + listed[:3])),
        ('stack_uai_unload_cancels', synthetic(rng, 14, stack=['uai_unload'])),
        ('stack_unknown_cancels', synthetic(rng, 14, stack=['uai_sleep', 'nonexistent'])),
        ('stack_empty_name_rampage_cancels', synthetic(rng, 14, stack=[''])),
        ('stack_case_sensitive_cancels', synthetic(rng, 14, stack=['UAI_SLEEP'])),
        ('stack_embedded_nul_matches', synthetic(rng, 14, stack=[name('uai_sleep') + [0, 0x78]])),
        ('stack_leading_nul_rampage', synthetic(rng, 14, stack=[[0, 0x78]])),
        ('path_many', synthetic(rng, 14, path=path_records(rng, 9))),
        ('text_long', synthetic(rng, 14, text=name('x' * 300))),
        ('bad_outer_version', synthetic(rng, 14, overrides={'version': 15})),
        ('bad_outer_negative', synthetic(rng, 14, overrides={'version': -1})),
        ('bad_base_version', synthetic(rng, 14, overrides={'base_version': 4})),
        ('bad_updatable_version', synthetic(rng, 14, updatable=2)),
        ('bad_path_version', synthetic(rng, 14, path_version=1)),
        ('truncated', synthetic(rng, 14)[:-3]),
    ]
    # A negative stack count reads no names (signed loop).
    neg = bytearray(synthetic(rng, 14, stack=[]))
    at = 4 + 8 + 4 + 8 + 4 + 4
    assert struct.unpack_from('<I', neg, at)[0] == 0
    struct.pack_into('<i', neg, at, -3)
    syn.append(('stack_negative_count', bytes(neg)))
    for i, (label, payload) in enumerate(syn):
        precision = ['Single24', 'Double53', 'Extended64'][i % 3]
        rows.append({'source': 'synthetic:' + label, 'precision': precision, 'payload': payload.hex(),
                     'corpus': False})
    return rows


def main():
    corpus = Path(sys.argv[1])
    out = []
    for c in cases(corpus):
        cw = {'Single24': 0x7f, 'Double53': 0x27f, 'Extended64': 0x37f}[c['precision']]
        payload = bytes.fromhex(c['payload'])
        m = Machine(payload, cw)
        native = m.element()
        if c['corpus']:
            assert native['outcome'] == 'loaded', (c['source'], native['outcome'])
            c['payload'] = payload[:native['consumed']].hex()
        out.append({'input': c, 'native': native})
    for a, b in VISITED.items():
        assert bytes.fromhex(b) == P.range(a, len(bytes.fromhex(b))) == V.range(a, len(bytes.fromhex(b)))
    doc = {'schema': 1, 'passed': True, 'producer_sha256': H(Path(__file__).read_bytes()),
           'original_sha256': H(P.b), 'v14_sha256': H(V.b), 'scope': __doc__,
           'corpus_sha256': H(corpus.read_bytes()), 'cases': out}
    path = HERE / 'native-mind-load.json'
    path.write_text(json.dumps(doc, separators=(',', ':')))
    outcomes = {}
    for row in out:
        outcomes[row['native']['outcome']] = outcomes.get(row['native']['outcome'], 0) + 1
    print(json.dumps({'cases': len(out), 'outcomes': outcomes, 'sha256': H(path.read_bytes())}))


if __name__ == '__main__':
    main()
