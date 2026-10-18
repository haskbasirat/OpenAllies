"""Actual5890C0/4D1050/529C30, with observed lazy field-read order.

Controlled native Structure/Actor/Data fields, not a natural mission capture.
No intercepted child body or allocator: all calls execute unchanged.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ
P,V=h.P,h.V
def run(image,pc,case):
 h.P=image;m=h.Machine(pc);structure=m.alloc(0x40);actor=m.alloc(0x180);definition=m.alloc(0x600);wanted=m.alloc(0x600)
 m.w32(structure+4,actor);m.w32(actor+4,definition)
 m.w32(actor+0x100,(case['state25']<<25)|(case['state26']<<26)|(case['constructing']<<21),case['state36']<<4)
 def links(values):
  head=0
  for value in reversed(values):q=m.alloc(8);m.w32(q,value,head);head=q
  return head
 kind=case['match'];target=definition if kind=='direct' else wanted
 m.w32(definition+0x4c0,links([wanted]if kind=='upgrade'else[]))
 m.w32(definition+0x4d0,links([wanted]if kind=='predecessor'else[]))
 events=[]
 def code(u,address,size,data):
  label={0x5890c6:'state25',0x4d1050:'transient',0x529c30:'definition',0x589103:'constructing',0x58918d:'constructing',0x5891be:'constructing'}.get(address)
  if label:events.append(label)
 def read(u,access,address,size,value,data):
  if address==definition+0x4c0:events.append('contains4c0')
  if address==definition+0x4d0:events.append('contains4d0')
 m.u.hook_add(UC_HOOK_CODE,code);m.u.hook_add(UC_HOOK_MEM_READ,read)
 value=bool(m.call(0x5890c0,structure,[target,int(case['complete']),int(case['upgrades'])])&255)
 assert m.final_esp==h.STACK+16 and m.fpu_top()==0
 return dict(result=value,events=events)
def main():
 h.VISITED.clear();rows=[]
 for pc,lo25,lo26,hi4,building,kind,complete,upgrades in itertools.product(h.PRECISION,[0,1],[0,1],[0,1],[0,1],['direct','upgrade','predecessor','absent'],[False,True],[False,True]):
  case=dict(state25=lo25,state26=lo26,state36=hi4,constructing=building,match=kind,complete=complete,upgrades=upgrades)
  a=run(P,pc,case);b=run(V,pc,case);assert a==b
  rows.append(dict(precision=pc,input=case,output=a))
 h.P=P
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  rows=rows,pins=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())],seams=[])
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-structure-match.json'
 if '--write'in sys.argv:
  with p.open('xb')as f:f.write(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),executed_pins=len(h.VISITED),sha256=hashlib.sha256(data).hexdigest(),result='PASS')))
if __name__=='__main__':main()
