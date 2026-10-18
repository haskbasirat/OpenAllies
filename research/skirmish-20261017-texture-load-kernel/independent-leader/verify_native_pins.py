"""Independent read-only fixture instruction/image verification; no producer imports."""
from pathlib import Path
import sys, json, hashlib
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as n
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=[
 'research/skirmish-20261017-texture-load-kernel/native-texture-load.json',
 'research/skirmish-20261017-texture-load-kernel/native-default-texture.json',
 'research/skirmish-20261017-view-factory/native-view-factory.json',
 'research/development10/visual-map/native-dimensions.json']
out={'method':'Read existing fixtures and original/v14 PE bytes independently; no fixture mutation, no game process or Cargo build.',
 'original_sha256':n.sha(n.original.b),'v14_sha256':n.sha(n.modified.b),'fixtures':[]}
for rel in paths:
 p=ROOT/rel;d=json.loads(p.read_text())
 pins=d.get('instructions')
 if pins is None:pins=[{'va':a,'bytes':b}for a,b in d['pins'].items()]
 for item in pins:
  address=int(item['va'],16);raw=bytes.fromhex(item['bytes'])
  assert n.original.range(address,len(raw))==raw,(rel,address,'original')
  assert n.modified.range(address,len(raw))==raw,(rel,address,'v14')
 out['fixtures'].append({'path':rel,'sha256':sha(p),'rows':len(d['rows']),'pins_matching_both_images':len(pins)})
out['qualification']='Pin equality proves the observed bytes. native_machine records observers before callback cutoffs; no blanket per-case claim that every observed instruction completed. Successful allocator, child virtual bodies and transports remain as documented in each producer.'
p=Path(__file__).with_name('native-pin-verification.json');p.write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(out,indent=2))
