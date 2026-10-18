"""Whole shipped kind-2 CV dependency slice, original/v14, explicit PC24/53/64.

Input fields are the hash-pinned actual-source binding receipt. Native 4BBF90,
4BB590, recursive4618B0, all45E090 arithmetic/getters and SSE2 CRT pow execute.
Allocator/free, SVar lookup (actual typed input cells), closed stream virtual,
assertion and diagnostics are explicit leaves. Prior non-CV completion passes
are omitted under the separately audited typed, nonalias dependency slice.
Fresh empty CSV and FP controls are explicit laboratory inputs, not observed
native startup/profile context or a full4BD800/source lifecycle certificate.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as h
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
sha=lambda raw:hashlib.sha256(raw).hexdigest()
INPUT=HERE/'full-source-inputs-v3.json'
LEAVES={0x6f7810,0x6f7430,0x4b2e60,0x855d30,0x404f60}
FIELDS=[0x1e4,0x30c,0x36c]
class Machine(h.Machine):
 def __init__(self,pc,image,source):
  super().__init__(pc);self.events=[];self.numeric=next(x['values'] for x in source['numeric'] if x['precision']==pc);self.actors={};self.ids={};self.nodes=[];self.stores=[]
  old=(h.original.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
  if new>old:self.u.mem_map(image.base+old,new-old)
  for _,rva,size,off in image.sections:self.u.mem_write(image.base+rva,image.b[off:off+size])
  self.u.reg_write(UC_X86_REG_MXCSR,0x1f80);self.w(0xb4b340,1);self.w(0xb3a66c,0);self.w(0xb3b4f4,0);self.wb(0xb3b4f8,0)
  table=self.alloc(0x200);self.w(0xb39ab0,table);self.cells={}
  for key,n in self.numeric.items():
   cell=self.cells[n['name']]=self.alloc(4);self.w(cell,n['bits'])
   if n['cached_pointer_offset'] is not None:self.w(table+n['cached_pointer_offset'],cell)
  # Native String variable layout; explicit supplied empty UTF16 debug filename.
  assert source['csv']['value_utf16']==[]
  text=self.alloc(0x20);self.w(text,1000);self.w(text+4,0);self.w(text+12,1000)
  cell=self.alloc(0x20);self.w(cell+0x10,text+0x10);self.cells['CombatValueCSV']=cell
  self.manager=self.alloc(0x100);self.w(0xb39f58,self.manager)
  stream_vtable=self.alloc(0x30);stream_flag=self.stub(lambda m:m.ret(0));self.w(stream_vtable+0x14,stream_flag);self.w(self.manager+0x4c,stream_vtable)
  rows=[r for r in source['rows'] if r['precision']==pc];assert len(rows)==1028
  for r in rows:
   d=self.actors[r['id']]=self.alloc(0x500);self.ids[d]=r['id']
  for r in rows:
   t=r['input']['Ok'];d=self.actors[r['id']]
   assert t['flags']&(1<<14)==0, 'Settlement extent dependency requires completed radius'
   for off,value in [(0x174,t['flags']),(0x288,(t['body'] or [0,0])[0]),(0x284,(t['body'] or [0,0])[1]),(0x228,t['speed'] or 0),(0x210,t['bias'] or 0),(0x1e4,0xbf800000)]:self.w(d+off,value)
   arr=self.alloc(4*max(1,len(t['abilities'])));self.w(d+0x1ec,arr);self.w(d+0x1f0,len(t['abilities']))
   for i,a in enumerate(t['abilities']):
    assert a['shared_reference'] is None,'shared definition source identity requires its completed canonical cell'
    local=self.alloc(0x60);ad=self.alloc(0x80);self.w(arr+4*i,local);self.w(local,ad)
    for off,key in [(0x54,'definition_flags'),(0x24,'range'),(0x38,'area'),(0x64,'power')]:self.w(ad+off,a[key])
    self.w(ad+0x3c,int(a['projectile']));self.w(local+4,a['reload']);self.w(local+0x18,a['recharge'])
    if a['animation'] is not None:
     p,q,data,elem=self.alloc(4),self.alloc(8),self.alloc(4),self.alloc(0x20)
     self.w(local+0x3c,p);self.w(local+0x40,1);self.w(p,q);self.w(q,data);self.w(q+4,1);self.w(data,elem);self.w(elem+0x10,a['animation'])
   head=0
   for _,value in reversed(t['properties']):
    node,item=self.alloc(8),self.alloc(0x30);self.w(item+0x28,value);self.w(node,item);self.w(node+4,head);head=node
   self.w(d+0x1d4,head)
   holder,layout=self.alloc(0x10),self.alloc(0x30);n=len(t['children']);indices=self.alloc(4*max(1,n));slots=self.alloc(4*max(1,n))
   for i,child in enumerate(t['children']):
    slot=n-i-1;self.w(indices+4*i,slot);self.w(slots+4*slot,self.actors[child] if child else 0)
   self.w(layout+0x28,indices);self.w(layout+0x2c,n);self.w(holder+8,layout);self.w(holder+12,slots);self.w(d+0x2f0,holder)
   head=0
   for child,count in reversed(t['weighted']):
    node=self.alloc(0x10);self.w(node,self.actors[child]);self.w(node+8,count);self.w(node+12,head);head=node
   self.w(d+0x35c,head)
  holder=self.alloc(0x10);self.w(self.manager+0x68,holder);previous=holder
  for ident in source['manager_order']:
   node=self.alloc(0x10);self.w(node,self.actors[ident]);self.w(previous+8,node);previous=node
  self.w(previous+8,0)
  self.seam(0x6f7810,lambda m:m.ret(m.alloc(max(4,m.arg(1)))))
  self.seam(0x6f7430,lambda m:m.ret());self.seam(0x4b2e60,self.lookup);self.seam(0x855d30,self.assertion);self.seam(0x404f60,self.log)
  self.observe(0x4618b0,lambda m:m.events.append(['definition',m.ids[m.ecx()]]))
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.write)
 def wstr(self,address):
  words=[]
  while True:
   word=struct.unpack('<H',self.u.mem_read(address+2*len(words),2))[0]
   if word==0:return ''.join(map(chr,words))
   words.append(word)
 def lookup(self,m):
  name=m.wstr(m.arg(1));m.events.append(['lookup',name,m.arg(2)]);m.ret(m.cells[name],pop=8)
 def assertion(self,m):raise AssertionError('unexpected mean assertion on shipped full-source inputs')
 def log(self,m):
  line=m.arg(3);assert 0x253<=line<=0x259
  value=struct.unpack('<Q',m.u.mem_read(m.u.reg_read(UC_X86_REG_ESP)+24,8))[0];m.events.append(['mean_log',line,f'{value:016x}']);m.ret()
 def write(self,u,access,address,size,value,_):
  if size!=4:return
  base=address-(address&15)
  # Definitions are 16-aligned; offsets are not all 16-aligned.
  for off in FIELDS:
   if address-off in self.ids:
    self.events.append(['store',self.ids[address-off],off,value&0xffffffff]);return
def main():
 source=json.loads(INPUT.read_text());cases=[];pins={}
 for image_name,image in [('original',h.original),('v14',h.modified)]:
  for pc in h.PRECISIONS:
   m=Machine(pc,image,source);m.call(0x4bbf90,ecx=m.manager,count=20_000_000);m.call(0x4bb590,ecx=m.manager,count=40_000_000)
   m.verify_v14();assert not m.null_accesses;pins.update(m.visited)
   rows=[{'id':ident,'value_1e4':m.r(d+0x1e4),'children_30c':m.r(d+0x30c),'weighted_36c':m.r(d+0x36c)} for ident,d in m.actors.items()]
   assert all(r['value_1e4']!=0xbf800000 for r in rows)
   cases.append({'image':image_name,'precision':pc,'means':[m.r(m.manager+off) for off in range(0x74,0x94,4)],'values':rows,'events':m.events,'final_assert_guard':m.rb(0xb3b4f8),'final_lookup_guard':m.r(0xb3a66c),'final_csv_guard':m.r(0xb3b4f4)})
   print(image_name,pc,len(rows),'values',len(m.events),'events',flush=True)
 out=h.header(__file__,__doc__);out.update(passed=True,input_sha256=sha(INPUT.read_bytes()),cases=cases,instructions=h.instructions(pins),declared_leaves=[hex(a) for a in sorted(LEAVES)],limitations=['This replays the full bound shipped CV dependency slice, not full native source loading or4BD800.','No original startup FP/profile capture: FreshNoProfile empty CSV, explicit PCs/MXCSR/SSE2 and fresh zero lookup/assert guards are supplied laboratory controls.','Native allocator/free/failure behavior, closed-stream virtual, diagnostic logger and actual source SVar cell lookup are declared leaves;45E090 and CRTpow are not seamed.','Typed nonalias source objects and prerequisite write-set disjointness are required. Shared/per-actor source mutations outside this supported shipped receipt remain unsupported.'])
 (HERE/'native-source-completion-v3.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8');print('passed',len(cases),'full-source cases',len(pins),'pins',flush=True)
if __name__=='__main__':main()
