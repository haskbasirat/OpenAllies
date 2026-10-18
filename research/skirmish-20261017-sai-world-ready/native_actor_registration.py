"""Full native7A0390/79F6E0/7A0C20 complete non-MARKER category3/4 paths.

Fresh original/v14 machines, actual Actor/Kingdom vtable owner getters and
classification/setter bodies. No child/allocation seams. Ordinary category
0/1/2, MARKER and Flare retirement are outside this fixture's complete paths.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
H=lambda b:hashlib.sha256(b).hexdigest()
O=h.P;V=h.V

def run(c,image):
 prior=h.P;h.P=O if image=='original'else V
 try:m=h.Machine(c['precision'])
 finally:h.P=prior
 h.VISITED.clear()
 manager=m.alloc(0xb0);actor=m.alloc(0x300);data=m.alloc(0x500);category=m.alloc(0x20)
 m.w32(actor,c['vtable']);m.w32(actor+4,data);m.w32(actor+8,c['flags8'])
 m.w32(actor+0x140,c['previous']);m.w32(actor+0xe8,c['owner'])
 m.w32(actor+0x84,int(not c['class_three']))
 m.w32(actor+0xc8,0xdead0001 if c['flare']else 0)
 m.w32(data+0x18,category);m.w32(category+0xc,2);m.w32(data+0x88,c['model88'])
 m.w8(manager+0x20,c['ready'])
 if c['operation']=='register':m.call(0x7a0390,manager,[actor,c['requested'],0]);pop=16
 elif c['operation']=='unregister':m.call(0x79f6e0,manager,[actor,0]);pop=12
 else:m.call(0x7a0c20,manager,[actor]);pop=8
 assert m.final_esp==h.STACK+pop and not m.events
 return dict(category=m.r32(actor+0x140),flags8=m.r32(actor+8)),dict(h.VISITED)

def cases():
 for pc in h.PRECISION:
  for vt in [0xa273f0,0xa299d8]:
   base=dict(precision=pc,vtable=vt,flags8=0,previous=5,owner=0xdead0001,
       model88=0xdead0001,ready=True,class_three=True,flare=False,requested=0)
   for req in range(5):
    for cls,flare in [(True,False),(True,True),(False,True)]:
     yield base|dict(operation='register',requested=req,class_three=cls,flare=flare,
         flags8=0 if cls else 1<<23)
   for owner in [0,0xdead0001]:
    for model in [0,0xdead0001]:
     yield base|dict(operation='unregister',previous=3,owner=owner,model88=model)
   for ready,cls,flare,previous,removed in [
       (False,False,False,0,False),(True,True,False,5,False),
       (True,True,False,3,False),(True,True,False,5,True),
       (True,False,True,5,False),(True,False,True,4,False),
       (True,False,True,3,False),(True,True,False,3,True)]:
    yield base|dict(operation='notify',ready=ready,class_three=cls,flare=flare,
        previous=previous,flags8=(0 if cls else 1<<23)|int(removed))

def main():
 rows=[];pins={}
 for c in cases():
  a,ap=run(c,'original');b,bp=run(c,'v14');assert a==b
  pins.update(ap|bp);rows.append(dict(input=c,output=a))
 for a,b in pins.items():assert O.range(a,len(bytes.fromhex(b)))==V.range(a,len(bytes.fromhex(b)))==bytes.fromhex(b)
 data=[]
 for vt in [0xa273f0,0xa299d8]:
  a=vt+0x108;assert O.u32(a)==V.u32(a)==0x4d1e80
  data.append(dict(va=hex(a),bytes=O.range(a,4).hex(),meaning='actual runtime virtual108 -> actual E8 getter'))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(O.b),
   v14_sha256=H(V.b),rows=rows,machines=2*len(rows),
   executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],intercepted_entries=[],data_pins=data,
   dependency=dict(path='research/phase8-movement/harness.py',sha256=H((ROOT/'research/phase8-movement/harness.py').read_bytes())))
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-actor-registration.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),seams=0,sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
