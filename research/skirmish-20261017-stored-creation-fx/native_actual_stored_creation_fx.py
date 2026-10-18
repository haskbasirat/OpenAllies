"""Actual constructed SourceNew ActorStore/CurrentInputs capture -> native event.
The Rust lab uses real source/Bank/constructor/clock/queue owners but explicit
counting execution IDs, not a real OS worker nor a completed Staged World.
Native42A590/429080/428480 executes; typed actor virtual getters and allocator/
OS-ID providers remain explicit fixture services. Source Thing+0C is null.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_existing_fx_null_view as core

def main():
 source=HERE/'actual-source-input.json';captured=json.loads(source.read_text());assert len(captured)==3
 rows=[];pins={}
 for row in captured:
  c=row['input'];pc=row['precision']
  assert c['source_view']is False and c['kind']=='Actor' and c['flag']==0
  assert row['source_id']=='ger_corps' and row['graph_nodes']==20 and row['constructed_roots']==17
  a,pa=core.run(c,'original',pc);b,pb=core.run(c,'v14',pc);assert a==b
  for command in row['commands']:assert command==a['command'],(pc,command,a)
  assert row['pending']==2 and row['execution_fixture_trace']==['current_os','application_os']*2
  rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert raw==core.ORIGINAL.range(va,len(raw))==core.nm.modified.range(va,len(raw))
 doc={'schema':1,'scope':__doc__,'capture_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),
  'basis_sha256':hashlib.sha256((HERE/'basis-native-existing-fx-command.py').read_bytes()).hexdigest(),
  'null_view_adapter_sha256':hashlib.sha256((HERE/'native_existing_fx_null_view.py').read_bytes()).hexdigest(),
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,
  'instructions':[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())]}
 path=HERE/'native-actual-stored-creation-fx.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
