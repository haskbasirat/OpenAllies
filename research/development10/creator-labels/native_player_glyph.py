"""7666C0 lazy variable-cache/Player/online/Session/glyph selection proof.

Named4B2E60,792DC0 and48D500 leaves supply explicit laboratory answers.
Required getter stops are external observation cutoffs, not native C++ SEH
unwinding claims.493500's actual integer-to-word store is proved separately
with base/typed-getter seams. No online account, readiness or AI success is
manufactured. All native rows run original/v14 atPC24/53/64.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));from native_creator_labels import Machine,PE0,PE1,PC
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
class Stop(Exception):pass
class GlyphMachine(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'glyph_ready',False):
   if pc==0x4b2e60:
    name=self.text(self.arg());assert self.arg(1)==4
    label='lookup_kingdom'if name=='KingdomGlyphInfo'else'lookup_observer';self.trace.append(label)
    if self.cutoff==label:raise Stop()
    self.ret(8,self.kingdom_var if name=='KingdomGlyphInfo'else self.observer_var);return
   if pc==0x792dc0:
    assert self.arg()==self.peer+0x7c
    self.trace.append('online_glyph')
    if self.cutoff=='online_glyph':raise Stop()
    self.ret(value=self.glyph if self.online_result else 0);return
   if pc==0x48d500:
    text=self.text(self.arg());self.trace.append('find:'+text)
    if self.cutoff=='find':raise Stop()
    self.ret(4,self.glyph);return
  if getattr(self,'index_ready',False):
   if pc==0x47d960:self.ret(4);return
   if pc==0x5b4ed0:
    assert self.text(self.arg())=='index';self.ret(4,self.index_input);return
   if pc==0x493525:raise Stop()
  super().hook(u,pc,size,user)
 def watch_read(self,u,access,address,size,value,user):
  if not getattr(self,'glyph_ready',False):return
  pc=u.reg_read(UC_X86_REG_EIP)
  label={0x766744:'ai',0x76674d:'online_manager',0x766757:'peer',0x766774:'session',0x766789:'kingdom',0x7667a8:'variable_value'}.get(pc)
  if label:
   self.trace.append(label)
   if self.cutoff==label:raise Stop()
def glyph_case(m,pc,case):
 m.fpu(pc);m.actual={};m.record_actual=True;m.glyph_ready=True;m.index_ready=False;m.trace=[]
 m.cutoff=case.get('cutoff');m.player=m.alloc(0x60);m.peer=m.alloc(0x90);m.glyph=m.alloc(0x20)
 m.kingdom_var=m.alloc(0x20);m.observer_var=m.alloc(0x20)
 m.cs(m.kingdom_var+0x10,case.get('kingdom_text','ch_gem'));m.cs(m.observer_var+0x10,'ch_observer')
 flags=case.get('flags',0)
 m.w(0xb40aec,flags);m.w(0xb40ae8,m.kingdom_var if flags&1 else 0x55555555);m.w(0xb40ae4,m.observer_var if flags&2 else 0x55555555)
 m.b(m.player+0xc,case.get('ai',0));m.w(m.player+4,m.peer if case.get('peer',False) else 0)
 m.w(m.player+0x28,0x12345678 if case.get('kingdom',True) else 0)
 m.w(0xb40c9c,0x12340000 if case.get('online',False) else 0);m.online_result=case.get('online_result',False)
 session=m.alloc(0x100);m.w(0xb36550,session);m.w(session+0xe4,case.get('session',2))
 data=m.alloc(0x400);manager=m.alloc(0x60);m.w(0xb36530,data);m.w(data+0x3bc,manager)
 error=None;result=None
 try:result=m.call(0x7666c0,this=m.player)
 except Stop:error=m.cutoff
 out={'flags':m.r(0xb40aec),'kingdom_pointer':m.r(0xb40ae8)==m.kingdom_var,
      'observer_pointer':m.r(0xb40ae4)==m.observer_var,'returned_glyph':None if result is None else result!=0,'error':error,'trace':m.trace.copy()}
 return out,m.actual.copy()
def main():
 cases=[{}, {'ai':1,'online':True,'peer':True,'online_result':True},
  {'online':True}, {'online':True,'peer':True}, {'online':True,'peer':True,'online_result':True,'session':0},
  {'session':0},{'session':1},{'session':3},{'session':4,'kingdom':False},{'session':5},{'session':6},
  {'kingdom':False},{'kingdom_text':''},{'flags':1},{'flags':2},{'flags':3}]
 cases += [dict(online=True,peer=True,cutoff=label)for label in
  ['lookup_kingdom','lookup_observer','ai','online_manager','peer','online_glyph','session','kingdom','variable_value','find']]
 rows=[];pins={};index_rows=[]
 for image in ['original','v14']:
  m=GlyphMachine(image);m.u.hook_add(UC_HOOK_MEM_READ,m.watch_read)
  for pc in PC:
   for case in cases:
    output,executed=glyph_case(m,pc,case);pins.update(executed);rows.append({'image':image,'precision':pc,'input':case,'output':output})
   for index in [0,132,140,32768,65535,65536,0x12345678,0xffffffff]:
    m.fpu(pc);m.actual={};m.record_actual=True;m.glyph_ready=False;m.index_ready=True;m.index_input=index
    data=m.alloc(0x20);m.u.mem_write(data,b'\xa5'*0x20);node=m.alloc(4)
    try:m.call(0x493500,[node],this=data,callee_pop=4)
    except Stop:pass
    else:raise AssertionError('index prefix was not reached')
    value=struct.unpack('<H',m.u.mem_read(data+0x18,2))[0];assert value==index&0xffff
    index_rows.append({'image':image,'precision':pc,'input':index,'index_word':value,'following_byte':m.rb(data+0x1a)})
    assert m.rb(data+0x1a)==0xa5;pins.update(m.actual)
 for collection in [rows,index_rows]:
  a=[{k:v for k,v in r.items()if k!='image'}for r in collection if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in collection if r['image']=='v14'];assert a==b
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append({'va':hex(va),'bytes':raw.hex()})
 globals=[]
 for image,pe in [('original',PE0),('v14',PE1)]:
  rva=0xb40aec-pe.base
  vs,section,raw,offset=next(s for s in pe.sections if s[1]<=rva< s[1]+s[0])
  within=rva-section
  value=pe.b[offset+within:offset+within+4]if within+4<=raw else bytes(4)
  assert value==bytes(4)
  globals.append({'image':image,'flags_va':'0xb40aec','initial_word':0,'section_rva':section,
      'virtual_size':vs,'raw_size':raw,'within_section':within,'initialized_raw_bytes':within+4<=raw})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,'index_rows':index_rows,'global_image':globals,'instructions':instructions}
 path=HERE/'native-player-glyph.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'index_rows':len(index_rows),'pins':len(instructions),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
