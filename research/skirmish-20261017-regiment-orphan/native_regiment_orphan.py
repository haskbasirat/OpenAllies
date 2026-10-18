"""Original522270/529D80, actual ordinary Actor vf2C4CEEF0 and live4364C0.
The real4D7960 removed-bit guard executes. Active death is stopped precisely
at its first4D57C0 child; that positive child is not acknowledged as success.
Controlled virtual-return mutations test current Mind.actor, one captured Org,
and current Org count/vector after each callback. No original file is changed.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-member-deployment'))
from native_member_deployment import machine,nm
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EAX
class Required(Exception):pass
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def cases():
 base=dict(reasons=[0,0,7,0,4,0xffffffff],members=[1,None,2,1,3],
           alternate=[4,5,None,2,1,3],initial_actor=0,initial_org=0,
           after_first_actor=None,after_member_actor=None,mutation='',death_removed=True,fail_at=0)
 out=[]
 for reasons in [[0]*6,[1,2,3,4,5,6],[0,3,0,0xffffffff,0,4]]:
  for members in [[],[None],[1],[1,None,2,1,3],[None,None,None],[0,0,5]]:
   c=copy.deepcopy(base);c.update(reasons=reasons,members=members);out.append(c)
 for mutation in ['change_actor_before_org','change_org_field_only','replace_vector','shrink','grow','change_actor_after_member']:
  for removed in [False,True]:
   c=copy.deepcopy(base);c.update(mutation=mutation,death_removed=removed)
   if mutation=='change_actor_before_org':c['after_first_actor']=4
   if mutation=='change_actor_after_member':c['after_member_actor']=5
   out.append(c)
 for removed in [False,True]:
  for members in [[],[None],[1,2]]:
   c=copy.deepcopy(base);c.update(members=members,death_removed=removed);out.append(c)
 for fail in [1,2,3,4,5]:
  c=copy.deepcopy(base);c['fail_at']=fail;out.append(c)
 return out

def run(m,c):
 m.observers.clear();m.seams.clear()
 actors=[m.alloc(0x180)for _ in range(6)];mind=m.alloc(0x300)
 orgs=[m.alloc(0x40)for _ in range(2)]
 vectors=[m.alloc(32)for _ in range(3)]
 lists=[c['members'],c['alternate'],c['alternate']]
 events=[];writes=[];error=None;calls=0
 for i,actor in enumerate(actors):
  m.w(actor,0xa273f0);m.w(actor+0x178,c['reasons'][i]);m.w(actor+8,1 if c['death_removed']else 0)
  m.w(actor+0x7c,orgs[0]if i<4 else orgs[1])
 for vec,values in zip(vectors,lists):
  for i,value in enumerate(values):m.w(vec+4*i,0 if value is None else actors[value])
 m.w(orgs[0]+0x28,vectors[0]);m.w(orgs[0]+0x2c,len(c['members']))
 m.w(orgs[1]+0x28,vectors[1]);m.w(orgs[1]+0x2c,len(c['alternate']))
 m.w(mind,0xa335a0);m.w(mind+4,actors[c['initial_actor']])
 def identity(ptr):return actors.index(ptr)
 def enter(mm):
  nonlocal calls
  calls+=1;actor=identity(mm.ecx());assert mm.arg(1)==4
  events.append(['reason',actor,4])
  if c['fail_at']==calls:raise Required('reason')
 m.observe(0x4ceef0,enter)
 def returned(mm):
  #Explicit supplied callback effects at the real virtual return, after the
  #unchanged4CEEF0 body has committed its native conditional178 store.
  if calls==1 and c['after_first_actor']is not None:m.w(mind+4,actors[c['after_first_actor']])
  if calls==2:
   mutation=c['mutation']
   if mutation=='change_org_field_only':m.w(m.r(mind+4)+0x7c,orgs[1])
   if mutation=='replace_vector':m.w(orgs[0]+0x28,vectors[2])
   if mutation=='shrink':m.w(orgs[0]+0x2c,1)
   if mutation=='grow':
    m.w(orgs[0]+0x28,vectors[2]);m.w(orgs[0]+0x2c,len(c['alternate']))
   if c['after_member_actor']is not None:m.w(mind+4,actors[c['after_member_actor']])
 m.observe(0x4cef04,returned)
 def die(mm):events.append(['death',identity(mm.ecx())])
 m.observe(0x4d7960,die)
 reason_words={actor+0x178:i for i,actor in enumerate(actors)}
 def write(uc,access,address,size,value,_):
  if address in reason_words:
   assert size==4;writes.append([reason_words[address],value])
 hook=m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 def credit(mm):events.append(['required_credit',identity(mm.ecx())]);raise Required('4D57C0')
 m.seam(0x4d57c0,credit)
 try:m.call(0x522270,mind)
 except Required as e:error=str(e)
 m.u.hook_del(hook)
 return dict(events=events,writes=writes,error=error,reasons=[m.r(a+0x178)for a in actors],current_actor=identity(m.r(mind+4))),m.visited

def death_prefix(m,c):
 m.observers.clear();m.seams.clear()
 actor=m.alloc(0x180);session=m.alloc(0x100);events=[];error=None
 m.w(actor+8,c['flags']);m.w(0xb36550,0 if c['session']is None else session)
 if c['session']is not None:m.w(session+0xe4,c['session'])
 m.observe(0x4d57c0,lambda mm:events.append('credit'))
 def after(mm):events.append(['after_credit',mm.u.reg_read(UC_X86_REG_EAX)]);raise Required('remaining_death')
 m.observe(0x4d7993,after)
 try:m.call(0x4d7960,actor)
 except Required as e:error=str(e)
 return dict(events=events,error=error,flags=m.r(actor+8)),m.visited

def main():
 static=struct.pack('<I',0x4ceef0)
 assert nm.original.range(0xa273f0+0x2c,4)==nm.modified.range(0xa273f0+0x2c,4)==static
 rows=[];death=[];pins={}
 for pc in nm.PRECISIONS:
  machines=[machine(image,pc)for image in ['original','v14']]
  for c in cases():
   results=[run(m,c)for m in machines];assert results[0][0]==results[1][0]
   rows.append(dict(precision=pc,input=c,output=results[0][0]))
   for _,p in results:pins.update(p)
  for c in [dict(flags=1,session=0),dict(flags=0x11,session=3),dict(flags=0x10,session=0),dict(flags=0,session=3),dict(flags=1,session=None),dict(flags=0x10,session=None)]:
   results=[death_prefix(m,c)for m in machines];assert results[0][0]==results[1][0]
   death.append(dict(precision=pc,input=c,output=results[0][0]))
   for _,p in results:pins.update(p)
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 #The observer at this first post-credit instruction stops BEFORE execution;
 #it is a cutoff byte pin, not an executed instruction.
 cutoff=pins.pop(0x4d7993,None)
 assert cutoff is not None
 doc={**nm.header(__file__,__doc__),'rows':rows,'death_prefix':death,
      'seams':['4D57C0 required child only; no positive success'],'virtual_return_effects':'Explicit input-controlled observer effects at4CEF04; original4CEEF0 executes before each effect',
      'static_vtable':[dict(va=hex(0xa273f0+0x2c),bytes=static.hex())],
      'cutoff_pins':[dict(va=hex(0x4d7993),bytes=cutoff.hex(),classification='stopped before execution; positive death remainder unsupported')],
      'instructions':nm.instructions(pins)}
 p=HERE/'native-regiment-orphan-v4.json'
 if '--write'in sys.argv:assert not p.exists();p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),death_prefix=len(death),executions=2*(len(rows)+len(death)),executed_pins=len(pins),sha256=sha(p))))
if __name__=='__main__':main()
