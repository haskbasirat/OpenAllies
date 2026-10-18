"""Original4C8500/4294E0/428F40/428570 FX request and instance creation.
Actor virtual calls, thread-ID, allocation, and final50C800 manager boundary
are explicit hooks. This does not execute particle graphs or render visuals.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine as Base,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ABILITY,DEF,CONTEXT,SOURCE,TARGET,ALLEGIANCE,VT,KING,ACTOR,OWNER,CLOCK,WORLD,RENDER,THREAD,MANAGER,INSTANCE=range(0x4000000,0x4010000,0x1000)
bits=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__();self.c=copy.deepcopy(c);self.events=[];self.instances=[];self.visited={}
  self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x20000)
  self.w(ABILITY+4,DEF);self.w(ABILITY+8,123 if c['local'] else 0);self.w(ABILITY+0xc,c['count'])
  for i,t in enumerate(c['triggers']):self.put(DEF+0x70+i*8,t['definition'],t['word4'])
  self.ptrs={1:SOURCE,2:TARGET,3:ALLEGIANCE};self.ids={v:k for k,v in self.ptrs.items()}
  for h,p in self.ptrs.items():self.w(p,VT);self.w(p+0x14,0x10000+h);self.put(p+0x20,*c['positions'][str(h)])
  self.w(VT+0xa8,KING);self.w(VT+0xb4,ACTOR);self.w(VT+0x108,OWNER);self.w(0x9f01e4,CLOCK)
  self.w(0xb36534,WORLD);self.w(WORLD+0x130,MANAGER);self.u.mem_write(MANAGER,bytes([c['suppressed']]));self.w(WORLD+0xe0,c['world_time']);self.w(0xb36538,RENDER);self.w(RENDER+0x8c,c['render_time']);self.w(0xb364d4,THREAD);self.w(THREAD+0x48,717)
  ctx=c['context'];self.put(CONTEXT,ctx['event_kind'],ctx['phase'],self.ptrs.get(ctx['source'],0),self.ptrs.get(ctx['allegiance'],0),*ctx['source_position'],self.ptrs.get(ctx['target'],0),*ctx['target_position'],ctx['word2c'])
  self.u.hook_add(UC_HOOK_CODE,self.hook)
 def put(self,p,*v):self.u.mem_write(p,struct.pack('<'+'I'*len(v),*(w&0xffffffff for w in v)))
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def ret(self,value=0,pop=0):
  s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop);self.u.reg_write(UC_X86_REG_EAX,value)
 def instance(self,p):
  trigger=self.r(p+4)
  return {'trigger':{'definition':self.r(trigger),'word4':self.r(trigger+4)},'word8':self.r(p+8),'allegiance_id':self.r(p+0xc),'source_id':self.r(p+0x10),'source_position':[self.r(p+0x14+i*4)for i in range(3)],'target_id':self.r(p+0x20),'target_position':[self.r(p+0x24+i*4)for i in range(3)],'local_definition':11 if self.r(p+0x30) else None,'use_count':self.r(p+0x34),'started_at':self.r(p+0x38),'parameters':[self.r(p+0x3c+i*4)for i in range(3)],'offset':[self.r(p+0x48+i*4)for i in range(3)],'kind':self.r(p+0x54),'flag58':bool(self.u.mem_read(p+0x58,1)[0]),'callback_word60':self.r(p+0x60),'next':None}
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:
   raw=original.range(pc,size);assert raw==modified.range(pc,size);self.visited[pc]=raw.hex()
  if pc==KING:self.events.append('kingdom');self.ret(int(self.c['source_kind']=='kingdom'))
  elif pc==ACTOR:self.events.append('actor');self.ret(int(self.c['source_kind']=='actor'))
  elif pc==OWNER:
   self.events.append('allegiance')
   if self.c['mutate']:self.put(SOURCE+0x20,*map(bits,[9.17,-3.7,4.11]));self.put(TARGET+0x20,*map(bits,[-3.3,17.7,1.919]))
   self.ret(ALLEGIANCE if self.c['owner'] else 0)
  elif pc==CLOCK:self.events.append('clock');self.ret(717 if self.c['render_thread'] else 123)
  elif pc==0x6f7810:self.ret(INSTANCE)
  elif pc==0x50c800:
   assert self.arg(1)==0;self.events.append('submit');self.instances.append(self.instance(self.arg()));self.ret(pop=8)
 def run(self):
  for event in range(4):
   self.reset(0);self.u.reg_write(UC_X86_REG_ECX,ABILITY);self.put(self.STACK+4,event,CONTEXT)
   regs={r:0xabc000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
   for r,v in regs.items():self.u.reg_write(r,v)
   self.u.emu_start(0x4c8500,self.STOP,count=100000)
   assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
   assert all(self.u.reg_read(r)==v for r,v in regs.items())
  return{'events':self.events,'instances':self.instances}
def cases():
 rows=[]
 for source in [None,1]:
  for target in [None,2]:
   for kind in ['actor','kingdom','object']:
    for render in [False,True]:
     for owner in [False,True]:
      rows.append({'name':f'{source}_{target}_{kind}_{render}_{owner}','context':{'event_kind':1,'phase':2,'source':source,'allegiance':3,'source_position':list(map(bits,[7.17,-3.2,1.1])),'target':target,'target_position':list(map(bits,[-4.1,3.17,8.19])),'word2c':0},'positions':{str(i):list(map(bits,[i*1.317,i*(-.717),i*.19]))for i in range(1,4)},'local':True,'count':17,'source_kind':kind,'owner':owner,'render_thread':render,'world_time':bits(9.17),'render_time':bits(9.3),'triggers':[{'definition':10+i,'word4':i*7}for i in range(4)],'suppressed':False,'mutate':True})
 for prop,value in [('suppressed',True),('triggers',[{'definition':0,'word4':7}]*4),('local',False),('count',0xffffffff)]:
  c=copy.deepcopy(rows[0]);c[prop]=value;c['name']=prop;rows.append(c)
 return rows
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(original.b).hexdigest(),'accepted_sha256':hashlib.sha256(modified.b).hexdigest(),'method':__doc__,'cases':rows,'native_emissions':len(rows)*4,'visited_windows':[{'address':hex(a),'bytes':b}for a,b in sorted(visited.items())]}
 (HERE/'native-fx-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows),'emissions':len(rows)*4}))
if __name__=='__main__':main()
