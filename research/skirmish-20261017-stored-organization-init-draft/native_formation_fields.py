"""Actual577F30/57BF50/57BFF0 fields with explicit state/property/refresh children.
Enum48A440/48A540 actual constructor registration arguments are observed at
47A360 (declared metadata child), not claimed as a second live enum manager.
No live Organization/source-world authority is supplied by these lab inputs.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
H=lambda b:hashlib.sha256(b).hexdigest()

def text(m,p):
 out=[]
 while m.u.mem_read(p,2)!=b'\0\0':out.append(int.from_bytes(m.u.mem_read(p,2),'little'));p+=2
 return ''.join(map(chr,out))

def enums(m):
 registrations=[]
 m.seams={0x6f7810:lambda x:x.ret(0,x.alloc(x.arg())),
  0x47a360:lambda x:(registrations.append([x.arg(),text(x,x.arg(1)),x.arg(2)]),x.ret(12))}
 for addr in [0x48a440,0x48a540]:m.call(addr,ecx=m.alloc(0x40),args=(1,))
 return registrations

def run(m,c):
 m.next=h.HEAP+0x1000;m.seams={};trace=[];stopped=None
 org=m.alloc(0xdc);owner=m.alloc(0x200);layout=m.alloc(0x60);state=m.alloc(0x40);table=m.alloc(8)
 fp=[m.alloc(0x20),m.alloc(0x20)];fd=[m.alloc(0x30),m.alloc(0x30)]
 m.w32(org+4,owner);m.w32(org+0x18,layout);m.w32(layout+0x34,table)
 m.w32(layout+0x44,0,1);m.w32(layout+0x50,100);m.w32(table,*fp)
 for index in range(2):m.w32(fp[index]+0xc,fd[index]);m.w8(fd[index]+0x1a,index);m.w32(fd[index]+0x24,101+index)
 to_ptr=lambda i:0 if i==0 else fp[i-1]
 m.w32(org+0x70,to_ptr(c['desired']),to_ptr(c['current']))
 for o,v in zip([0x8c,0x8d,0x8e],c['bytes']):m.w8(org+o,v)
 if c['state'] is not None:m.w32(state+0x20,*c['state'])
 def event(e):
  nonlocal stopped
  trace.append(e)
  if c.get('fail_at')==len(trace)-1:stopped='required';m.u.emu_stop();return False
  return True
 def group(x):
  assert x.ecx()==owner and x.arg()==4
  if event(['group',4]):x.ret(4,0 if c['state'] is None else state)
 def has(x):
  assert x.ecx()==owner
  if event(['has',x.arg(),1]):x.ret(8,1 if c['has'] else 0)
 def remove(x):
  assert x.ecx()==owner
  if event(['remove',x.arg(),1]):x.ret(8)
 def apply(x):
  assert x.ecx()==owner
  if event(['apply',x.arg(),x.arg(1),1]):x.ret(12,1)
 def refresh(x):
  assert x.ecx()==org
  if event(['refresh']):
   if c.get('mutate_desired') is not None:m.w32(org+0x70,to_ptr(c['mutate_desired']))
   x.ret()
 def view(x):
  assert x.ecx()==owner
  if event(['view']):x.ret(0,owner if c['view'] else 0)
 m.seams.update({0x4d49b0:group,0x4cef80:has,0x4cef70:remove,0x4cef10:apply,0x57bef0:refresh,0x4d1900:view})
 try:m.call(0x577f30 if c['operation']=='byte' else 0x57bff0,ecx=org,args=(c['value'] if c['operation']=='byte' else to_ptr(c['desired']),))
 except AssertionError:
  if stopped is None:raise
 def to_id(p):return 0 if p==0 else fp.index(p)+1
 return {'trace':trace,'stop':stopped,'desired':to_id(m.r32(org+0x70)),
  'current':to_id(m.r32(org+0x74)),'bytes':[m.r8(org+o)for o in[0x8c,0x8d,0x8e]]}

def cases():
 out=[]
 for mode,value,old in itertools.product([None,0,1,2,3,0xffffffff],[0,1,128],[0,1,128]):
  out.append(dict(operation='byte',desired=1,current=2,bytes=[99,old,77],state=None if mode is None else[0,mode],value=value,has=False,view=False))
 for desired,current,state in itertools.product([0,1,2],[0,1,2],[None,[0,0],[0,2],[1,1],[2,0],[3,2],[4,4]]):
  out.append(dict(operation='formation',desired=desired,current=current,state=state,bytes=[99,0,77],value=0,has=True,view=True))
 for desired in [0,1,2]:out.append(dict(operation='formation',desired=2,current=1,state=[0,2],bytes=[88,128,77],value=0,has=False,view=False,mutate_desired=desired))
 return out

def main():
 rows=[];registrations=[];orig=h.P
 try:
  for label,image in [('original',orig),('v14',h.V)]:
   h.P=image
   for pc in h.PRECISION:
    m=h.Machine(pc);registrations.append({'image':label,'precision':pc,'values':enums(m)})
    for c in cases():rows.append({'image':label,'precision':pc,'input':c,'output':run(m,c)})
    c=next(c for c in cases()if c['operation']=='formation'and c['state']==[0,2]and c['desired']==2 and c['current']==1)
    calls=run(m,c)['trace']
    for i in range(len(calls)):
     cut=dict(c,fail_at=i);rows.append({'image':label,'precision':pc,'input':cut,'output':run(m,cut)})
 finally:h.P=orig
 half=len(rows)//2;assert [{k:v for k,v in r.items()if k!='image'}for r in rows[:half]]==[{k:v for k,v in r.items()if k!='image'}for r in rows[half:]]
 expected=[[0,'DESIRED',0],[1,'BROKEN',0],[2,'COMBAT',0],[3,'RETREAT',0],[0,'PASSIVE',0],[1,'AGGRESSIVE',0],[2,'DESIRED',0]]
 assert all(r['values']==expected for r in registrations)
 seams={0x6f7810,0x47a360,0x4d49b0,0x4cef80,0x4cef70,0x4cef10,0x57bef0,0x4d1900}
 doc={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'executions':len(rows),
  'constructor_machines':registrations,'pins':[{'address':hex(a),'bytes':b,'kind':'declared_child_entry'if a in seams else'executed'}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-formation-fields.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'result':'PASS','rows':len(rows),'constructor_machines':len(registrations),'pins':len(doc['pins']),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
