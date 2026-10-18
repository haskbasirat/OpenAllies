"""Full original property lifetime with ordered mixed Flag/Add/Multiply modifiers."""
from pathlib import Path
import sys,copy,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];sys.path.insert(0,str(ROOT/'research/phase7-properties'))
import original_effects as base
from original_modifier_load import Machine as LoadBase,MOD,NODE,STACK as LS,STOP as LE
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(base.Machine):
 def __init__(self,c):
  raw=copy.deepcopy(c)
  for d in raw['definitions'].values():d['modifiers']=[m['Numeric']for m in d['modifiers']if 'Numeric'in m]
  super().__init__(raw);self.c=copy.deepcopy(c)
  self.w(base.ACTOR+0x78,base.PROP);self.w(base.ACTOR+0x6c,0x3700000 if c['mover']else 0);self.w(base.ACTOR+0x68,0x3701000 if c['character']else 0);self.w(base.ACTOR+0x84,0x3702000 if c['economy']else 0)
  for id,d in c['definitions'].items():
   id=int(id);p=base.definition(id);self.w(p+0x1c,p+0x800 if d['modifiers']else 0)
   for i,m in enumerate(d['modifiers']):
    node=p+0x800+i*8;q=base.num(id,i);self.w(node,q);self.w(node+4,node+8 if i+1<len(d['modifiers'])else 0)
    if 'Flag'in m:self.w(q,0xa2d66c);self.w(q+8,0);self.w(q+12,m['Flag']['index'])
    else:
     m=m['Numeric'];self.w(q,base.ADD_VT if m['kind']=='Add'else base.MUL_VT);self.w(q+8,1 if m['kind']=='Add'else 2);self.w(q+12,m['amount']);self.w(q+16,m['index']);self.w(q+20,int(m['repair']))
 def event(self,event):
  super().event(event)
  # Mutations below use explicit different named callbacks to verify rereads.
 def hook(self,u,a,n,data):
  if a in [0x4d4c60,0x4daf20,0x4daf40,0x576800,0x570020,0x4d1350]:
   if 0x400000<=a<0xc00000:self.visited[a]=n
   names={0x4d4c60:'ActorStop',0x4daf20:'Pause',0x4daf40:'Resume',0x576800:'Mover576800',0x570020:'Economy',0x4d1350:'Actions'}
   v={'enabled':bool(self.arg()&255)}if a in [0x4d4c60,0x576800]else {'flags':self.r(base.PROP+0x1c)}
   self.event({names[a]:v});self.ret(4 if a in [0x4d4c60,0x576800]else 0);return
  super().hook(u,a,n,data)
 def snapshot(self):
  d=super().snapshot();d['flags']=self.r(base.PROP+0x1c);d['mover_byte_4c']=self.u.mem_read(0x370004c,1)[0];return d
class Loader(LoadBase):
 def hook(self,u,a,n,data):
  if a==0x410c40:self.visited[a]=n;self.error='attribute';u.emu_stop();return
  super().hook(u,a,n,data)
 def run(self):
  self.w(LS,LE);self.w(LS+4,NODE);self.u.reg_write(UC_X86_REG_ESP,LS);self.u.reg_write(UC_X86_REG_ECX,MOD)
  self.u.emu_start(0x4febb0,LE,count=10000)
  if self.error:return {'error':self.error}
  assert self.u.reg_read(UC_X86_REG_EIP)==LE and self.u.reg_read(UC_X86_REG_ESP)==LS+8
  return {'index':self.r(MOD+12)}
class Register(Machine):
 def hook(self,u,a,n,data):
  if a==0x4c0f50:
   self.visited[a]=n;p=self.arg(1);s=bytearray()
   while bytes(u.mem_read(p,2))!=b'\0\0':s+=bytes(u.mem_read(p,2));p+=2
   self.registered.append({'index':self.arg(),'name':s.decode('utf-16le'),'extra':self.arg(2)});self.ret(12);return
  super().hook(u,a,n,data)
def cases():
 f=lambda i:{'Flag':{'index':i}}
 n=lambda kind,index,amount:{'Numeric':{'kind':kind,'index':index,'amount':base.bits(amount),'repair':False}}
 one=lambda m,flags=1:{'flags':flags,'modifiers':m}
 ins=lambda h,d,s=0x10001:{'Insert':{'handle':h,'definition':d,'kind':2,'created':base.bits(10),'source_id':s,'disabled':False}}
 rem=lambda d,s=1:{'Remove':{'definition':d,'source':s}}
 rows=[]
 def add(name,defs,ops,**kw):rows.append({'name':name,'definitions':{str(i+1):d for i,d in enumerate(defs)},'operations':ops,'minimum':base.bits(.001),'mutations':[],'mover':True,'character':True,'economy':True,**kw})
 for index in list(range(12))+[31,32,43]:
  for present in [False,True]:add(f'flag{index}_{present}',[one([f(index)])],[ins(1,1),rem(1)],mover=present,character=present,economy=present)
 add('duplicate_indispensable',[one([f(11)])],[ins(1,1),ins(2,1,0x10002),rem(1),rem(1,2)])
 add('remove_disabled',[one([f(11)])],[ins(1,1),ins(2,1,0x10002),rem(1,2),rem(1)])
 add('same_flag_different_properties',[one([f(11)]),one([f(11)],2)],[ins(1,1),ins(2,2),rem(1),rem(2)])
 add('mixed_order',[one([f(11),n('Add',3,.1),f(9),n('Multiply',19,.25)]),one([n('Add',3,.2),f(0)],2)],[ins(1,1),ins(2,2),rem(1),rem(2)])
 add('mixed_duplicate',[one([n('Add',3,.1),f(11),f(11),n('Add',3,.2),n('Multiply',19,.25)])],[ins(1,1),rem(1)])
 add('stale_source',[one([f(11)])],[ins(1,1,0x20001),rem(1,None)])
 add('numeric_callback_affects_next',[one([n('Add',3,.1),f(11),n('Multiply',19,.25)])],[ins(1,1),rem(1)],mutations=[{'on':'NumericChanged','scalars':{'19':base.bits(.75)}}])
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 loaders=[]
 for index,found in [(i,True)for i in list(range(12))+[31,32,43,-1]]+[(0,False)]:
  c={'precision':'Extended64','attribute':index,'found':found,'amount':0,'repair':False};m=Loader(c);loaders.append({'input':c,'output':m.run()});visited.update(m.visited)
 m=Register(next(cases()));m.registered=[];m.call(0x4c1110,0x3703000,[]);visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=base.IMAGE.at(a,n);assert raw==base.ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 dependencies={str(Path(base.__file__).relative_to(ROOT)):H(Path(base.__file__).read_bytes()),'research/phase7-properties/original_modifier_load.py':H((ROOT/'research/phase7-properties/original_modifier_load.py').read_bytes())}
 doc={'schema':1,'original_sha256':base.pe.ORIGINAL_SHA,'v14_sha256':base.pe.ACCEPTED_SHA,'producer_sha256':H(Path(__file__).read_bytes()),'dependencies':dependencies,'registration':m.registered,'loader_cases':loaders,'cases':rows,'visited_windows':windows,'seams':['Heap/vector allocation and free','Allegiance and property-added/removed/actor-added/removed effects','Numeric actor consequences','Flag actorStop, Character pause/resume, Mover576800, Economy570020 and Actions4D1350 required callbacks; actual4FE460 dispatch and Mover+4C write execute','TGI string/flag-domain lookup supplied to actual4FEBB0; invalid loader stops at native exception','Flag domain4C1110 registration collection transport']}
 path=HERE/'native-property-flags.json';path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8');print(json.dumps({'sequences':len(rows),'operations':sum(len(r['output'])for r in rows),'loaders':len(loaders),'flags':len(m.registered),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
