"""Original7A0D40 startup ordering with explicit required live-service fixtures."""
from pathlib import Path
import copy,hashlib,json,sys
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
WORLD,REGISTRY,KINGDOMS,SLOTS,ACTORVT,CONTEXTVT,KB,AB,SB,CB,HOOK,REGIONB=0x4000000,0x4010000,0x4080000,0x4081000,0x4082000,0x4083000,0x4084000,0x4090000,0x40c0000,0x40e0000,0x41ff000,0x41e0000
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(Base):
 def __init__(self,case):
  super().__init__();self.u.mem_map(0x4000000,0x200000);self.c=copy.deepcopy(case);self.actions=copy.deepcopy(case.get('mutations',{}));self.events=[];self.executed={}
  self.u.hook_add(UC_HOOK_CODE,self.hook);self.reset(0);self.w(0xb36534,WORLD);self.w(0xb364c0,REGISTRY);self.w(0xb3cc94,case['global_revision'])
  self.u.mem_write(self.STATE+0x20,bytes([case['state']['ready']]))
  for off,key in [(0x24,'world_revision'),(0x6c,'word6c'),(0x70,'word70')]:self.w(self.STATE+off,case['state'][key])
  self.array('kingdoms',case['kingdoms']);self.array('ai_slots',case['ai_slots'])
  self.w(ACTORVT+0xb4,HOOK);self.w(ACTORVT+0xb0,HOOK+16);self.w(CONTEXTVT+0x38,HOOK+32)
  self.w(REGISTRY+0x60004,len(case['actors']));self.w(REGISTRY+0x6000c,len(case['actors'])+1)
  for i,a in enumerate(case['actors']):
   p=AB+i*0x100;self.w(p,ACTORVT);self.w(p+8,a['flags']);self.w(p+0x14,0x10000+i+1);self.w(REGISTRY+0x20004+(i+1)*4,p)
  for i,s in enumerate(case['seen']):
   p=SB+i*0x100;context=CB+i*0x1000;self.w(p+8,s['position_bits'][0]);self.w(p+12,s['position_bits'][1]);self.w(p+16,context)
   self.w(context,CONTEXTVT);self.w(context+0x548,s['words'][0]);self.w(context+0x534,s['words'][1])
 def array(self,name,values):
  self.c[name]=values[:];ptr=KINGDOMS if name=='kingdoms'else SLOTS;owner=WORLD+0x154 if name=='kingdoms'else self.STATE+0x60
  self.w(owner,ptr);self.w(owner+4,len(values))
  for i,key in enumerate(values):self.w(ptr+i*4,0 if key is None else KB+key*16)
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def ret(self,n=0,value=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+n)
 def event(self,name,args=()):
  self.events.append({'name':name,'args':list(args),'ready':bool(self.u.mem_read(self.STATE+0x20,1)[0]),'revision':self.r(self.STATE+0x24)})
  key=name+(':'+str(args[0])if args else '')
  for change in self.actions.pop(key,[]):
   kind=change['kind'];value=change['value']
   if kind in ['kingdoms','ai_slots']:self.array(kind,value)
   elif kind=='ready':self.u.mem_write(self.STATE+0x20,bytes([value]))
   elif kind=='revision':self.w(0xb3cc94,value)
   elif kind=='actor_flags':self.w(AB+change['index']*0x100+8,value);self.c['actors'][change['index']]['flags']=value
   elif kind=='context_words':
    p=CB+change['index']*0x1000;self.w(p+0x548,value[0]);self.w(p+0x534,value[1]);self.c['seen'][change['index']]['words']=value
   elif kind=='remove_actor':self.w(REGISTRY+0x20004+(value+1)*4,0);self.w(REGISTRY+0x60004,self.r(REGISTRY+0x60004)-1)
   else:raise ValueError(kind)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.executed[pc]=size
  ecx=u.reg_read(UC_X86_REG_ECX)
  simple={0x79f9b0:'clear78',0x56a270:'clear80',0x7abc30:'prepare'}
  if pc in simple:self.event(simple[pc]);self.ret();return
  if pc==0x79e170:self.event('kingdom',[(self.arg()-KB)//16]);self.ret(4);return
  if pc==0x7b4830:self.event('ai',[(ecx-KB)//16]);self.ret();return
  if pc in (HOOK,HOOK+16):
   i=(ecx-AB)//0x100;name='b4'if pc==HOOK else'b0';self.event(name,[i]);self.ret(value=int(self.c['actors'][i][name]));return
  if pc==0x7a0c20:self.event('special',[(self.arg()-AB)//0x100]);self.ret(4);return
  if pc==0x4ea430:
   actor=(ecx-AB)//0x100;kingdom=(self.arg()-KB)//16;self.event('seen',[actor,kingdom])
   index=next((i for i,s in enumerate(self.c['seen'])if s['actor']==actor and s['kingdom']==kingdom),None);self.ret(4,0 if index is None else SB+index*0x100);return
  if pc==HOOK+32:
   i=(ecx-CB)//0x1000;self.event('eligible',[i]);self.ret(value=int(self.c['seen'][i]['eligible']));return
  if pc==0x7aa3d0:
   point=[self.arg(),self.arg(1)];self.event('region',point);s=next(s for s in self.c['seen']if s['position_bits']==point);self.ret(8,0 if s['region']is None else REGIONB+s['region']*16);return
  if pc==0x7b8ff0:
   region=(ecx-REGIONB)//16;kingdom=(self.arg()-KB)//16;self.event('binding',[region,kingdom]);s=next(s for s in self.c['seen']if s['region']==region and s['kingdom']==kingdom);self.ret(4,0 if s['binding']is None else REGIONB+0x1000+s['binding']*16);return
  if pc==0x7ba0d0:self.event('register_seen',[(ecx-REGIONB-0x1000)//16,(self.arg()-SB)//0x100]);self.ret(4);return
 def run(self):
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(0x7a0d40,self.STOP,count=1000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4
  assert [self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  return {'state':{'ready':bool(self.u.mem_read(self.STATE+0x20,1)[0]),'world_revision':self.r(self.STATE+0x24),'word6c':self.r(self.STATE+0x6c),'word70':self.r(self.STATE+0x70)},'events':self.events}
def cases():
 base={'name':'empty','state':{'ready':False,'world_revision':17,'word6c':123,'word70':456},'global_revision':99,'kingdoms':[],'ai_slots':[],'actors':[],'seen':[],'mutations':{}}
 rows=[base]
 c=copy.deepcopy(base);c['name']='already_ready';c['state']['ready']=True;rows.append(c)
 c=copy.deepcopy(base);c['name']='kingdom_slot_holes';c['kingdoms']=[None,1,None,3];c['ai_slots']=[None,8,None,9];rows.append(c)
 c=copy.deepcopy(c);c['name']='actors_and_seen';c['actors']=[{'b4':True,'b0':True,'flags':0},{'b4':True,'b0':False,'flags':1},{'b4':False,'b0':True,'flags':0}]
 for i,(actor,kingdom)in enumerate([(0,1),(0,3),(2,1),(2,3)]):c['seen'].append({'actor':actor,'kingdom':kingdom,'eligible':True,'words':[1,0],'position_bits':[0x3f800000+i*0x800000,0x40000000],'region':i,'binding':i+10})
 rows.append(c)
 for name,index,key,value in [('ineligible',0,'eligible',False),('empty_context',1,'words',[0,0]),('second_word',2,'words',[0,1]),('outside_region',2,'region',None),('missing_binding',3,'binding',None)]:
  row=copy.deepcopy(c);row['name']=name;row['seen'][index][key]=value;rows.append(row)
 mutations=[('late_revision','clear80',[{'kind':'revision','value':713}]),('append_kingdom','kingdom:1',[{'kind':'kingdoms','value':[None,1,3,4]}]),('shrink_kingdoms','kingdom:1',[{'kind':'kingdoms','value':[None,1]}]),('replace_slots','ai:8',[{'kind':'ai_slots','value':[None,8,None,10,11]}]),('disable_on_b4','b4:0',[{'kind':'actor_flags','index':0,'value':1}]),('remove_next_actor','special:0',[{'kind':'remove_actor','value':1}]),('clear_ready_at_seen','seen:0',[{'kind':'ready','value':False}]),('context_callback_clears_words','eligible:0',[{'kind':'context_words','index':0,'value':[0,0]}])]
 for name,key,changes in mutations:
  row=copy.deepcopy(c);row['name']=name;row['mutations']={key:changes};rows.append(row)
 return rows
def main():
 fixtures=[];visited={}
 for case in cases():
  m=Machine(case);result=m.run();fixtures.append({'input':case,'native':result});visited.update(m.executed)
 pins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 doc={'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),'fixtures':fixtures,'native_instructions_and_seam_entries':pins,'scope':'Complete7A0D40 outer initialization, actual null-skipping Kingdom iteration and ID registry iteration. Cleanup/topology/Kingdom/AI/special/seen effects and actor virtual results are explicit required fixture services; no complete live AI initialization claim. ABI checked on every execution.'}
 (OUT/'native-startup.json').write_text(json.dumps(doc,indent=2)+'\n');print({'passed':True,'cases':len(fixtures)})
if __name__=='__main__':main()
