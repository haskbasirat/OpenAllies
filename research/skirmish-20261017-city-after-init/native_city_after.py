"""Whole original563FE0 CityAfterInit with real529C30 getter.

CreateZone4D69A0 and Move515F40 are named mutating/cutoff leaves, not
complete zone solver proofs. Raw source radius and position reads execute.
Each PE/all3PC runs independently. Isolated probes mutate SAME owner fields
at CreateZone to prove post-call reads. No arbitrary cross-owner retargeting.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn.x86_const import UC_X86_REG_EIP
sha=lambda b:hashlib.sha256(b).hexdigest()

def run(image,pc,c):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 component,actor,definition,slots,zone=[m.alloc(n)for n in [0x24,0x200,0x600,36,0x18]]
 m.w(component+4,actor);m.w(actor+4,definition);m.w(definition+0x5b4,c['radius'])
 m.w(actor+0x20,c['xy'][0]);m.w(actor+0x24,c['xy'][1]);m.w(actor+0x148,slots)
 m.w(actor+0x14c,9);m.w(slots+32,0)
 events=[];terminal=['complete'];allocated=[False]
 def event(label,data):
  events.append([label,data])
  if c.get('stop')==label:terminal[0]=label;m.u.emu_stop();return False
  return True
 def create(_):
  assert m.ecx()==actor and m.arg(1)==8
  if not event('create',8):return
  if c['create']&255:
   m.w(slots+32,zone);allocated[0]=True
  if c.get('mutate'):
   m.w(definition+0x5b4,c['mutate']['radius']);m.w(actor+0x20,c['mutate']['xy'][0]);m.w(actor+0x24,c['mutate']['xy'][1])
  m.ret(c['create'],pop=4)
 def radius(_):event('radius',m.r(definition+0x5b4))
 def position(_):event('position',[m.r(actor+0x20),m.r(actor+0x24)])
 def move(_):
  assert m.ecx()==zone
  if not event('move',[m.arg(1),m.arg(2),m.arg(3)]):return
  m.ret(pop=12)
 m.seam(0x4d69a0,create);m.observe(0x563ff9,radius);m.observe(0x564003,position);m.seam(0x515f40,move)
 try:m.call(0x563fe0,component,[])
 except AssertionError:
  if terminal[0]=='complete':raise
 if terminal[0]!='complete':assert m.u.reg_read(UC_X86_REG_EIP) in [0x4d69a0,0x563ff9,0x564003,0x515f40]
 return dict(events=events,terminal=terminal[0],zone_allocated=allocated[0]),m.visited

def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');args=p.parse_args()
 cases=[]
 for result in [0,256,1,255]:cases.append(dict(name='create_'+str(result),create=result,radius=0x41a00000,xy=[0x42c80000,0x43480000]))
 for radius in [0,0x80000000,0x7fc12345,0x7f800000]:cases.append(dict(name='raw_radius_'+hex(radius),create=1,radius=radius,xy=[0x3f800000,0xc0000000]))
 base=dict(name='same_owner_mutation',create=1,radius=0x41200000,xy=[0x3f800000,0x40000000],mutate=dict(radius=0x42700000,xy=[0x43160000,0x43960000]))
 cases.append(base)
 for stop in ['create','radius','position','move']:cases.append(dict(base,name='stop_'+stop,stop=stop))
 rows=[];pins={}
 for image_name,image in [('original',original),('v14',modified)]:
  for pc in ['Single24','Double53','Extended64']:
   for c in cases:
    result,visited=run(image,pc,c);pins.update(visited);rows.append(dict(image=image_name,precision=pc,input=c,native=result))
 out=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),rows=rows,instructions=[])
 for a,b in sorted(pins.items()):
  assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
  out['instructions'].append(dict(address=hex(a),bytes=b.hex()))
 path=HERE/'native-city-after.json'
 if args.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(rows=len(rows),pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
