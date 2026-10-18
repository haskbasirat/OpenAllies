"""Generate only from the corrected executed CRT registration inventory."""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
rows=json.loads((HERE/'orders-native-oracle.json').read_text())['registry']
assert [r['id']for r in rows]==list(range(358))
assert rows[323]['initializer']=='0x9e65f0' and rows[323]['name']=='OrderVersionAckRequest'
text='// Generated from both-PE actual CRT execution; see phase4-orders and the 20261017 correction.\n'
text+=f'pub const DESCRIPTORS: [Descriptor; {len(rows)}] = [\n'
for r in rows:text+=f'    Descriptor {{ id: {r["id"]}, name: {json.dumps(r["name"])}, handler_va: {r["handler"]} }},\n'
text+='];\n'
target=ROOT/'crates/aa-orders/src/native_registry.rs'
if '--write' in sys.argv:target.write_text(text)
else:assert target.read_text()==text
print(len(rows),'descriptors')
