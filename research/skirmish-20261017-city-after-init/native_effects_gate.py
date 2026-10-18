"""Exact 54F7F0 SessionE4/World48 gate prefix, not the effects traversal.

Both native PEs/all3PC run independently. Enabled stops at54F80E before
the actor-list body; disabled stops at54F868 before the epilogue. Non3
Session values use an unmapped GWorld, proving the World read stays lazy.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn import UC_HOOK_MEM_READ
sha=lambda b:hashlib.sha256(b).hexdigest()

def run(image,pc,c):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 session=m.alloc(0x100);world=m.alloc(0x60);tile=m.alloc(0x30)
 m.w(0xb36550,session);m.w(session+0xe4,c['e4']);m.w(world+0x48,c['world48'])
 m.w(0xb36534,world if c['e4']==3 else 0xdeadc000)
 reads=[];terminal=[]
 def memory(u,access,address,size,value,data):
  if address==session+0xe4:reads.append('session_e4')
  if address==world+0x48:reads.append('world48')
 m.u.hook_add(UC_HOOK_MEM_READ,memory)
 def stop(value):
  def reached(_):terminal.append(value);m.u.emu_stop()
  return reached
 #Cutoffs replace these entries and are NOT executed instruction pins.
 m.seam(0x54f80e,stop(True));m.seam(0x54f868,stop(False))
 try:m.call(0x54f7f0,tile,[0])
 except AssertionError:
  if not terminal:raise
 assert len(terminal)==1
 assert reads==(['session_e4','world48']if c['e4']==3 else ['session_e4'])
 return dict(enabled=terminal[0],reads=reads),m.visited

def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');args=p.parse_args()
 rows=[];pins={}
 for name,image in [('original',original),('v14',modified)]:
  for pc in ['Single24','Double53','Extended64']:
   for e4 in [0,2,3,4,0xffffffff]:
    for word in [0,1,0x80000000,0xffffffff]:
     c=dict(e4=e4,world48=word);result,visited=run(image,pc,c)
     pins.update(visited);rows.append(dict(image=name,precision=pc,input=c,native=result))
 out=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),rows=rows,instructions=[])
 for a,b in sorted(pins.items()):
  assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
  out['instructions'].append(dict(address=hex(a),bytes=b.hex()))
 path=HERE/'native-effects-gate.json'
 if args.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(rows=len(rows),pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
