"""520250 calls REAL52FD00 on current Move target; both PEs/allPC.

GTID generation/pointer/flags/self checks execute without callback seams.
Positive foreign live target stops BEFORE4CEDE0; no combat predicate succeeds
by substitution. NULL ground target therefore completes the native state getter
even when its actual ZOC slot4 pointer is nonNULL.
"""
from pathlib import Path
import argparse,json,struct
import native_company_actor_state as a
import native_regiment_stop as s
from unicorn.x86_const import *
HERE=Path(__file__).resolve().parent;TARGET=a.ACTOR+0x6000
class Probe(a.Probe):
 def hook(self,u,pc,size,arg):
  if pc==0x4cede0:self.visited[pc]=bytes(u.mem_read(pc,size));self.cut=True;u.emu_stop();return
  s.Probe.hook(self,u,pc,size,arg)
 def run(self,mode,flags,command):
  self.reset(0);raw=bytearray(s.RAW)
  for state in s.LAYOUT:struct.pack_into('<I',raw,state['offset']+4,s.MIND)
  self.u.mem_write(s.MIND,bytes(raw));self.w(s.MIND+4,a.ACTOR)
  self.w(a.ACTOR+0x14c,9);self.w(a.ACTOR+0x148,a.VECTOR);self.w(a.VECTOR+16,a.ZONE)
  self.w(s.MIND+0x20,0 if command is None else a.COMMAND)
  if command is not None:self.w(a.COMMAND+8,a.DEFINITION);self.w(a.DEFINITION+0xc,command)
  self.w(0xb364c0,a.REGISTRY)
  self.u.mem_write(a.REGISTRY+4,b'\x00\x00\x01\x00\x01\x00')
  self.w(a.REGISTRY+0x20004,0);self.w(a.REGISTRY+0x20008,a.ACTOR);self.w(a.REGISTRY+0x2000c,TARGET if mode!='empty'else 0)
  self.w(a.ACTOR+8,flags);self.w(TARGET+8,flags)
  self.w(s.MIND+0x78,{'ground':0,'empty':0x10002,'stale':0x20002,'self':0x10001,'foreign':0x10002}[mode])
  self.cut=False;self.u.reg_write(UC_X86_REG_ECX,s.MIND+0x68)
  self.u.emu_start(0x520250,self.STOP,count=1000)
  if self.cut:return {'required':'4CEDE0'}
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  return {'index':self.u.reg_read(UC_X86_REG_EAX)}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');args=ap.parse_args();machines=[];pins={}
 for image,label in [(s.P,'original'),(s.V,'v14')]:
  for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
   m=Probe(image,cw);rows=[]
   for mode in ['ground','empty','stale','self','foreign']:
    for flags in [0,1,0x20000000,0x20000001,0x80000000,0xffffffff]:
     for command in [None,0,5,7]:rows.append({'mode':mode,'flags':flags,'command_type':command,'output':m.run(mode,flags,command)})
   machines.append({'image':label,'precision':pc,'rows':rows});pins.update(m.visited)
 for m in machines[1:]:assert m['rows']==machines[0]['rows']
 for address,raw in pins.items():assert raw==s.P.range(address,len(raw))==s.V.range(address,len(raw))
 doc={'scope':__doc__,'producer_sha256':s.H(Path(__file__).read_bytes()),'original_sha256':s.H(s.P.b),'v14_sha256':s.H(s.V.b),'machines':machines,'executed_instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(pins.items())]}
 p=HERE/'native-company-target.json'
 if args.write:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':720,'pins':len(pins),'sha256':s.H(p.read_bytes())}))
if __name__=='__main__':main()
