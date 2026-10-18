"""Actual5AFB00→857840(0) event construction and8573A0 signaling.

Inherited mutex constructor, CString provider and Win32 CreateEvent/SetEvent
are explicit host transports. The Win32 event is auto-reset/initially clear;
native Signal+4 is deliberately uninitialized until SetEvent writes its byte.
This is not a native resource-worker bootstrap/waiter proof.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
sha=lambda b:hashlib.sha256(b).hexdigest()
def run(image,pc,poison):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 queue=m.alloc(0x2c);m.u.mem_write(queue,bytes([poison])*0x2c)
 manager=m.alloc(4);vt=m.alloc(16);header=m.alloc(0x14)
 m.w(manager,vt);m.w(header+0xc,2);m.w(vt+0xc,m.stub(lambda _:m.ret(header)))
 m.seam(0x856370,lambda _:m.ret(manager))
 m.seam(0x857d60,lambda _:m.ret(m.ecx()))
 calls=[];signalled=[False]
 def create(_):
  args=[m.arg(i)for i in range(1,5)];assert args==[0,0,0,0],args
  calls.append(['CreateEvent',args]);signalled[0]=False;m.ret(0x1234,16)
 def signal(_):
  assert m.arg(1)==0x1234;calls.append(['SetEvent',m.arg(1)]);signalled[0]=True;m.ret(1,4)
 m.w(0xb36408,m.stub(create));m.w(0x9f00f4,m.stub(signal))
 m.call(0x5afb00,queue)
 constructed=dict(head=m.r(queue+0x24),tail=m.r(queue+0x28),event_present=m.r(queue+0x1c)!=0,signal_byte=m.u.mem_read(queue+0x20,1)[0],event_signalled=signalled[0])
 assert constructed['signal_byte']==poison
 m.call(0x8573a0,queue+0x1c);m.call(0x8573a0,queue+0x1c)
 return dict(constructed=constructed,after=dict(signal_byte=m.u.mem_read(queue+0x20,1)[0],event_signalled=signalled[0]),calls=calls),m.visited
def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');a=p.parse_args();rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for poison in [0x5a,0xcc]:
   pair=[]
   for image in [original,modified]:
    out,visited=run(image,pc,poison);pair.append(out)
    for address,raw in visited.items():assert raw==original.range(address,len(raw))==modified.range(address,len(raw));pins[address]=raw
   assert pair[0]==pair[1];rows.append(dict(precision=pc,poison=poison,native=pair[0]))
 out=dict(scope=__doc__,rows=rows,pins=[dict(address=hex(a),bytes=b.hex())for a,b in sorted(pins.items())]);path=HERE/'native-queue-signal.json'
 if a.create:
  with path.open('x',encoding='utf-8')as f:json.dump(out,f,indent=2);f.write('\n')
 else:assert out==json.loads(path.read_text())
 print(json.dumps(dict(paired_rows=len(rows),separate_executions=len(rows)*2,pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
