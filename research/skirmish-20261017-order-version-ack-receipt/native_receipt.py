"""Actual registered766CF0 -> complete751FC0 receipt, both PEs/all PC.

Actual CRT registration provides the handler/ordinal.8569A0 getter and6C
store execute.7515A0 client-description and404F60 diagnostic are named callback
boundaries, with successful exact-value CString transport or explicit cutoff.
No actual client-description/color-text producer or logger completion claim.
"""
from pathlib import Path
import argparse,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-orders'))
from recover_orders import OrdersMachine,registry,PE0,PE1,sha
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *

class Machine(OrdersMachine):
 def __init__(self,image,control):
  self.receiving=False;self.receipt_events=[]
  super().__init__(image);registry(self)
  row=next(r for r in self.registrations if r['descriptor']=='0xb40af0')
  assert row['id']==323 and row['handler']=='0x766cf0'
  self.handler=self.r(0xb40af0);self.w(0xb36558,self.admin)
  self.player=self.alloc(0x80);self.client=self.alloc(0x100)
  self.u.reg_write(UC_X86_REG_FPCW,control)
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.write)
 def read(self,u,access,address,size,value,user):
  if not self.receiving:return
  if address==self.admin+0x284:self.receipt_events.append(['player',bool(self.r(address))])
  if address==self.player+4:self.receipt_events.append(['client',bool(self.r(address))])
  assert not(self.reader<=address<self.reader+16),'receipt must not read wire payload'
 def write(self,u,access,address,size,value,user):
  if self.receiving and address==self.client+0x6c:
   self.receipt_events.append(['store',value]);assert size==1 and value==1
 def hook(self,u,pc,size,user):
  if self.receiving and pc==0x7515a0:
   self.visited[pc]=size;self.receipt_events.append(['describe'])
   if self.fail=='describe':self.cutoff='describe';u.emu_stop();return
   dst=self.arg();self.w(dst,self.string('[actual-client,7]'));self.ret(4,dst);return
  if self.receiving and pc==0x404f60:
   self.visited[pc]=size
   line=self.arg(2);text=self.text(self.arg(5))if line==0xb52 else None
   self.receipt_events.append(['diagnostic',line,text])
   if self.fail=='diagnostic':self.cutoff='diagnostic';u.emu_stop();return
   self.ret();return
  super().hook(u,pc,size,user)

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');a=ap.parse_args();machines=[]
 for label,image in [('original',PE0),('v14',PE1)]:
  for precision,control in [(24,0x7f),(53,0x27f),(64,0x37f)]:
   m=Machine(image,control);m.mode='receipt';m.receiving=True;rows=[]
   for presence in (0,1,2):
    for old in (0,1,128,255):
     for fail in (None,'describe','diagnostic'):
      m.w(0,0);m.w(m.admin+0x284,m.player if presence else 0)
      m.w(m.player+4,m.client if presence==2 else 0);m.u.mem_write(m.client+0x6c,bytes([old]))
      m.w(m.reader+8,2);m.w(m.reader+12,5);m.receipt_events=[];m.fail=fail;m.cutoff=None
      try:m.call(m.handler,[m.reader])
      except AssertionError:
       assert m.cutoff is not None
      value=m.u.mem_read(m.client+0x6c,1)[0]
      assert value==(1 if presence==2 else old)
      assert m.position(m.reader)=={'byte':2,'bit':5}
      rows.append({'presence':presence,'old6c':old,'fail':fail,'cutoff':m.cutoff,'events':m.receipt_events,'new6c':value})
   pins=[{'pc':hex(pc),'size':size,'sha256':sha(image.range(pc,size))}for pc,size in sorted(m.visited.items())if image.base<=pc<image.base+image.image_size]
   machines.append({'image':label,'precision':precision,'registry_count':m.r(m.order_manager+4),'handler':hex(m.handler),'rows':rows,'executed_and_callback_entry_pins':pins})
 for m in machines[1:]:assert m['rows']==machines[0]['rows'] and m['executed_and_callback_entry_pins']==machines[0]['executed_and_callback_entry_pins']
 report={'schema':1,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(PE0.b),'v14_sha256':sha(PE1.b),'scope':__doc__,'machines':machines}
 p=HERE/'native-receipt.json'
 if a.write:p.write_text(json.dumps(report,indent=2)+'\n')
 else:assert json.loads(p.read_text())==report
 print(json.dumps({'machines':6,'rows':216,'pins':len(machines[0]['executed_and_callback_entry_pins']),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
