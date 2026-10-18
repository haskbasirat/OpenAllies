"""511297..51145E trigger/objective extras, entered through real510EB0
prologue with an explicit already-captured parameter table at511297. Real
54DE30/5422F0/54A5A0/54DAC0/54DA70/4F1AD0/50DC00/49AD00/510150 execute.
4F8F00 named Kingdom lookup and4BAA20 existing text entry lookup are explicit
seams. No trigger Load, source registration, missing-entry creation, heap
failure or C++ unwind claim. Preinstruction service cutoffs retain prefixes.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from native_film_teams import cstring,read
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stopped(Exception):pass
def prefix(s):return s[:s.index(0)]if 0 in s else s
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
 data=m.alloc(0x100);m.w(0xb36530,data);m.w(data+0x38,0);m.w(0xb43b48,cstring(m,[]));m.w(0xb43b40,cstring(m,[]))
 target=m.alloc(0x30);m.call(0x50f890,target);world=m.alloc(0x300);m.w(0xb36534,world)
 manager=m.alloc(0x80);m.w(world+0x13c,manager);m.w(0xb36520,manager)
 identity=m.alloc(0x60010);m.w(0xb364c0,identity);actors={}
 for row in case['actors']:
  actor=m.alloc(0x80);actors[row['raw']]=actor;m.w(actor+0x18,cstring(m,row['name']))
  slot=row['raw']&65535;m.u.mem_write(identity+4+2*slot,(row['raw']>>16).to_bytes(2,'little'));m.w(identity+0x20004+4*slot,actor)
 m.w(manager+0x24,actors.get(case['condition'],0));m.w(manager+0x28,actors.get(case['event'],0))
 objectives=m.alloc(4*max(1,len(case['objectives'])));m.w(world+0x250,objectives);m.w(world+0x254,len(case['objectives']))
 entries={}
 for i,row in enumerate(case['objectives']):
  obj=m.alloc(0x20);m.w(obj+4,cstring(m,row['id']));m.w(objectives+4*i,obj)
  key=tuple(row['id'])
  if key not in entries:
   entry=m.alloc(0x20);m.w(entry,entry);m.w(entry+4,10000);m.w(entry+0x10,cstring(m,row['text']));entries[key]=entry
 def linked(values):
  nodes=[m.alloc(8)for _ in values]
  for i,(node,value)in enumerate(zip(nodes,values)):m.w(node,value);m.w(node+4,nodes[i+1]if i+1<len(nodes)else 0)
  return nodes[0]if nodes else 0
 triggers=[]
 for trigger in case['triggers']:
  t=m.alloc(0x40);m.u.mem_write(t+0x34,bytes([trigger['enabled']]));m.w(t+0x1c,m.alloc(8)if trigger['conditions']else 0);actions=[]
  for action in trigger['actions']:
   e=m.alloc(0x20);d=m.alloc(0x30);m.w(e,d);m.w(d+0xc,action['opcode']);m.w(d+0x24,2)
   params=action['parameters'];arr=m.alloc(0x34*max(1,len(params)));m.w(e+4,len(params));m.w(e+8,arr)
   for j,p in enumerate(params):
    dest=arr+0x34*j;m.w(dest,p['storage']);m.w(dest+0x2c,p['item'])
    if p['storage']in[1,2]:m.w(dest+4,p['value'])
    elif p['storage']==5:
     for k,w in enumerate(p['value']):m.w(dest+0x10+4*k,w)
    elif p['storage']==6:m.w(dest+0x1c,cstring(m,p['value']))
    elif p['storage']==7:m.w(dest+0x20,cstring(m,p['value'][0]));m.w(dest+0x24,cstring(m,p['value'][1]))
   actions.append(e)
  m.w(t+0x24,linked(actions));triggers.append(t)
 m.w(manager+4,linked(triggers));events=[];started=False;step=0
 def event(name):
  nonlocal step
  if step==case.get('stop',-1):raise Stopped(name)
  step+=1;events.append(name)
 def named():
  name=read(m,m.r(m.arg(0)));result=next((actors[r['raw']]for r in case['actors']if r['kingdom']and prefix(r['name'])==prefix(name)),0);m.ret(4,result)
 def lookup():
  key=tuple(read(m,m.r(m.arg(1))));entry=entries[key];m.w(m.arg(0),entry);m.w(entry+4,m.r(entry+4)+1);m.ret(16,m.arg(0))
 m.seam(0x4f8f00,named);m.seam(0x4baa20,lookup)
 hooks={0x5112f5:'triggers',0x511315:'enabled',0x511320:'conditions',0x51132b:'actions',0x511338:'opcode',
        0x5422f0:'convert',0x54dac0:'kingdom',0x54da70:'objective',0x4bedf0:'name',0x49ad00:'localized',
        0x5113ca:'next_action',0x5113d5:'next_trigger'}
 def observe(u,at,size,_):
  nonlocal started
  if at==0x511297:
   started=True
   arr=m.r(target+0x10);m.w(target+0x14,len(case['parameter_ids']))
   assert len(case['parameter_ids'])<=8
   for i,name in enumerate(case['parameter_ids']):m.w(arr+0x6c*i,cstring(m,name))
  if not started:return
  if at==0x5113c2 and u.reg_read(UC_X86_REG_ESI)==0:raise Stopped('destination')
  if at in hooks:event(hooks[at])
 m.u.hook_add(UC_HOOK_CODE,observe);stopped=None
 try:m.call(0x510eb0,target,[world])
 except Stopped as e:stopped=str(e)
 out=[]
 for i in range(m.r(target+0x14)):
  p=m.r(target+0x10)+0x6c*i;values=[]
  for j in range(m.r(p+0x44)):
   entry=m.r(p+0x40)+8*j;values.append(dict(key=read(m,m.r(entry)),text=read(m,m.r(entry+4))))
  out.append(values)
 if stopped is None:
  assert(m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 return dict(events=events,stopped=stopped,entries=out),m.windows()
def parameter(storage,item,value):return dict(storage=storage,item=item,value=value)
def action(*parameters,opcode=50):return dict(opcode=opcode,parameters=list(parameters))
def base(actions):return dict(parameter_ids=[[75]],actors=[dict(raw=1,name=[75],kingdom=True),dict(raw=2,name=[65],kingdom=False)],
 objectives=[dict(id=[79],text=[84,0,0xd800]),dict(id=[111],text=[116])],condition=1,event=2,
 triggers=[dict(enabled=1,conditions=False,actions=actions)])
def cases():
 raw=parameter(1,19,1);named=parameter(6,18,[75]);objective=parameter(6,13,[79]);
 yield base([]);yield base([action(raw,objective)]);yield base([action(named,objective),action(),action(parameter(0,19,0))]);
 yield base([action(parameter(0,23,0),objective),action(parameter(0,24,0),objective)]);
 yield base([action(raw,parameter(6,13,[111]))]);yield base([action(parameter(1,19,0x10001),objective)]);
 c=base([action(raw,objective)]);c['triggers']=[dict(enabled=0,conditions=False,actions=c['triggers'][0]['actions']),dict(enabled=1,conditions=True,actions=c['triggers'][0]['actions']),dict(enabled=1,conditions=False,actions=[action(opcode=49)])];yield c
 c=base([action(raw,objective)]);c['parameter_ids']=[];yield c
 c=base([action(raw,objective)]);c['objectives']=[];yield c
 c=base([action(raw,objective),action(parameter(5,19,[0,0x80000000,0x7f812345]),parameter(7,13,[[81],[82]]))]);yield c
 c=base([action(raw,objective)for _ in range(20)]);yield c
 c=base([action(raw,objective),action(named,parameter(6,13,[111]))]);n=len(run(c,'original','Double53')[0]['events'])
 for stop in range(n):yield dict(c,stop=stop)
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 doc=dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-extras.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
