"""Native569E40 including real57DC40 and null-modifier45D730, no successful seams.
Explicit synthetic head-first inventories/current GTID generations and optional
Performer/Body/source fields. Both untouched images, all x87 precision modes.
"""
from pathlib import Path
import argparse,hashlib,json,random,struct,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'));import native_machine as nm
from unicorn import UC_HOOK_CODE
def bits(v):return struct.unpack('<I',struct.pack('<f',v))[0]

def run(image,pc,c):
 m=nm.Machine(pc);old=(nm.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 m.w(nm.G_IDMGR,m.generations)
 denizen=m.alloc(0xA0,0xA5);head=0;tail=0
 for i,slot in enumerate(c['slots']):
  node=m.alloc(0x14,0xA5);target=m.alloc(0x300,0xA5)if slot['target']is not None else 0
  if target:
   m.w(target+0x174,slot['target']['flags']);m.w(target+0x284,slot['target']['health']);m.w(target+0x1E4,slot['target']['value'])
  id=0
  occupant=slot['occupant']
  if occupant is not None:
   id=((2 if occupant['stale']else 1)<<16)|(i+1)
   m.u.mem_write(m.generations+4+(i+1)*2,struct.pack('<H',1))
   actor=m.alloc(0x200,0xA5);performer=0
   if occupant['value']is not None:
    performer=m.alloc(0x70,0xA5);m.w(performer+4,actor);m.w(performer+0x60,occupant['value'])
   body=0
   if occupant['health']is not None:
    body=m.alloc(0x30,0xA5);m.w(body+0x10,occupant['health'][0]);m.w(body+0x14,occupant['health'][1])
   m.w(actor+0x74,performer);m.w(actor+0x60,body);m.w(m.generations+0x20004+(i+1)*4,actor)
  m.w(node,id);m.w(node+4,target);m.w(node+8,slot['slot_value']);m.w(node+0x10,0)
  if tail:m.w(tail+0x10,node)
  else:head=node
  tail=node
 m.w(denizen+0x10,c['old_current']);m.w(denizen+0x14,c['old_maximum']);m.w(denizen+0x18,head)
 pins={}
 def observe(_,address,size,__):
  if image.base<=address<image.base+image.image_size:
   assert address not in m.seams,'no successful body interception in this domain'
   raw=image.range(address,size);assert raw==nm.P.range(address,size)==nm.V.range(address,size)
   pins[f'{address:08x}']=raw.hex()
 m.u.hook_add(UC_HOOK_CODE,observe);m.call(0x569E40,denizen,[],0)
 assert m.outcome is None and not m.null_accesses,(m.outcome,m.null_accesses,c)
 return dict(current=m.r(denizen+0x10),maximum=m.r(denizen+0x14)),pins

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 rng=random.Random(0x569e40)
 def target(flags=0x21):return dict(flags=flags,health=bits(100),value=bits(27.5))
 def slot(occ=None,t=None,value=50):return dict(occupant=occ,target=t,slot_value=bits(value))
 def actor(value=7.5,health=None,stale=False):return dict(value=None if value is None else bits(value),health=None if health is None else list(map(bits,health)),stale=stale)
 inputs=[[],[slot()],*[ [slot(t=target(f))] for f in [0,1,0x20,0x21]],
  [slot(actor(None),target())],[slot(actor(),target())],[slot(actor(7.5,[25,100]),target())],
  [slot(actor(7.5,[25,100],True),target())],
  [slot(t=target()),slot(actor(9,[12,80])),slot(actor(None),target()),slot(t=target(0x20))]]
 for _ in range(36):
  slots=[]
  for i in range(rng.randrange(1,9)):
   t=None if rng.randrange(4)==0 else dict(flags=rng.choice([0,1,0x20,0x21]),health=bits(rng.uniform(1,500)),value=bits(rng.uniform(-9,90)))
   o=None if rng.randrange(3)==0 else actor(None if rng.randrange(4)==0 else rng.uniform(-2,150),None if rng.randrange(3)==0 else [rng.uniform(0,150),rng.uniform(1,500)],rng.randrange(4)==0)
   slots.append(slot(o,t,rng.uniform(0,100)))
  inputs.append(slots)
 rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for slots in inputs:
   c=dict(slots=slots,old_current=bits(123),old_maximum=bits(456))
   a,pa=run(nm.P,pc,c);b,pb=run(nm.V,pc,c);assert a==b and pa==pb
   rows.append(dict(precision=pc,input=c,output=a));pins.update(pa)
 v=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=len(rows)*2,cases=rows,executed_instruction_pins=pins,successful_seams=[],source_images=[dict(name=n,sha256=hashlib.sha256(p.b).hexdigest())for n,p in [('original',nm.P),('v14',nm.V)]])
 path=HERE/'native-denizen-value.json'
 if args.write:
  with path.open('x')as f:json.dump(v,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_bytes())==v
 print(json.dumps(dict(paired_cases=len(rows),native_executions=len(rows)*2,executed_pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
