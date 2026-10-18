"""Full native4D1350 with absent Organization and real Element571640/publication.

Actor, Element and native virtual getters are explicit fixture storage. No
Organization or Mind is fabricated. Positive external UI/renderer services
stop at their reached boundary, retaining the actual action-mask prefix.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE
ORIGINAL=h.P
class Required(Exception):pass
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def run(c,pc,image):
 h.P=image
 m=h.Machine(pc)
 actor,definition,vt=[m.alloc(n)for n in [0x200,0x600,0x200]]
 m.w32(actor,vt);m.w32(actor+4,definition);m.w32(actor+8,c['flags'])
 m.w32(vt+0x108,0x4d1e80)
 m.w32(actor+0xf0,c['actions']&0xffffffff,c['actions']>>32)
 m.w32(actor+0x100,c['state']&0xffffffff,c['state']>>32)
 m.w32(actor+0xe8,m.alloc(0x400)if c['owner']else 0);m.w8(actor+0x4c,c['object4c'])
 m.w32(actor+0xc,m.alloc(0x80)if c['renderer']else 0)
 if c['element']:
  element=m.alloc(0x40);m.w32(actor+0x80,element);m.w32(element+4,actor)
  m.w32(element+0x14,0x77777777 if c['definition']else 0);m.w8(element+0x1c,c['ready'])
 events=[]
 def observe(u,entry,size,_):
  if entry==0x76c7f0:raise Required('owner_views_required')
  if entry==0x4cfd0e:raise Required('renderer_required')
  if entry==0x57b390:raise AssertionError('fabricated Organization contributor reached')
  if entry in [0x4d1350,0x571640,0x4cfcc0]:events.append(hex(entry))
 m.u.hook_add(UC_HOOK_CODE,observe)
 outcome='ok'
 try:m.call(0x4d1350,actor)
 except Required as e:outcome=str(e)
 if outcome=='ok':assert m.final_esp==h.STACK+4 and m.fpu_top()==0
 assert m.r32(actor+0x7c)==0
 return dict(outcome=outcome,actions=m.r32(actor+0xf0)|(m.r32(actor+0xf4)<<32),events=events)
def inputs():
 for element,definition,ready,flags,state in itertools.product([False,True],[False,True],[False,True],[0,1,0x80000000],[0,1<<12,1<<27]):
  if not element and(definition or ready):continue
  yield dict(element=element,definition=definition,ready=ready,flags=flags,state=state,actions=0x123456789a,owner=False,object4c=0,renderer=False)
 base=dict(element=True,definition=True,ready=True,flags=0x80000000,state=1<<12,actions=0x123456789a,owner=False,object4c=0,renderer=False)
 for c in [dict(owner=True,object4c=1),dict(owner=True,object4c=0x80),dict(renderer=True)]:yield {**base,**c}
def main():
 rows=[]
 try:
  for pc in h.PRECISION:
   for c in inputs():
    a=run(c,pc,ORIGINAL);b=run(c,pc,h.V);assert a==b
    rows.append(dict(precision=pc,input=c,output=a))
 finally:h.P=ORIGINAL
 boundaries={0x76c7f0,0x4cfd0e}
 assert boundaries.issubset(h.VISITED)
 # Unicorn code hooks run before the instruction. These exact boundary
 # entries were observed but stopped, so they are not executed-byte pins.
 pins=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())if a not in boundaries]
 stopped=[dict(va=hex(a),bytes=h.VISITED[a])for a in sorted(boundaries)]
 assert {0x4d1350,0x571640,0x4cfcc0,0x4d1e80}.issubset(h.VISITED)
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  producer_sha256=sha(Path(__file__)),cases=rows,executed_address_byte_pins=pins,
  required_boundary_entry_pins=stopped,successful_intercepted_bodies=[])
 p=HERE/'native-held-actor-actions.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),sha256=sha(p),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
