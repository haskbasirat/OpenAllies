"""Bounded fresh Object/Doodad native4F9C10 contract in both PE images.

Reuses the declared native_fresh_creation world-subsystem transports, but
executes each original/v14 image independently. This qualifies constructor,
identity/scale, component scheduling and caller order; it does NOT certify
runtime world-service bindings or the unpublished fresh Object factory.
B279C0/C4 values below are the PE-entry fixture words, not a live-startup
capability. No claim of absence of indirect writers is made.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as h
import native_fresh_creation as f

def main():
 rows=[];original=h.P
 try:
  for image,pe in [('original',original),('v14',h.V)]:
   h.P=pe
   for pc in h.CW:
    cases=[('object',f.base(pc=pc,cls=2,requested_id=0,variant=0xffffffff,scene=False)),
      ('doodad-empty',f.base(pc=pc,cls=9,requested_id=0,variant=0xffffffff,scene=False)),
      ('doodad-both',f.base(pc=pc,cls=9,requested_id=0,variant=0xffffffff,components=[f.f32(5),f.f32(2.5)],scene=False)),
      ('random-facing',f.base(pc=pc,cls=2,flags=1<<15,requested_id=0,variant=0xffffffff,scene=False)),
      ('axis-before-random',f.base(pc=pc,cls=9,flags=(1<<15)|(1<<17),requested_id=0,components=[0,None],scene=False)),
      ('placement-reject',f.base(pc=pc,cls=2,requested_id=0,placement=2,map_mode=0,scene=False)),
      ('mode2-placement-continue',f.base(pc=pc,cls=2,requested_id=0,placement=2,map_mode=2,scene=False)),
      ('requested-id',f.base(pc=pc,cls=9,components=[0,f.f32(1)],scene=False))]
    for label,spec in cases:
     out=f.run_create(spec)
     assert out['outcome'] is None or out['outcome']['kind']!='Unexpected',(label,out['outcome'])
     rows.append(dict(image=image,precision=pc,label=label,input=spec,output=out))
 finally:h.P=original
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']
 b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 pins=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())]
 tables=[]
 for vt in [0xa2a2a0,0xa28e90]:
  for off in [0xb0,0xb4,0xe4,0xe8,0xec,0xf0,0xe0]:
   raw=original.range(vt+off,4);assert raw==h.V.range(vt+off,4)
   tables.append(dict(va=hex(vt+off),bytes=raw.hex()))
 globals=[]
 for va in [0xb279c0,0xb279c4]:
  raw=original.range(va,4);assert raw==h.V.range(va,4)
  globals.append(dict(va=hex(va),bytes=raw.hex(),scope='PE-entry native fixture only'))
 inputs=[Path(__file__),ROOT/'research/phase8-conversion/native_fresh_creation.py',ROOT/'research/phase8-conversion/native_machine.py',ROOT/'research/phase8-conversion/native_conversion.py']
 doc=dict(scope=__doc__,producers=[dict(path=str(x.relative_to(ROOT)).replace('\\','/'),sha256=hashlib.sha256(x.read_bytes()).hexdigest())for x in inputs],rows=rows,observed_pins=pins,dispatch=tables,pe_entry_globals=globals)
 doc=json.loads(json.dumps(doc)) # Canonical JSON sequence types for immutable replay.
 target=HERE/'native-fresh-object-contract-v2.json'
 if '--create'in sys.argv:
  with target.open('x')as out:json.dump(doc,out,indent=2);out.write('\n')
 else:assert json.loads(target.read_bytes())==doc
 print(json.dumps(dict(rows=len(rows),observed_pins=len(pins),dispatch=len(tables),sha256=hashlib.sha256(target.read_bytes()).hexdigest())))
if __name__=='__main__':main()
