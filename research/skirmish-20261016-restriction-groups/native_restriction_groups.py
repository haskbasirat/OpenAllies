"""Run original505600/5049C0, constructors, map inserts/rehash and typed Find.

Explicit seams: source registry lookup transport (427BD0/4915F0), stream I/O,
CString allocation/storage/assignment, C-locale ASCII _wcsicmp, diagnostic log,
localization table lookup. Actual4BDBA0 and463FA0 execute. Current source lookup
transport inputs are supplied by Rust capture, not guessed from serialized IDs.
Historical profile0/1 complements are deliberately outside this current oracle.
Both PE images run, all x87 PCs (these readers have no numeric arithmetic).
"""
from pathlib import Path
import sys,struct,json,hashlib,importlib.util,argparse
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from native_header import Machine as Base,P,V,VISITED,read_chunks
sys.path.insert(0,str(ROOT/'research/phase8-economy'))
from harness import guard_null_page,check_null_accesses,MODES
from unicorn.x86_const import *

def digest(b):return hashlib.sha256(b).hexdigest()
def packstr(s):
 b=s.encode('utf-16le');return struct.pack('<H',len(b)//2)+b
def payload(groups,manager_word=0):
 out=struct.pack('<III',0,manager_word,len(groups))
 for g in groups:
  v=g.get('version',4);out+=struct.pack('<I',v)+packstr(g['name'])+struct.pack('<I',g.get('word',0))+packstr(g.get('display',''))
  for l in g['lists']:out+=struct.pack('<I',len(l))+b''.join(map(packstr,l))
 return out

class Machine(Base):
 def __init__(self,data,cw,image,lookups):
  super().__init__(data);self.uc=self.u;guard_null_page(self)
  if image=='v14':
   for vs,r,size,off in P.sections:self.u.mem_write(P.base+r,V.range(P.base+r,size))
  self.u.reg_write(UC_X86_REG_FPCW,cw);self.write(self.read(self.stream)+12,0x2e00040)
  self.write(0xb43b48,self.empty)
  self.manager=self.alloc(0x18);self.resource=self.alloc(0x40);self.write(self.resource+0x3c,self.read(self.stream))
  self.lookups=lookups;self.trace=[];self.diagnostics=[];self.definition={};self.nodes={};self.registry=self.alloc(0x40)
  self.game_data=self.alloc(0x600);self.write(0xb36530,self.game_data);self.write(self.game_data+0x464,self.registry)
  self.call(0x5042d0,[],self.manager)
 def call(self,pc,args,this):
  self.write(self.stack,self.stop)
  for i,v in enumerate(args):self.write(self.stack+4+i*4,v)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,this)
  self.u.emu_start(pc,self.stop,count=20000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop,hex(self.u.reg_read(UC_X86_REG_EIP));check_null_accesses(self)
  return self.u.reg_read(UC_X86_REG_EAX)
 def text(self,p):
  out=bytearray()
  while True:
   b=bytes(self.u.mem_read(p,2));p+=2
   if b==b'\0\0':return out.decode('utf-16le')
   out+=b
 def resolve(self,kind,ids):
  found=self.lookups.get(kind,{}).get(ids.split('\0')[0].lower()) if ids else None
  object_data = kind=='actor' and found is None and ids.lower() in self.lookups.get('object',{})
  if object_data:found=self.lookups['object'][ids.lower()]
  if found is None:return 0
  key=(kind,found,object_data)
  if key not in self.definition:
   p=self.alloc(0x60);self.write(p,0xa13318 if object_data else (0xa05540 if kind=='actor' else 0xa24198));self.write(p+8,self.string(found.encode('utf-16le')))
   if kind=='actor':
    desc=self.alloc(0x10);self.write(desc+12,19);self.write(p+0x18,desc)
   self.definition[key]=p;cell=self.alloc(4);self.write(cell,p);self.nodes[p]=cell
  return self.definition[key]
 def hook(self,u,pc,size,arg):
  if P.base<=pc<P.base+P.image_size and pc not in VISITED:
   raw=P.range(pc,size);assert raw==V.range(pc,size),hex(pc);VISITED[pc]=raw.hex()
  if pc in (0x4bdba0,0x463fa0):
   kind='actor' if pc==0x4bdba0 else 'property';ids=self.text(self.arg())
   self.trace.append([kind,ids,self.lookups.get(kind,{}).get(ids.lower()) if ids else None])
  # Original version helper runs; stream virtual0C says read, not write.
  if pc==0x8589b0:return
  if pc==0x2e00040:self.ret(value=0);return
  if pc==0x4012c0:
   h=self.arg();self.write(h+12,self.read(h+12)+1);self.ret(value=h);return
  if pc==0x401b00:
   dest=u.reg_read(UC_X86_REG_ECX);p=self.arg();self.write(dest,p);self.write(p-4,self.read(p-4)+1);self.ret(4,value=dest);return
  if pc==0x405b80:
   dest=u.reg_read(UC_X86_REG_ECX);p=self.read(self.arg());self.write(dest,p);self.write(p-4,self.read(p-4)+1);self.ret(4,value=dest);return
  if pc==0x427bd0:self.ret(4,value=self.resolve('actor',self.text(self.arg())));return
  if pc==0x4915f0:
   p=self.resolve('property',self.text(self.read(self.arg())));self.ret(4,value=self.nodes[p] if p else 0);return
  if pc==0x96cad6:
   a,b=self.text(self.arg()).lower(),self.text(self.arg(1)).lower();self.ret(value=(a>b)-(a<b));return
  if pc==0x404f60:self.diagnostics.append(self.arg(2));self.ret();return
  if pc in (0x2e00000,0x2e00010,0x2e00020,0x2e00030,0x856370,0x401330,0x41d290,0x59e520,0x4b9f40,0x6f7810,0x6f7430):
   super().hook(u,pc,size,arg)
 def values(self,at):
  out=[]
  for bucket in range(self.read(at+4)):
   p=self.read(self.read(at)+bucket*4)
   while p:
    out.append((self.text(self.read(p)),self.read(p+4)));p=self.read(p+8)
  assert len(out)==self.read(at+8)
  return out
 def run(self):
  self.call(0x505600,[self.resource],self.manager);assert self.r.at==len(self.r.b),(self.r.at,len(self.r.b))
  profiles=[]
  for name,p in self.values(self.manager+4):
   profiles.append({'name':name,'word':self.read(p),'maps':[sorted(n for n,v in self.values(p+off)) for off in (0x14,0x24,0x34,0x44)]})
  return {'bytes':self.r.at,'word':self.read(self.manager),'count':self.read(self.manager+0x14),'profiles':profiles,
          'trace_sha256':digest(json.dumps(self.trace,separators=(',',':')).encode()),'lookups':len(self.trace),'duplicate_log_lines':self.diagnostics}

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');args=ap.parse_args()
 src=json.loads((HERE/'source-lookups.json').read_text());rows=[]
 for maprec in src['maps']:
  b=read_chunks(ROOT.parent/'reconstruction/assets/original/Data/Maps'/maprec['map'])['RESTRICTION_GROUPS'][0]
  results=[]
  for image in ('original','v14'):
   for mode,cw in MODES:
    results.append(Machine(b,cw,image,maprec['lookups']).run())
  assert all(r==results[0] for r in results)
  rows.append({'map':maprec['map'],'payload_sha256':digest(b),'executions':6,'expected':results[0]})
  if len(rows)%10==0:print(json.dumps({'maps_replayed':len(rows)}),flush=True)
 tests=[{'name':'typed-aliases-duplicates-unknown','groups':[{'name':'same','word':13,'lists':[['tank','old','missing','tree',''],['tech','oldtech','missing'],['old','tank','tree'],['oldtech','tech','tech']]}]},
        {'name':'duplicate-profile-rehash','groups':[{'name':'same' if i%3==0 else 'group'+str(i),'word':i,'lists':[[],[],['tank'] if i%2 else [],[]]} for i in range(64)]},
        {'name':'rehash-changes-visible-duplicate','groups':[{'name':'same' if i in (0,3,6,9) else 'group'+str(i),'word':i,'lists':[[],[],['tank'] if i%2 else [],[]]} for i in range(40)]},
        {'name':'canonical-shadow-not-translated-twice','groups':[{'name':'shadow','lists':[['alias','old'],['aliastech','oldtech'],['alias'],['aliastech']]}]},
        {'name':'schema-two-three','groups':[{'version':2,'name':'two','lists':[[],[],['tank'],[]]},{'version':3,'name':'three','display':'#key','lists':[[],['tech'],[],['tech']]}]}]
 lookups={'actor':{'tank':'Tank','old':'Tank','alias':'Old','tree':None},'property':{'tech':'Tech','oldtech':'Tech','aliastech':'OldTech'},'object':{'tree':'Tree'}}
 synthetic=[]
 for test in tests:
  b=payload(test['groups']);results=[Machine(b,cw,image,lookups).run() for image in ('original','v14') for mode,cw in MODES]
  assert all(r==results[0] for r in results);synthetic.append({'name':test['name'],'payload_hex':b.hex(),'lookups':lookups,'expected':results[0],'executions':6})
 report={'schema':1,'scope':__doc__,'original_sha256':digest(P.b),'v14_sha256':digest(V.b),'source_lookup_receipt_sha256':digest((HERE/'source-lookups.json').read_bytes()),'maps':rows,'synthetic':synthetic,
         'caller_order_pin':{'start':'0x4FCA33','end':'0x4FCA8B','bytes':P.range(0x4fca33,0x58).hex(),'equal_v14':P.range(0x4fca33,0x58)==V.range(0x4fca33,0x58),'scope':'Static original caller: version>=8 opens restriction chunk and calls505600 before TEAMS; no whole caller execution claimed.'},
         'typed_actor_metadata':[{'address':hex(a),'bytes':P.range(a,4).hex(),'equal_v14':P.range(a,4)==V.range(a,4)} for a in (0xa05540+0x38,0xa09b30+0x38,0xa13318+0x38)],
         'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(VISITED.items())]}
 raw=(json.dumps(report,indent=2)+'\n').encode();out=HERE/'native-restriction-groups.json'
 if args.write:out.write_bytes(raw)
 else:assert out.read_bytes()==raw,'immutable fixture mismatch'
 print(json.dumps({'maps':len(rows),'synthetic':len(synthetic),'executions':6*(len(rows)+len(synthetic)),'pins':len(VISITED),'sha256':digest(raw)}))
if __name__=='__main__':main()
