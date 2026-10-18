"""Rekey immutable emitter addresses by actual descriptor allocation identity.

Existing static emitter addresses are retained as historical static evidence;
this does not claim new native execution of every emitter. New766D40 has its
own fresh full-registry execution fixture.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
old_root=HERE/'historical-357'
old=json.loads((old_root/'research/phase4-orders/registry-exploration.json').read_text())
new=json.loads((ROOT/'research/phase4-orders/registry-exploration.json').read_text())
sites=json.loads((old_root/'research/phase8-orders/descriptor-write-sites.json').read_text())
by_address={r['descriptor']:r for r in new}
mapping={}
for row in old:
 current=by_address[row['descriptor']]
 assert all(current[k]==row[k]for k in ('name','handler','initializer'))
 mapping[row['id']]=current['id']
output={str(mapping[int(i)]):v for i,v in sites.items()}
assert '323'not in output
output['323']=['0x766d40']
output={k:output[k]for k in sorted(output,key=int)}
target=ROOT/'research/phase8-orders/descriptor-write-sites.json'
if '--write'in sys.argv:target.write_text(json.dumps(output,indent=1)+'\n')
else:assert json.loads(target.read_text())==output
print(len(output),'descriptor emitter groups; identity-projected static entries and separately executed766D40')
