"""Native4FA3D0 hero postpass after an explicit4FA150 creation service.
Real component/layout/member reads and loop branches execute. Owner virtual108,
recruit4E2A60, hero57B830 and positive notification are explicit child services;
notification skips only4FA4A0..54F localized formatting/temporary destruction.
No actual actor creation, Kingdom hero insertion or UI completion is claimed.
"""
from pathlib import Path
import sys,json,hashlib,itertools,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_EBP,UC_X86_REG_ESI,UC_X86_REG_ECX,UC_X86_REG_EDX
class Required(Exception):pass

def cases():
 base={'created':True,'groups':[0,1,2,-1],'members':[1,2,3,4],'owners1':[1,1,1,1],'table2':[1,1,1,1],'recruit':True,'attach':False}
 out=[]
 for created,groups,member,owner,table,recruit,attach in [(False,[],0,0,0,False,False),(True,[],0,0,0,False,False)]+list(itertools.product([True],[[-1],[0],[1],[2]], [0,1],[0,1],[0,1],[False,True],[False,True])):
  c=copy.deepcopy(base);c.update(created=created,groups=groups,members=[member]*len(groups),owners1=[owner]*4,table2=[table]*4,recruit=recruit,attach=attach);out.append(c)
 out.append(base)
 for child in ['recruit','attach','notify']:
  c=copy.deepcopy(base);c.update(attach=True,fail=child);out.append(c)
 for action in ['switch_org','shorten_layout','definition_after_owner']:
  c=copy.deepcopy(base);c.update(attach=True,action=action);out.append(c)
 return out

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 root=m.alloc(0x100);orgs=[m.alloc(0x40)for _ in range(2)];layouts=[m.alloc(0x40)for _ in range(2)];m.w(root+0x7c,orgs[0])
 groups=c['groups'];members=[m.alloc(0x114)for _ in range(4)];memberids={p:i+1 for i,p in enumerate(members)}
 definitions={i:m.alloc(0x20)for i in [11,12,13,14,99]};definitionids={p:i for i,p in definitions.items()}
 memberdefs=[11,12,13,14];table=m.alloc(0x250);owner=table
 events=[];attached=[];counts={p:0 for p in members};action_done=False
 def event(label,*args):
  events.append([label,*args])
  if c.get('fail')==label:raise Required(label)
 def slot(mm):return m.r(mm.u.reg_read(UC_X86_REG_ESP)+0x14)
 for n,layout in enumerate(layouts):
  groupdata=m.array([v&0xffffffff for g in groups for v in [0,0,g,0]])
  m.w(layout+0x1c,groupdata);m.w(layout+0x28,m.array(range(len(groups))));m.w(layout+0x2c,len(groups) if n==0 else min(2,len(groups)))
  m.w(orgs[n]+0x18,layout)
  ids=c['members'] if n==0 else list(reversed(c['members']))
  m.w(orgs[n]+0x28,m.array([members[i-1]if i else 0 for i in ids]))
 for i,p in enumerate(members):
  vt=m.alloc(0x110);m.w(p,vt);m.w(p+4,definitions[memberdefs[i]])
  def current_owner(mm,index=i,pointer=p):
   first=counts[pointer]%2==0;counts[pointer]+=1;event('owner1'if first else'owner2',index+1)
   if first:
    value=owner if c['owners1'][index]else 0
    if c.get('action')=='definition_after_owner':m.w(pointer+4,definitions[99]);memberdefs[index]=99
   else:value=owner if c['table2'][index]else 0xfffffe04
   mm.ret(value)
  m.w(vt+0x108,m.stub(current_owner))
 m.seam(0x4fa150,lambda mm:mm.ret(root if c['created']else 0,4))
 m.observe(0x4fa404,lambda mm:event('capture',1))
 m.observe(0x4fa40a,lambda mm:event('count',0))
 m.observe(0x4fa55a,lambda mm:event('count',0))
 m.observe(0x4fa434,lambda mm:event('group',0,slot(mm)))
 m.observe(0x4fa445,lambda mm:event('member',1,slot(mm)))
 m.observe(0x4fa453,lambda mm:event('definition',memberids[mm.u.reg_read(UC_X86_REG_EBP)]))
 def recruit(mm):
  assert mm.ecx()==owner+0x1fc;event('recruit',1,definitionids[mm.arg(1)],mm.arg(2));mm.ret(int(c['recruit']),8)
 m.seam(0x4e2a60,recruit)
 def attach(mm):
  nonlocal action_done
  assert mm.ecx()==m.r(root+0x7c);d=definitionids[mm.arg(1)];event('attach',1,d)
  if c['attach']:attached.append(d)
  if not action_done:
   if c.get('action')=='switch_org':m.w(root+0x7c,orgs[1])
   if c.get('action')=='shorten_layout':m.w(orgs[0]+0x18,layouts[1])
   action_done=True
  mm.ret(int(c['attach']),4)
 m.seam(0x57b830,attach)
 def notify(mm):
  event('notify',1,definitionids[mm.u.reg_read(UC_X86_REG_ESI)]);mm.u.reg_write(UC_X86_REG_EIP,0x4fa54f)
 m.seam(0x4fa4a0,notify)
 error=None;result=None
 try:result=1 if m.call(0x4fa3d0,0,[m.alloc(0x60)])else None
 except Required as e:error=str(e)
 return dict(result=result,error=error,events=events,attached=attached,member_definitions=memberdefs),m.visited

def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'notification_service_skip':['4FA4A0','4FA54F']}
 p=HERE/'native-hero-postpass.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
