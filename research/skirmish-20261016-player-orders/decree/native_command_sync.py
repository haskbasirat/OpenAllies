"""Original4D9A40 A-synchronizer call order/labels/raw command reads and required child stops. Logger bodies are explicit seams here; their enabled/suspend/hash behavior is separately native-backed in aa-orders::net::sync. Caller label _command is an explicit selected-order fixture."""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
P=h.P;V=h.V;H=lambda b:hashlib.sha256(b).hexdigest()
class Stop(Exception):pass
class Machine(h.Machine):
 def run(self,c):
  log=self.alloc(0x40);cmd=self.alloc(0x40);self.w32(0xb3655c,log);self.w32(cmd+0xc,c['flags']);self.w32(cmd+0x28,c['value']);label=self.alloc(32);self.u.mem_write(label,'_command\0'.encode('utf-16le'));events=[]
  def leaf(m,name,pop):
   assert m.ecx()==log;address=m.arg();raw=bytearray()
   while m.u.mem_read(address,2)!=b'\0\0':raw.extend(m.u.mem_read(address,2));address+=2
   value=0 if name=='marker'else m.arg(1);assert m.arg(1 if name=='marker'else 2)==0
   events.append({'kind':name,'label':raw.decode('utf-16le'),'value':value})
   if c['stop']==name:raise Stop(name)
   m.ret(pop)
  self.seams[0x77dc50]=lambda m:leaf(m,'marker',8);self.seams[0x77d930]=lambda m:leaf(m,'value',12);self.seams[0x77da70]=lambda m:leaf(m,'flags',12)
  status='complete'
  try:self.call(0x4d9a40,0,[label,cmd if c['command']else 0]);assert self.final_esp==h.STACK+4 and self.fpu_top()==0
  except Stop as e:status=str(e)
  return{'status':status,'events':events}
def main():
 cases=[dict(command=cmd,value=v,flags=f,stop=s)for cmd,v,f,s in itertools.product([False,True],[0,0x7fc12345,0xffffffff],[0,0x5c,0x7ff],[None,'marker','value','flags'])];rows=[]
 for image,label in[(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for c in cases:rows.append({'image':label,'precision':pc,'input':c,'output':Machine(pc).run(c)})
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 p=HERE/'native-command-sync.json';p.open('x').write(json.dumps({'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]},indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()

