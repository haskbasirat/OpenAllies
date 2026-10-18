"""Real520250 current ZOC guard and command-type return, both PEs/allPC.

Positive ZOC slot4 POINTER cases stop BEFORE52FD00 with the actual full-generation
target lookup completed; no target/combat predicate result is supplied. Other
cases run the whole native return. No tested graph child is stubbed success.
"""
from pathlib import Path
import argparse,json,struct
import native_regiment_stop as s
from unicorn.x86_const import *
HERE=Path(__file__).resolve().parent
ACTOR=s.MIND+0x2000;VECTOR=ACTOR+0x1000;ZONE=VECTOR+0x1000;COMMAND=ZONE+0x1000;DEFINITION=COMMAND+0x1000;REGISTRY=s.MIND+0x100000
class Probe(s.Probe):
 def hook(self,u,pc,size,arg):
  if pc==0x52fd00:self.visited[pc]=bytes(u.mem_read(pc,size));self.cut=True;u.emu_stop();return
  super().hook(u,pc,size,arg)
 def run(self,length,zone,command):
  self.reset(0);self.cut=False;raw=bytearray(s.RAW)
  for state in s.LAYOUT:struct.pack_into('<I',raw,state['offset']+4,s.MIND)
  self.u.mem_write(s.MIND,bytes(raw));self.w(s.MIND+4,ACTOR);self.w(ACTOR+0x14c,length);self.w(ACTOR+0x148,VECTOR)
  self.w(VECTOR+16,ZONE if zone is not None else 0)
  if zone is not None:self.w(ZONE+0x10,zone&0xffffffff)
  self.w(s.MIND+0x20,0 if command is None else COMMAND)
  if command is not None:self.w(COMMAND+8,DEFINITION);self.w(DEFINITION+0xc,command)
  self.w(0xb364c0,REGISTRY);self.u.reg_write(UC_X86_REG_ECX,s.MIND+0x68)
  self.u.emu_start(0x520250,self.STOP,count=1000)
  if self.cut:return {'required':'52FD00'}
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  return {'index':self.u.reg_read(UC_X86_REG_EAX)}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');a=ap.parse_args();machines=[];pins={}
 for image,label in [(s.P,'original'),(s.V,'v14')]:
  for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
   m=Probe(image,cw);rows=[]
   for length,zone in [(0,None),(4,None),(5,None),(5,0),(9,-1),(9,1)]:
    for command in [None,0,5,6,7,0xffffffff]:rows.append({'length':length,'zone_y':zone,'command_type':command,'output':m.run(length,zone,command)})
   machines.append({'image':label,'precision':pc,'rows':rows});pins.update(m.visited)
 for m in machines[1:]:assert m['rows']==machines[0]['rows']
 for address,raw in pins.items():assert raw==s.P.range(address,len(raw))==s.V.range(address,len(raw))
 doc={'scope':__doc__,'producer_sha256':s.H(Path(__file__).read_bytes()),'original_sha256':s.H(s.P.b),'v14_sha256':s.H(s.V.b),'machines':machines,'executed_instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(pins.items())]}
 p=HERE/'native-company-actor-state-v2.json'
 if a.write:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':216,'pins':len(pins),'sha256':s.H(p.read_bytes())}))
if __name__=='__main__':main()
