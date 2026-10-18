"""Original4C8CB0 plus projectile spawn branch4BE780 execute.
Hooks: native empty-string resource, actor factory, target hit geometry, and
580BC0 boundary (the concrete lifecycle is separately native-tested).
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ABILITY,DEF,LOCAL,CONTEXT,SOURCE,TARGET,PROJECTILE,COMP,MANAGER,STRING,VT,KING,EMPTY=range(0x4000000,0x400d000,0x1000)
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c);self.events=[];self.visited={}
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x20000)
  self.w(ABILITY+4,DEF);self.w(ABILITY+8,LOCAL if c['local'] else 0);self.w(DEF+0x3c,123);self.w(LOCAL,DEF)
  for i,v in enumerate(c['local_values']['spawn_position']):self.w(LOCAL+8+i*4,v)
  self.w(LOCAL+0x14,c['local_values']['spawn_bone'])
  self.ptrs={1:SOURCE,2:TARGET,3:0x400e000,7:PROJECTILE};self.ids={v:k for k,v in self.ptrs.items()}
  for h,p in [(1,SOURCE),(2,TARGET)]:
   self.w(p,VT);self.w(VT+0xa8,KING)
   for i,v in enumerate(c['sources'][str(h)]['position']):self.w(p+0x20+i*4,v)
   for i,v in enumerate(c['sources'][str(h)]['facing']):self.w(p+0x2c+i*4,v)
  ctx=c['context'];self.put(CONTEXT,ctx['event_kind'],ctx['phase'],self.ptrs.get(ctx['source'],0),self.ptrs.get(ctx['allegiance'],0),*ctx['source_position'],self.ptrs.get(ctx['target'],0),*ctx['target_position'],ctx['word2c'])
  self.w(MANAGER,VT+0x200);self.w(VT+0x20c,EMPTY);self.w(STRING+0xc,2);self.w(PROJECTILE+0x64,COMP)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,p,*w):self.u.mem_write(p,struct.pack('<'+'I'*len(w),*(v&0xffffffff for v in w)))
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def ctx(self,p=CONTEXT):
  w=struct.unpack('<12I',self.u.mem_read(p,48));return{'event_kind':w[0],'phase':w[1],'source':self.ids.get(w[2]),'allegiance':self.ids.get(w[3]),'source_position':list(w[4:7]),'target':self.ids.get(w[7]),'target_position':list(w[8:11]),'word2c':w[11]}
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
  if pc==0x856370:self.ret(MANAGER)
  elif pc==EMPTY:self.ret(STRING)
  elif pc==KING:self.events.append({'name':'kingdom','actor':self.ids[u.reg_read(UC_X86_REG_ECX)]});self.ret(int(self.c['kingdom']))
  elif pc==0x404f60:self.events.append({'name':'missing_local','definition':10});self.ret()
  elif pc==0x4f9c10:
   p=self.arg();req={'definition':self.r(p+4),'position':[self.r(p+8+i*4)for i in range(3)],'direction':[self.r(p+0x14),self.r(p+0x18)],'name_utf16':[],'saved_id':self.r(p+0x20),'word24':self.r(p+0x24),'allegiance':self.ids.get(self.r(p+0x28)),'word2c':self.r(p+0x2c),'word30':self.r(p+0x30),'variant':self.r(p+0x34)-0x100000000 if self.r(p+0x34)>=0x80000000 else self.r(p+0x34),'reason':self.r(p+0x38),'words3c_4c':[self.r(p+0x3c+i*4)for i in range(5)],'flags50_52':[bool(b)for b in self.u.mem_read(p+0x50,3)]}
   self.events.append({'name':'create','request':req})
   if self.c['mutate_after_create']:
    self.w(LOCAL+0x10,bits(7.17));self.w(LOCAL+0x14,99);self.put(TARGET+0x20,*map(bits,[1.7,-2.9,5]))
   self.ret(PROJECTILE if self.c['factory_success'] else 0,4)
  elif pc==0x4eeee0:
   self.events.append({'name':'surface','actor':self.ids[u.reg_read(UC_X86_REG_ECX)],'direction':[self.arg(i)for i in [1,2,3]]});out=self.arg();self.put(out,*self.c['surface']);self.ret(out,16)
  elif pc==0x580bc0:
   self.events.append({'name':'start','actor':7,'context':self.ctx(self.arg()),'bone':self.arg(2)});self.ret(pop=12)
 def run(self):
  self.reset(0);self.w(self.STACK+4,CONTEXT);self.u.reg_write(UC_X86_REG_ECX,ABILITY)
  regs={r:0x99a0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  try:self.u.emu_start(0x4c8cb0,self.STOP,count=100000)
  except Exception as e:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP)))from e
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  assert all(self.u.reg_read(r)==v for r,v in regs.items())
  return {'result':self.ids.get(self.u.reg_read(UC_X86_REG_EAX)),'context':self.ctx(),'events':self.events}
def cases():
 rows=[]
 for source in [None,1]:
  for target in [None,2]:
   for local in [False,True]:
    for kingdom in [False,True]:
     for success in [False,True]:
      c={'name':f'{source}_{target}_{local}_{kingdom}_{success}','local':local,'kingdom':kingdom,'factory_success':success,'mutate_after_create':False,'local_values':{'spawn_position':list(map(bits,[.3,-.7,2.17])),'spawn_bone':17},'context':{'event_kind':1,'phase':0,'source':source,'allegiance':3,'source_position':list(map(bits,[17.1,-4.3,2.7])),'target':target,'target_position':list(map(bits,[-3,17,1.3333333])),'word2c':0},'sources':{'1':{'position':list(map(bits,[5.7,3.17,-1])),'facing':list(map(bits,[.6,-.8]))},'2':{'position':list(map(bits,[14.1,-7.33,8])),'facing':[0,bits(-1)]}},'surface':list(map(bits,[13.7,-7.1,1.5]))}
      rows.append(c)
 c=copy.deepcopy(rows[-1]);c['name']='factory_mutation';c['mutate_after_create']=True;rows.append(c)
 c=copy.deepcopy(rows[-1]);c['name']='point_factory_mutation';c['context']['target']=None;rows.append(c)
 return [{**copy.deepcopy(c),'precision':pc,'name':c['name']+pc}for pc in ['Single24','Double53','Extended64']for c in rows]
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as e:raise RuntimeError(c['name'])from e
  rows.append({'input':c,'output':out});visited.update(m.visited)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-launch-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
