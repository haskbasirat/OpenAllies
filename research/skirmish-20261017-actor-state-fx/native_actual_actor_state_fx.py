"""Full native Enter-prefix compilation and all42 state scans from actual Farm input.
Source capture is canonical Rust-owned source data; original constructors and
base/derived byte code run on both images/all PC. Predicate class1 is projected
only from separately proved494F30 permanent equivalence, not guessed VFX state.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-fx-graph'));import native_fx_graph as graph
sys.path.insert(0,str(HERE));import native_actor_state_fx as scan
def main():
 source=HERE/'actual-source-input.json';captured=json.loads(source.read_text());c=captured['input'];rows=[];pins={}
 for pc in graph.nm.PRECISIONS:
  a,pa=graph.run(c,'original',pc);b,pb=graph.run(c,'v14',pc);assert a==b and a['error']is None
  observed=dict(a);observed.pop('error');observed.pop('events');assert observed==captured['compiled'],(pc,observed,captured['compiled'])
  pins.update(pa);pins.update(pb);matches=[]
  nodes=[]
  for i in a['roots']:
   n=a['nodes'][i];reference=n['references']['60'];permanent=captured['active_sources'][reference]['permanent']
   nodes.append(dict(kind='create',state=n['words']['52'],**{'class':int(bool(permanent)),'duration':bool(n['bytes']['100'])}))
  for result in captured['matches']:
   v=dict(nodes=nodes,state=result['state'],active=int(result['active']),muted=True,kind='Actor',thread_id=41,app_thread=42,
    world_time=0x3f123456,view_time=0x41234567,thing_id=2696,kingdom_id=781,
    position=[0x3f800001,0x40000002,0x40400003],global_position=[0xbf800004,0xc0000005,0xc0400006])
   x,px=scan.run(v,'original',pc);y,py=scan.run(v,'v14',pc);assert x==y
   assert x['matched']==result['matched'];assert x['command']is None
   pins.update(px);pins.update(py);matches.append(dict(input=v,output=x))
  rows.append(dict(precision=pc,compiled=a,matches=matches))
 for va,raw in pins.items():assert graph.ORIGINAL.range(va,len(raw))==graph.nm.modified.range(va,len(raw))==raw
 runs=2*len(rows)*(1+len(captured['matches']))
 doc=dict(schema=1,scope=__doc__,source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),rows=rows,native_executions=runs,instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())])
 path=HERE/'native-actual-actor-state-fx.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),scans=len(rows)*len(captured['matches']),runs=runs,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
