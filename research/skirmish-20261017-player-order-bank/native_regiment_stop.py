"""Regiment Stop51D1C0 and exact current-state51D650/529F80 predicate.

Every registered Regiment metadata CRT initializer executes separately in both
PEs/allPC; actual constructor-offset/metadata links come from the retained
native constructor fixture.51D650 executes its real529F70/529F80 callees.
51D1C0 executes real52A810 +that predicate; accepted branches stop BEFORE
52A420 lifecycle effects. Rejected Stop returns natively. No successful
activation callback, state-entry, actor action or movement effect is supplied.
CString/atexit storage seams are inherited, explicitly outside predicate scope.
"""
from pathlib import Path
import argparse,json,sys,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-mind'));import native_metadata as n
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
P=n.original;V=n.modified
SOURCE=ROOT/'research/phase7-mind/native-constructors.json'
RAW=bytes.fromhex(next(c for c in json.loads(SOURCE.read_text())['cases']if c['input']['kind']=='regiment')['output']['initialized_hex'])
LAYOUT=json.loads((ROOT/'research/phase7-mind/native-metadata.json').read_text())['state_layouts']['regiment']
MIND=0x4500000;NODE=0x4501000
class Probe(n.MetadataMachine):
 def __init__(self,image,control):
  super().__init__(int(LAYOUT[0]['metadata_va'],16));self.recording=False;self.cut=False
  if image is V:
   old=(P.image_size+0xfff)&~0xfff;new=(V.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(P.base+old,new-old)
   for vs,r,size,off in V.sections:
    if size:self.u.mem_write(V.base+r,V.b[off:off+size])
  self.u.reg_write(UC_X86_REG_FPCW,control);self.initial=self.u.context_save()
 def hook(self,u,pc,size,arg):
  if self.recording and pc==0x52a420:
   assert u.reg_read(UC_X86_REG_ECX)==MIND and self.r(u.reg_read(UC_X86_REG_ESP)+4)==MIND+0x34
   self.cut=True;u.emu_stop();return
  super().hook(u,pc,size,arg)
 def predicate(self,current):
  self.reset(0);installed=bytearray(RAW)
  for state in LAYOUT:struct.pack_into('<I',installed,state['offset']+4,MIND)
  self.u.mem_write(MIND,bytes(installed))
  if current=='empty':self.w(MIND+0x10,0)
  else:self.w(MIND+0x10,NODE);self.w(NODE,0 if current=='null'else MIND+current);self.w(NODE+4,0)
  self.u.reg_write(UC_X86_REG_ECX,MIND+0x34)
  self.u.emu_start(0x51d650,self.STOP,count=1000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  allowed=self.u.reg_read(UC_X86_REG_EAX)&255
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,MIND);self.w(self.STACK+4,0x12345678)
  self.recording=True;self.cut=False
  self.u.emu_start(0x51d1c0,self.STOP,count=1000)
  self.recording=False
  if self.cut:assert allowed
  else:
   assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
   assert not allowed and self.u.reg_read(UC_X86_REG_EAX)&255==0
  struct.pack_into('<I',installed,0x10,0 if current=='empty'else NODE)
  assert bytes(self.u.mem_read(MIND,len(RAW)))==bytes(installed)
  return {'allowed':bool(allowed),'stop_result':'required52A420'if self.cut else 'returnedFalse','metadata_priority':None if isinstance(current,str)else self.r(int(next(s for s in LAYOUT if s['offset']==current)['metadata_va'],16)+4)}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');a=ap.parse_args();machines=[];pins={}
 for image,label in [(P,'original'),(V,'v14')]:
  for pc,control in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
   m=Probe(image,control);metadata=[]
   for item in LAYOUT:
    m.address=int(item['metadata_va'],16);metadata.append({'offset':item['offset'],**m.run_metadata()})
   rows=[{'current':cur,'output':m.predicate(cur)}for cur in ['empty','null']+[s['offset']for s in LAYOUT]]
   machines.append({'image':label,'precision':pc,'metadata':metadata,'rows':rows});pins.update(m.visited)
 for m in machines[1:]:assert m['metadata']==machines[0]['metadata'] and m['rows']==machines[0]['rows']
 for address,raw in pins.items():assert raw==P.range(address,len(raw))==V.range(address,len(raw))
 doc={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'constructor_fixture_sha256':H(SOURCE.read_bytes()),'machines':machines,'executed_instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(pins.items())]}
 p=HERE/'native-regiment-stop.json'
 if a.write:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'machines':6,'metadata_rows':120,'predicate_stop_rows':132,'pins':len(pins),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
