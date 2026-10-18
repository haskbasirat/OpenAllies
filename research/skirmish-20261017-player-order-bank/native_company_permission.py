"""Real51D730 Move/Kill permission,5247C0,529FC0 and52A000 in both PEs/allPC.

Every Regiment metadata initializer is executed first. Authored command/stack/
Organization inputs expose exact lazy definition/Organization/head reads. There
are no successful predicate callbacks or intercepted instruction bodies in the
tested graph; inherited CString/atexit seams apply only to CRT metadata setup.
"""
from pathlib import Path
import argparse,json,struct
import native_regiment_stop as s
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import *
HERE=Path(__file__).resolve().parent
ACTOR=s.MIND+0x2000;ORG=ACTOR+0x1000;COMMAND=ORG+0x1000;DEFINITION=COMMAND+0x1000
class Probe(s.Probe):
 def __init__(self,image,control):
  super().__init__(image,control);self.reads=None
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,arg):
  if self.reads is not None:
   for name,p,n in [('command',COMMAND+0xc,4),('definition',DEFINITION+0xc,4),('organization',ORG+0x8c,1),('stack',s.MIND+0x10,4)]:
    if address<p+n and address+size>p:self.reads.append(name)
 def predicate(self,current,engage,override,command):
  self.reset(0);raw=bytearray(s.RAW)
  for state in s.LAYOUT:struct.pack_into('<I',raw,state['offset']+4,s.MIND)
  self.u.mem_write(s.MIND,bytes(raw));self.w(s.MIND+4,ACTOR);self.w(ACTOR+0x7c,ORG)
  self.u.mem_write(ORG+0x8c,bytes([override]));self.w(s.MIND+0x1d8,engage)
  self.w(s.MIND+0x20,0 if command is None else COMMAND)
  if command is not None:self.w(COMMAND+8,DEFINITION);self.w(COMMAND+0xc,command[0]);self.w(DEFINITION+0xc,command[1])
  self.w(s.MIND+0x10,0 if current=='empty' else s.NODE)
  if current!='empty':self.w(s.NODE,0 if current=='null' else s.MIND+current)
  self.u.reg_write(UC_X86_REG_ECX,s.MIND+0x68);self.reads=[]
  self.u.emu_start(0x51d730,self.STOP,count=1000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  reads=self.reads;self.reads=None
  return {'allowed':bool(self.u.reg_read(UC_X86_REG_EAX)&255),'reads':reads}
def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');a=ap.parse_args();machines=[];pins={}
 for image,label in [(s.P,'original'),(s.V,'v14')]:
  for pc,control in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
   m=Probe(image,control)
   for item in s.LAYOUT:m.address=int(item['metadata_va'],16);m.run_metadata()
   rows=[]
   for cur in ['empty','null']+[v['offset']for v in s.LAYOUT]:
    for engage,override in [(0,0),(1,128),(2,0),(2,1),(2,128),(3,0)]:
     for cmd in [None,[0,7],[64,6],[64,7],[0xffffffff,7]]:
      rows.append({'current':cur,'engage_status':engage,'organization_override':override,'command':cmd,'output':m.predicate(cur,engage,override,cmd)})
   machines.append({'image':label,'precision':pc,'rows':rows});pins.update(m.visited)
 for m in machines[1:]:assert m['rows']==machines[0]['rows']
 for address,raw in pins.items():assert raw==s.P.range(address,len(raw))==s.V.range(address,len(raw))
 doc={'scope':__doc__,'producer_sha256':s.H(Path(__file__).read_bytes()),'original_sha256':s.H(s.P.b),'v14_sha256':s.H(s.V.b),'constructor_fixture_sha256':s.H(s.SOURCE.read_bytes()),'machines':machines,'executed_instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(pins.items())]}
 p=HERE/'native-company-permission.json'
 if a.write:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':sum(len(m['rows'])for m in machines),'pins':len(pins),'sha256':s.H(p.read_bytes())}))
if __name__=='__main__':main()
