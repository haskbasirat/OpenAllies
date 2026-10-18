"""Read-only source binding and instruction listings for combat reconstruction."""
from pathlib import Path
import csv, hashlib, json, struct, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase4-body'))
from original_body import IMAGE, ACCEPTED, pe
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

FUNCTIONS = [0x5625c0, 0x562380, 0x561e50, 0x561be0, 0x562ba0,
             0x45d430, 0x4c8100, 0x501650, 0x409230, 0x4be740,
             0x4d0960, 0x4d1120, 0x4cf110, 0x58cb30,
             0x4ca400, 0x4d57c0, 0x562d10]
def sha(value): return hashlib.sha256(value).hexdigest()
def main():
    sizes = {int(r['address'],16): int(r['bodyBytes']) for r in csv.DictReader(
        (ROOT.parent / 'reconstruction/research/native-aa/functions.tsv').open(), delimiter='\t')}
    cs = Cs(CS_ARCH_X86, CS_MODE_32)
    ranges = []
    for address in FUNCTIONS:
        code = IMAGE.at(address, sizes[address])
        assert code == ACCEPTED.at(address, len(code))
        ranges.append({'start': hex(address), 'size': len(code), 'sha256': sha(code)})
        (HERE / f'{address:08x}.asm.txt').write_text('\n'.join(
            f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in cs.disasm(code,address))+'\n')
    constants = {}
    for address in [0x9f32dc, 0x9f32e0, 0x9f3228, 0xa30de0]:
        raw = IMAGE.at(address,4)
        assert raw == ACCEPTED.at(address,4)
        constants[hex(address)] = {'bits': struct.unpack('<I',raw)[0], 'f32': struct.unpack('<f',raw)[0]}
    report = {'schema':1, 'original_sha256':pe.ORIGINAL_SHA,
              'accepted_sha256':pe.ACCEPTED_SHA, 'ranges':ranges, 'constants':constants}
    (HERE/'native-ranges.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(constants))
if __name__ == '__main__': main()
