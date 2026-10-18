"""527E30 ground command copies/defaults, stopping BEFORE520440's body.

No invoked command child is replaced by success. Both PE/allPC executions also
observe the freshly mapped process PathLog byte0 and immutable argument-default
words. This is a fresh-process observation, NOT a per-command reset or a proof
that later native Solve/writers leave PathLog unchanged.
"""
from pathlib import Path
import argparse,json
import native_regiment_stop as s
from unicorn.x86_const import *
HERE=Path(__file__).resolve().parent;COMMAND=s.MIND+0x3000
class Probe(s.Probe):
 def __init__(self,image,control):super().__init__(image,control);self.args=None
 def hook(self,u,pc,size,arg):
  if pc==0x520440:
   self.visited[pc]=bytes(u.mem_read(pc,size));sp=u.reg_read(UC_X86_REG_ESP)
   assert u.reg_read(UC_X86_REG_ECX)==s.MIND+0x68
   self.args=[self.r(sp+i)for i in range(4,28,4)];u.emu_stop();return
  super().hook(u,pc,size,arg)
 def run(self,flags,words):
  self.reset(0);self.w(COMMAND+0xc,flags)
  for i,v in enumerate(words):self.w(COMMAND+0x18+i*4,v)
  self.w(self.STACK+4,COMMAND);self.u.reg_write(UC_X86_REG_ECX,s.MIND);self.args=None
  self.u.emu_start(0x527e30,self.STOP,count=500);assert self.args is not None
  return self.args
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');a=ap.parse_args();machines=[];pins={}
 for image,label in [(s.P,'original'),(s.V,'v14')]:
  for pc,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
   m=Probe(image,cw);rows=[]
   for flags in [0,8,16,24,0x5c,0xfffffffe]:
    for words in [[0x41200000,0xc1200000,0,0,0],[0x7f812345,0xff812345,0x7f812346,0xff812346,0x7f812347],[0x80000000,0,0x7fc12345,0xffc12345,0xbf800000]]:
     rows.append({'flags':flags,'words':words,'args':m.run(flags,words)})
   machines.append({'image':label,'precision':pc,'fresh_path_log':m.u.mem_read(0xb3d43c,1)[0],
    'default_auxiliary':[m.r(0xb279c0),m.r(0xb279c4)],'default_speed':m.r(0x9f32dc),'rows':rows});pins.update(m.visited)
 for m in machines[1:]:
  for k in ['fresh_path_log','default_auxiliary','default_speed','rows']:assert m[k]==machines[0][k]
 assert machines[0]['fresh_path_log']==0 and machines[0]['default_auxiliary']==[0,0]and machines[0]['default_speed']==0
 for address,raw in pins.items():assert raw==s.P.range(address,len(raw))==s.V.range(address,len(raw))
 doc={'scope':__doc__,'producer_sha256':s.H(Path(__file__).read_bytes()),'original_sha256':s.H(s.P.b),'v14_sha256':s.H(s.V.b),'machines':machines,'executed_instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(pins.items())]}
 p=HERE/'native-company-arguments.json'
 if a.write:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':108,'pins':len(pins),'sha256':s.H(p.read_bytes())}))
if __name__=='__main__':main()
