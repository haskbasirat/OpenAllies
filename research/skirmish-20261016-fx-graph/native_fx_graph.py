"""Original42B990/42BF10/42C1B0/42C5E0 CREATE_VFX compilation.
Typed file getters, enums, registry lookup, string transport and allocation
are explicit services. Original command constructors, recursive loader order,
partial writes, head/tail links and first-VFX map insertion run unchanged.
"""
from pathlib import Path
import sys,json,hashlib,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
ORIGINAL=nm.original
FXC=['CREATE_VFX','STOP_VFX','EQUIPMENT','PLAY_SFX','STOP_SFX','CHANGE_ASPECT','CHANGE_SKIN','CREATE_RFX','PLAY_OBJECT_ANIMATION','PLAY_HIT_EFFECT']
AT=['SELF','TARGET_THING','SOURCE_GROUND','TARGET_GROUND','SOURCE_ABILITY_POSITION','TARGET_HIT_POSITION','MAP_CENTER']
HEALTH=['ABSOLUTE_BELOW','ABSOLUTE_ABOVE','RELATIVE_DECREASE','RELATIVE_INCREASE']
def node(**replace):
 d=dict(type='CREATE_VFX',ChainImmediate=[],Chain=[],target_flags='',terrain_group_filter='',frequency=1,delay=0,
  health=0xbf800000,health_trigger_type='ABSOLUTE_BELOW',stop_on_destructed=True,actor_state='',
  vfx_ids='smoke',duration=0,at='SELF',attached=True,location='',translation=[0,0,0],orientation=[0,0xbf800000,0])
 d.update(replace);return d
def cases():
 out=[dict(name='empty',nodes=[],roots=[]),dict(name='ordinary',nodes=[node()],roots=[0])]
 for i in range(7):out.append(dict(name='at_'+str(i),nodes=[node(at=AT[i])],roots=[0]))
 for bits in [0,0x80000000,1,0x80000001,0x3e99999a,0x7f7fffff,0xff7fffff]:
  out.append(dict(name='numeric_'+str(bits),nodes=[node(duration=bits,health=bits,delay=bits)],roots=[0]))
 for field,value in [('type','INVALID'),('vfx_ids','missing'),('vfx_ids','cutscene'),('at','INVALID'),
  ('location','missing'),('location','head'),('terrain_group_filter','missing'),('terrain_group_filter','snow'),
  ('actor_state','missing'),('actor_state','moving'),('actor_state','idle'),('health_trigger_type','UNKNOWN'),
  ('target_flags','friend !enemy +body'),('target_flags','friend missing')]:
  out.append(dict(name='field_'+field+'_'+value,nodes=[node(**{field:value})],roots=[0]))
 out.append(dict(name='duplicates_and_chains',nodes=[node(ChainImmediate=[1,2],Chain=[3]),node(vfx_ids='smoke'),
  node(vfx_ids='fire'),node(vfx_ids='smoke')],roots=[0,1]))
 out.append(dict(name='child_failure_after_first',nodes=[node(Chain=[1,2]),node(),node(vfx_ids='missing')],roots=[0]))
 for field in ['frequency','delay','health','stop_on_destructed','duration','attached','orientation']:
  out.append(dict(name='required_'+field,nodes=[node()],roots=[0],fail=field))
 path=HERE/'actual-source-inputs.json'
 if path.exists():
  actual=json.loads(path.read_text())
  names={'actual_'+row['id'].lower() for row in actual['tutorial02_created']}
  out.extend(c for c in actual['inputs'] if c['name'].lower() in names)
 return out
class NativeThrow(Exception):pass
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(ORIGINAL.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(ORIGINAL.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 nodes=c['nodes'];source_ptrs=[m.alloc(4) for _ in nodes];source_indices={p:i for i,p in enumerate(source_ptrs)}
 lists=m.alloc(8);context=m.alloc(32);m.w(context,m.alloc(31*4));m.w(context+4,31);m.w(context+8,0);m.wb(context+12,0)
 game=m.alloc(0x900);m.w(0xb36530,game)
 for off,manager in [(12,'VFX'),(0x5bc,'TerrainGroup'),(0x580,'Bone')]:
  ptr=m.alloc(32);m.w(game+off,ptr)
  source_indices[ptr]=manager
 source_indices[game+0x2c0]='ActorState'
 strings={};string_values={};events=[];allocated=[];complete=set();warnings=[]
 def string(text):
  if text not in strings:
   p=m.alloc(16+2*(len(text)+1));m.w(p+4,len(text));m.w(p+12,1000000);m.u.mem_write(p+16,(text+'\0').encode('utf-16-le'))
   strings[text]=p+16;string_values[p+16]=text
  return strings[text]
 def text(p):
  if p in string_values:return string_values[p]
  chars=[]
  while True:
   b=bytes(m.u.mem_read(p,2));p+=2
   if b==b'\0\0':return b''.join(chars).decode('utf-16-le')
   chars.append(b)
 m.w(0xb3b0c0,m.array([m.array([i,string(value)]) for i,value in enumerate(AT)]))
 state_names=m.alloc(36);m.w(state_names+32,m.array([8,0,string('moving')]))
 m.w(game+0x2e4,state_names)
 def input_value(mm,name):
  index=source_indices[mm.ecx()];events.append(['get',index,name])
  if c.get('fail')==name:raise NativeThrow('required '+name)
  return nodes[index][name]
 def getter(mm):
  name=text(mm.arg(1));value=input_value(mm,name);mm.ret(m.array([string(value)]),4)
 m.seam(0x5b54d0,getter)
 m.seam(0x4012c0,lambda mm:mm.ret(mm.arg(1)))
 m.seam(0x405b80,lambda mm:(m.w(mm.ecx(),m.r(mm.arg(1))),mm.ret(mm.ecx(),4)))
 m.seam(0x5b4ed0,lambda mm:mm.ret(input_value(mm,text(mm.arg(1))),4))
 m.seam(0x5b4ae0,lambda mm:mm.ret_st0_f32(input_value(mm,text(mm.arg(1))),4))
 m.seam(0x5b5380,lambda mm:mm.ret(int(input_value(mm,text(mm.arg(1)))),4))
 def point(mm):
  value=input_value(mm,text(mm.arg(2)));p=mm.arg(1)
  for i,v in enumerate(value):m.w(p+4*i,v)
  mm.ret(p,8)
 m.seam(0x5b4c30,point)
 def enum(mm):
  domain={0xb3b03c:('FXC',FXC),0xb3b0cc:('AT',AT),0xb39d78:('HealthTrigger',HEALTH)}[mm.ecx()]
  value=text(m.r(mm.arg(1)));events.append(['enum',domain[0],value])
  found=next((i for i,v in enumerate(domain[1]) if v.lower()==value.lower()),None)
  mm.ret(0 if found is None else m.array([m.array([found])]),4)
 m.seam(0x497220,enum)
 def child_count(mm):
  index=source_indices[mm.ecx()];name=text(mm.arg(1));events.append(['children',index,name]);mm.ret(len(nodes[index][name]),4)
 m.seam(0x5b4220,child_count)
 def child(mm):
  cursor=mm.ecx();owner=source_indices[m.r(cursor)];name=text(m.r(cursor+8));index=m.r(cursor+4)
  mm.ret(source_ptrs[nodes[owner][name][index]])
 m.seam(0x42be10,child)
 def allocate_command(mm):
  p=m.alloc(0x6c);m.u.mem_write(p,b'\xcd'*0x6c);allocated.append(p);mm.ret(p)
 m.seam(0x85c680,allocate_command)
 m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(mm.arg(1))))
 m.seam(0x410c40,lambda mm:(_ for _ in ()).throw(NativeThrow(text(mm.arg(1)))))
 references={};reference_values={}
 def ref(manager,ids):
  key=(manager,ids)
  if key not in references:
   data=c.get('lookups',{}).get(manager,{}).get(ids)
   p=m.alloc(0x80);references[key]=p;reference_values[p]=ids if data is None else data['id']
   m.w(p+12,{'head':3,'idle':0,'moving':8}.get(ids,0) if data is None else data.get('ordinal') or 0)
   m.wb(p+0x39,int(ids=='cutscene') if data is None else int(data['cutscene']))
  return references[key]
 def find(mm):
  manager=source_indices[mm.ecx()]
  ids=text(mm.arg(1));events.append(['find',manager,ids])
  missing=ids=='missing' if 'lookups' not in c else c['lookups'][manager][ids] is None
  mm.ret(0 if missing else ref(manager,ids),4)
 m.seam(0x4bdb70,find);m.seam(0x42c070,find)
 def target(mm):
  raw=text(m.r(mm.arg(1)))
  for token in raw.replace(',',' ').split():
   sign=token[0] if token[:1] in ('!','-','+') else '';name=token[1:] if sign else token
   events.append(['target',name]);i={'friend':0,'enemy':1,'body':12}.get(name)
   if i is None:raise NativeThrow('target '+name)
   off=4 if sign in ('!','-') else 0;m.w(mm.ecx()+off,m.r(mm.ecx()+off)|(1<<i))
  mm.ret(pop=4)
 m.seam(0x4c5bf0,target)
 m.seam(0x404f60,lambda mm:(warnings.append(allocated.index(mm.u.reg_read(nm.UC_X86_REG_ESI))),mm.ret()))
 # Native reference hash/insert execute; only the hash/string-copy transports
 # are explicit. No stop/animation consumers are claimed by this scope.
 m.seam(0x5043f0,lambda mm:mm.ret(sum(ord(x.lower()) for x in text(mm.arg(1)))%m.r(mm.ecx()+4),4))
 m.seam(0x96cad6,lambda mm:mm.ret(0 if text(mm.arg(1)).lower()==text(mm.arg(2)).lower() else 1))
 error=None
 for root in c['roots']:
  try:m.call(0x42b990,lists,[source_ptrs[root],context])
  except NativeThrow as e:error=str(e);break
  except Exception:
   from unicorn.x86_const import UC_X86_REG_EIP
   print(c['name'],hex(m.u.reg_read(UC_X86_REG_EIP)),events[-12:]);raise
 def linked(pair):
  result=[];p=m.r(pair);last=0
  while p:
   value=m.r(p);result.append(allocated.index(value));complete.add(value);last=p;p=m.r(p+4)
  assert m.r(pair+4)==last
  return result
 roots=linked(lists);out=[]
 for p in allocated:
  immediate=linked(p+8);chain=linked(p+0x10)
  words={str(off):m.r(p+off) for off in [0x18,0x1c,0x24,0x28,0x2c,0x30,0x34,0x40,0x44,0x48,0x4c,0x50,0x54,0x58,0x5c,0x60,0x68] if m.r(p+off)!=0xcdcdcdcd}
  byte={str(off):m.rb(p+off) for off in [4,0x38,0x64,0x65] if m.rb(p+off)!=0xcd}
  refs={str(off):reference_values.get(m.r(p+off)) for off in (0x20,0x3c) if m.r(p+off)!=0xcdcdcdcd}
  out.append(dict(words=words,bytes=byte,references=refs,immediate=immediate,chain=chain))
 for p,item in zip(allocated,out):item['complete']=p in complete
 return dict(error=error,nodes=out,roots=roots,warnings=warnings,events=events),m.visited
def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(pc,c['name']);pins.update(pa);pins.update(pb)
   rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,
  instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())])
 path=HERE/'native-fx-graph.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=doc['native_executions'],pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
