"""Full 766D40 header emission after real corrected CRT registration.

Only the two OS mutex virtual callbacks, allocations and CString/log transport
used during the CRT setup are seams. No descriptor ID/count is injected.
"""
from pathlib import Path
import argparse,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-orders'))
from recover_orders import OrdersMachine,registry,PE0,PE1,sha
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import *

class Machine(OrdersMachine):
 def __init__(self,image,pc):
  self.events=[];self.forbidden=[];self.header_mode=False
  super().__init__(image);self.registrations=registry(self)
  self.u.reg_write(UC_X86_REG_FPCW,pc)
  self.node=self.alloc(0x100);vt=self.alloc(16)
  self.w(self.node+8,vt);self.w(vt+4,0x2110200);self.w(vt+8,0x2110210)
  self.w(0xb36524,self.node);self.w(0xb36558,self.admin)
  self.stream=self.admin+0x254
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,user):
  if self.header_mode and any(address<=p<address+size for p in (self.admin+0x2c0,self.admin+0x210,self.admin+8)):
   self.forbidden.append(hex(address))
 def hook(self,u,pc,size,user):
  if self.header_mode and pc in (0x2110200,0x2110210):
   assert u.reg_read(UC_X86_REG_ECX)==self.node+8
   self.events.append({'kind':'lock'if pc==0x2110200 else'unlock','position':self.position(self.stream)})
   self.ret();return
  super().hook(u,pc,size,user)

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');args=ap.parse_args()
 machines=[]
 for label,image in [('original',PE0),('v14',PE1)]:
  for precision,control in [(24,0x7f),(53,0x27f),(64,0x37f)]:
   m=Machine(image,control);m.header_mode=True;m.mode='header';rows=[]
   for byte in range(3):
    for bit in range(9):
     for active in (0,1):
      start={'byte':byte,'bit':bit};raw=bytearray(32)
      raw[:byte]=bytes([0xa5])*byte;raw[byte]=0x96&((1<<bit)-1)
      packet=m.packet(raw);m.w(m.stream+4,packet);m.w(m.stream+8,byte);m.w(m.stream+12,bit)
      m.u.mem_write(m.admin+0x2c0,bytes([active]));m.w(m.admin+0x210,0x99);m.w(m.admin+8,0x1234)
      m.events=[];m.forbidden=[];m.call(0x766d40)
      assert [e['kind']for e in m.events]==['lock','unlock'] and not m.forbidden
      end=m.position(m.stream);n=(end['byte']*8+end['bit']+7)//8
      rows.append({'start':start,'prefix':list(raw[:byte+1]),'active2c0':active,'raw':list(m.u.mem_read(packet,n)),'end':end,'events':m.events})
   pins=[{'pc':hex(pc),'size':size,'sha256':sha(image.range(pc,size))}for pc,size in sorted(m.visited.items())if image.base<=pc<image.base+image.image_size]
   assert m.r(m.order_manager+4)==358 and m.r(0xb40af4)==323
   machines.append({'image':label,'precision':precision,'registry_sha256':sha(json.dumps(m.registrations,sort_keys=True).encode()),'count':m.r(m.order_manager+4),'id':m.r(0xb40af4),'rows':rows,'executed_pins':pins})
 for m in machines[1:]:assert m['rows']==machines[0]['rows'] and m['executed_pins']==machines[0]['executed_pins']
 report={'schema':1,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(PE0.b),'v14_sha256':sha(PE1.b),'machines':machines,'scope':__doc__}
 p=HERE/'native-version-ack-header.json'
 if args.write:p.write_text(json.dumps(report,indent=2)+'\n')
 else:assert json.loads(p.read_text())==report
 print(json.dumps({'machines':len(machines),'rows':sum(len(m['rows'])for m in machines),'executed_pins':len(machines[0]['executed_pins']),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
