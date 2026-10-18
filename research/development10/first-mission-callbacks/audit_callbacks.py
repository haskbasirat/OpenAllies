"""Read-only direct/virtual call slices; this is NOT an execution oracle.

Native function lengths come from the exported Ghidra functions inventory.
The same bytes are checked in both images. Reachability and mutable argument
values are only claimed where the genuine attempt37 failure establishes them.
"""
from pathlib import Path
import csv, hashlib, json, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
sys.path.insert(0, str(ROOT / 'research/phase8-ai'))
from native_machine import original, modified
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

sha = lambda data: hashlib.sha256(data).hexdigest()
addresses = [0x4d7230, 0x4f9c10, 0x4fa590, 0x4fba50, 0x4fc4a0]
inventory = ROOT.parent / 'reconstruction/research/native-aa/functions.tsv'
sizes = {int(row['address'], 16): int(row['bodyBytes'])
         for row in csv.DictReader(inventory.open(), delimiter='\t')
         if int(row['address'], 16) in addresses}
disassembler = Cs(CS_ARCH_X86, CS_MODE_32)
rows = []
for address in addresses:
    raw = original.range(address, sizes[address])
    assert raw == modified.range(address, len(raw))
    calls = [dict(address=hex(i.address), bytes=i.bytes.hex(),
                  mnemonic=i.mnemonic, operands=i.op_str)
             for i in disassembler.disasm(raw, address) if i.mnemonic == 'call']
    rows.append(dict(entry=hex(address), contiguous_bytes=len(raw),
                     bytes_sha256=sha(raw), calls=calls))
document = dict(scope=__doc__, original_sha256=sha(original.b),
                v14_sha256=sha(modified.b), producer_sha256=sha(Path(__file__).read_bytes()),
                inventory_sha256=sha(inventory.read_bytes()), rows=rows,
                qualification='Static contiguous disassembly only; not a full CFG or executed branch trace.')
path = HERE / 'native-call-slices.json'
if '--create' in sys.argv:
    with path.open('x') as output:
        json.dump(document, output, indent=2)
        output.write('\n')
else:
    assert json.loads(path.read_text()) == document
print(json.dumps(dict(functions=len(rows), call_pins=sum(len(r['calls']) for r in rows),
                      sha256=sha(path.read_bytes()))))
