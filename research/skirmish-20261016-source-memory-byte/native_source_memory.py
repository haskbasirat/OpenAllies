"""ObjectData16D constructor store and full49C460 verifier, no body seams.

The verifier derives this byte from type2 ActorData+4E0 required-object
references. It is not an authored announced boolean. Actual1084-entry source
censuses supply null4E0 pointers at all3PC; nonempty/Upgrade source graphs remain
outside the guarded Rust producer. Constructor execution is a bounded store
interval with explicit native register inputs, not the full TLS/heap constructor.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX
H=lambda b:hashlib.sha256(b).hexdigest()
VTABLES={'ObjectData':0xa13318,'ActorData':0xa05540}

def run(c,pc,image):
 m=h.Machine(pc)
 if image is h.V:
  old=(h.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
  if new>old:m.u.mem_map(image.base+old,new-old)
  m.u.mem_write(image.base,image.b[:4096])
  for vs,r,n,o in image.sections:
   if n:m.u.mem_write(image.base+r,image.b[o:o+min(n,max(vs,n))])
 target,other,game,manager,heads=[m.alloc(n)for n in [0x600,0x600,0x40,0x80,0x80]]
 m.w8(target+0x16d,0xa5)
 m.u.reg_write(UC_X86_REG_ESI,target);m.u.reg_write(UC_X86_REG_EBX,0)
 m.u.emu_start(0x49c970,0x49c976,count=1)
 assert m.r8(target+0x16d)==0
 m.w32(target,VTABLES[c['kind']]);m.w8(target+0x16d,c['initial'])
 m.w32(0xb36530,game);m.w32(game+0xc,manager);m.w32(manager+0x68,heads)
 nodes=[m.alloc(12)for _ in c['references']]
 for i,ref in enumerate(c['references']):
  actor=m.alloc(0x500)
  m.w32(actor+0x4e0,0 if ref is None else target if ref=='target'else other)
  m.w32(nodes[i],actor,0,nodes[i+1]if i+1<len(nodes)else 0)
 m.w32(heads+8,nodes[0]if nodes else 0)
 m.call(0x49c460,target)
 actual=m.r8(target+0x16d);expected=int(bool(c['initial']or'target'in c['references']))
 assert actual==expected and m.final_esp==h.STACK+4 and m.fpu_top()==0
 return dict(byte16d=actual,constructor_interval_byte16d=0)

def main():
 rows=[]
 for pc in h.PRECISION:
  for kind,initial,refs in itertools.product(VTABLES,[0,1],[[],[None],['other'],['target'],[None,'other',None],['other','target','other'],['target','target']]):
   c=dict(kind=kind,initial=initial,references=refs);a=run(c,pc,h.P);b=run(c,pc,h.V);assert a==b
   rows.append(dict(precision=pc,input=c,output=a))
 actual=ROOT/'research/skirmish-20261016-live-placement/source-requirements/actual-source-requirements.json'
 actual_rows=[]
 for case in json.loads(actual.read_text())['cases']:
  values=case['values'];assert len(values)==1084 and all(v['required4E0']is None for v in values)
  assert case['receipt']['upgrade_census']['explicit_instances']==[]
  c=dict(kind='ObjectData',initial=0,references=[None]*len(values))
  a=run(c,case['source_precision'],h.P);b=run(c,case['source_precision'],h.V);assert a==b
  actual_rows.append(dict(precision=case['source_precision'],actor_count=len(values),random_actor_count=case['receipt']['registered_random_actors'],output=a))
 pins=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())]
 assert not any(int(p['va'],16)in h.ALLOCATORS for p in pins)
 data=[]
 for kind,vt in VTABLES.items():
  a=vt+0x38;raw=h.P.range(a,4);assert raw==h.V.range(a,4)
  data.append(dict(kind=kind,va=hex(a),bytes=raw.hex(),target=hex(h.P.u32(a))))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),source_input_sha256=H(actual.read_bytes()),original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,cases=rows,actual_rows=actual_rows,machines=2*(len(rows)+len(actual_rows)),executed_pins=pins,data_pins=data,seams={})
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-source-memory.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),actual_rows=len(actual_rows),machines=doc['machines'],executed_pins=len(pins),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
