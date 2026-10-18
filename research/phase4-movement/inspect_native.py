"""Hash-pinned read-only disassembly for original mover recovery."""
from pathlib import Path
import hashlib, json, struct, sys

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
sys.path.insert(0, str(ROOT / 'research/phase3-terrain'))
from terrain_uv_oracle import Pe, ORIGINAL_SHA, ACCEPTED_SHA

OUT = Path(__file__).parent
RANGES = [(0x5766a0, 0x578350), (0x4453a0, 0x445529), (0x4be530, 0x4be548),
          (0x529c30, 0x529c37), (0x46593e, 0x46599d),
          (0x50701d, 0x507053), (0x508f29, 0x508f5a),
          (0x4eeb40, 0x4eebd4), (0x5300a0, 0x5303c8)]


def main():
    original = Pe(GAME / 'AA.exe', ORIGINAL_SHA)
    accepted = Pe(GAME / 'reconstruction/native-game/runtime-expanded-v14/AA.exe', ACCEPTED_SHA)
    md = Cs(CS_ARCH_X86, CS_MODE_32)
    rows = []
    for start, end in RANGES:
        data = original.at(start, end - start)
        assert data == accepted.at(start, end - start)
        text = '\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}' for i in md.disasm(data, start))
        path = OUT / f'{start:08x}-assembly.txt'
        path.write_text(text + '\n')
        rows.append({'start': hex(start), 'end': hex(end), 'sha256': hashlib.sha256(data).hexdigest(), 'v14_identical': True})
    tables = {}
    for start, count in [(0xA3CE88, 18)]:
        tables[hex(start)] = [hex(v) for v in struct.unpack('<' + 'I' * count, original.at(start, count * 4))]
    report = {'original_sha256': ORIGINAL_SHA, 'accepted_v14_sha256': ACCEPTED_SHA,
              'ranges': rows, 'tables': tables}
    (OUT / 'native-ranges.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main()
