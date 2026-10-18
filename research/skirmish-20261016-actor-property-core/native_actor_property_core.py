"""Original4D4CF0/4D4E50 plus4364C0 execute on separate original/v14 images.
Only allocation/free are native service seams. Named word boundaries are
instruction observers for explicit dependency/mutation probes, not callbacks.
"""
from pathlib import Path
import copy,json,hashlib,random,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from harness import UC_HOOK_CODE,UC_X86_REG_EAX as AX,UC_X86_REG_ECX as CX,UC_X86_REG_EDX as DX,UC_X86_REG_ESI as SI,UC_X86_REG_EDI as DI,UC_X86_REG_EBP as BP
ORIGINAL=h.P
class Dependency(Exception):pass
class Probe(h.Machine):
 def __init__(self,c,image,pc):
  h.P=ORIGINAL if image=='original' else h.V;super().__init__(pc);self.c=c;self.trace=[];self.counts={};self.nodes=[];self.actors=[self.alloc(0x600) for _ in c['actors']];self.defs=[self.alloc(0x600) for _ in c['actors']];self.orgs=[self.alloc(0x80) for _ in c['actors']];self.elements=[self.alloc(0x40) for _ in c['actors']]
  for i,a in enumerate(c['actors']):
   self.w32(self.actors[i]+4,self.defs[i]);self.w32(self.actors[i]+0xa8,0x77777777 if a['builder'] else 0);self.w32(self.defs[i]+0x174,a['flags'])
   if a['members'] is not None:
    slots=self.alloc(4*max(1,len(a['members'])));self.w32(self.actors[i]+0x7c,self.orgs[i]);self.w32(self.orgs[i]+0x28,slots);self.w32(self.orgs[i]+0x2c,len(a['members']))
    for j,v in enumerate(a['members']):self.w32(slots+4*j,self.actors[v] if v is not None else 0)
   if a['element']:
    self.w32(self.actors[i]+0x80,self.elements[i]);self.w32(self.elements[i]+0x10,self.actors[a['parent']] if a['parent'] is not None else 0)
  for i,a in enumerate(c['actors']):
   head,tail=self.make_list(a['list']);self.w32(self.actors[i]+0x108,self.ptr(head));self.w32(self.actors[i]+0x10c,self.ptr(tail))
  for i,a in enumerate(c['actors']):
   head,_=self.make_list(a['source']);self.w32(self.defs[i]+0x504,self.ptr(head))
  self.seams[0x6f7810]=self.allocate;self.seams[0x6f7430]=self.free;self.u.hook_add(UC_HOOK_CODE,self.observe)
 def nid(self,p):return next((i for i,n in enumerate(self.nodes) if n['address']==p),None) if p else None
 def ptr(self,n):return self.nodes[n]['address'] if n is not None else 0
 def make_list(self,values):
  first=len(self.nodes);ids=[]
  for v in values:
   n=len(self.nodes);p=self.alloc(8);self.nodes.append({'address':p,'value_written':True,'next_written':True,'freed':False});self.w32(p,v,0);ids.append(n)
  for a,b in zip(ids,ids[1:]):self.w32(self.ptr(a)+4,self.ptr(b))
  return (ids[0],ids[-1]) if ids else (None,None)
 def mutate(self,a):
  k=a['kind'];v=a.get('value')
  if k=='value':self.w32(self.ptr(a['node']),v);self.nodes[a['node']]['value_written']=True
  elif k=='head':self.w32(self.actors[a['actor']]+0x108,self.ptr(v))
  elif k=='tail':self.w32(self.actors[a['actor']]+0x10c,self.ptr(v))
  elif k=='next':self.w32(self.ptr(a['node'])+4,self.ptr(v));self.nodes[a['node']]['next_written']=True
  elif k=='count':self.w32(self.orgs[a['actor']]+0x2c,v)
  elif k=='member':self.w32(self.r32(self.orgs[a['actor']]+0x28)+4*a['slot'],self.actors[v] if v is not None else 0)
  else:raise AssertionError(k)
 def event(self,name,*args):
  self.trace.append([name,*args]);self.counts[name]=self.counts.get(name,0)+1
  for a in self.c.get('mutations',[]):
   if a['event']==name and a['occurrence']==self.counts[name]:self.mutate(a)
  s=self.c.get('stop')
  if s and s['event']==name and s['occurrence']==self.counts[name]:raise Dependency(name)
 def allocate(self,m):
  actor=self.actors.index(self.u.reg_read(SI));assert self.arg()==8;self.event('allocate_node',actor);n=len(self.nodes);p=self.alloc(8);self.u.mem_write(p,b'\xa5'*8);self.nodes.append({'address':p,'value_written':False,'next_written':False,'freed':False});self.ret(0,p)
 def free(self,m):
  actor=self.actors.index(self.u.reg_read(SI));n=self.nid(self.arg());assert n is not None;self.event('free_node',actor,n);assert not self.nodes[n]['freed'];self.nodes[n]['freed']=True;self.ret(0)
 def observe(self,u,pc,size,_):
  r=u.reg_read;e=self.event
  if pc in [0x4d4d00,0x4d4d50,0x4d4dd4,0x4d4df2]:e('actor_head',self.actors.index(r(SI)))
  elif pc==0x4d4dca:e('actor_head',self.actors.index(r(AX)))
  elif pc in [0x4d4d64,0x4d4e07]:e('actor_tail',self.actors.index(r(SI)))
  elif pc in [0x4d4d0f,0x4d4d6f,0x4d4e12]:e('store_actor_head',self.actors.index(r(SI)),self.nid(r(AX)))
  elif pc in [0x4d4d17,0x4d4d77,0x4d4e1a]:e('store_actor_tail',self.actors.index(r(SI)),self.nid(r(AX)))
  elif pc==0x4d4d0a:e('node_next',self.nid(r(CX)))
  elif pc==0x4d4d7d:e('node_next',self.nid(r(DI)))
  elif pc==0x4d4de5:e('node_next',self.nid(r(AX)))
  elif pc==0x4d4e20:e('node_next',self.nid(r(BP)))
  elif pc==0x4d4d28:e('has_source_component_a8',self.actors.index(r(SI)))
  elif pc==0x4d4d32:e('definition',self.actors.index(r(SI)))
  elif pc==0x4d4d35:e('definition_flags',self.defs.index(r(AX)))
  elif pc==0x4d4d43:e('definition_head_504',self.defs.index(r(AX)))
  elif pc==0x4d4d5d:e('node_value',self.nid(r(DI)))
  elif pc in [0x4d4dde,0x4d4dff]:e('node_value',self.nid(r(BP)))
  elif pc==0x4d4de1:e('node_value',self.nid(r(AX)))
  elif pc in [0x4d4d5f,0x4d4e02]:
   n=self.nid(r(AX));e('store_node_value',n,r(DX) if pc==0x4d4d5f else r(CX));self.nodes[n]['value_written']=True
  elif pc in [0x4d4d61,0x4d4e04]:
   n=self.nid(r(AX));e('store_node_next',n,self.nid(r(BP) if pc==0x4d4d61 else r(DI)));self.nodes[n]['next_written']=True
  elif pc==0x4d4d84:e('organization',self.actors.index(r(SI)))
  elif pc in [0x4364c9,0x4364fc]:e('organization_member_count',self.orgs.index(r(DX)))
  elif pc==0x4364d5:e('organization_member',self.orgs.index(r(AX)),r(DX))
  elif pc==0x4d4dc4:e('organization_member',self.orgs.index(r(CX)),r(AX))
  elif pc==0x4d4e50:e('element',self.actors.index(r(CX)))
  elif pc==0x4d4e5a:e('element_parent',self.elements.index(r(AX)))
 def run(self):
  outcome='ok'
  try:self.call(0x4d4cf0 if self.c['operation']=='refresh' else 0x4d4e50,self.actors[self.c['actor']])
  except Dependency as err:outcome=str(err)
  return {'outcome':outcome,'trace':self.trace,'actors':[{'head':self.nid(self.r32(p+0x108)),'tail':self.nid(self.r32(p+0x10c))} for p in self.actors],'nodes':[{'value':self.r32(n['address']) if n['value_written'] else None,'next':self.nid(self.r32(n['address']+4)) if n['next_written'] else None,'next_written':n['next_written'],'freed':n['freed']} for n in self.nodes]}

def actor(values=(),source=(),builder=False,flags=0,members=None,element=False,parent=None):return {'list':list(values),'source':list(source),'builder':builder,'flags':flags,'members':members,'element':element,'parent':parent}
def cases():
 rows=[]
 def add(name,actors,op='refresh',**kw):rows.append({'name':name,'actors':actors,'operation':op,'actor':0,**kw})
 add('empty_constructor',[actor()]);add('clear_only',[actor([1,2,3])]);add('own_duplicates',[actor([9,8],[1,2,1],True,1<<18)])
 add('builder_flag_gate',[actor([9],[1,2],True,0)]);add('absent_builder_gate',[actor([9],[1,2],False,1<<18)])
 add('member_union',[actor([9,8],[1,2,1],True,1<<18,[1,None,2]),actor([2,3,2]),actor([4,3,0])])
 add('recursive_parent',[actor([8],[1,2,1],True,1<<18,None,True,1),actor([9],[],False,0,[0,None,2],True,3),actor([2,4]),actor([99],[],False,0,[1])])
 add('parent_only_null',[actor([8])],op='parent');add('parent_only_no_parent',[actor([8],element=True)],op='parent')
 add('parent_only_propagate',[actor([8,7],element=True,parent=1),actor([9],members=[0])],op='parent')
 rng=random.Random(0x4d4cf0)
 for i in range(20):
  members=[1,None,2,3] if i%2 else None
  add(f'generated_{i}',[actor([rng.randrange(8) for _ in range(i%4)], [rng.randrange(7) for _ in range(i%5)],bool(i%3),1<<18 if i%4 else 0,members)]+[actor([rng.randrange(7) for _ in range(rng.randrange(6))]) for _ in range(3)])
 # Explicit allocator and free mutations, observed only at named required seams.
 add('allocation_changes_source_value',[actor([], [1,2],True,1<<18)],mutations=[{'event':'allocate_node','occurrence':1,'kind':'value','node':0,'value':0x12345678}])
 add('free_changes_current_head',[actor([8,9,10])],mutations=[{'event':'free_node','occurrence':1,'kind':'head','actor':0,'value':2}])
 add('allocation_changes_member_count',[actor([],[],False,0,[1,2]),actor([4]),actor([5])],mutations=[{'event':'allocate_node','occurrence':1,'kind':'count','actor':0,'value':1}])
 # Dependency stops at every boundary/occurrence of a full union case and a parent case.
 for baseindex in [5,6]:
  probe=rows[baseindex];out=Probe(probe,'original','Double53').run();seen={}
  for event in out['trace']:
   k=event[0];seen[k]=seen.get(k,0)+1;c=copy.deepcopy(probe);c['name']=f'stop_{baseindex}_{k}_{seen[k]}';c['stop']={'event':k,'occurrence':seen[k]};rows.append(c)
 return rows

def main():
 inputs=cases();rows=[]
 for pc in h.PRECISION:
  for c in inputs:
   a=Probe(c,'original',pc).run();b=Probe(c,'v14',pc).run();assert a==b,(pc,c['name']);rows.append({'input':c,'precision':pc,'output':a})
 out={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'rows':len(rows),'native_executions':len(rows)*2,'cases':rows,'instructions':[{'va':hex(k),'bytes':v} for k,v in sorted(h.VISITED.items())]}
 dest=HERE/'native-actor-property-core.json'
 if '--write' in sys.argv:dest.write_text(json.dumps(out,indent=1)+'\n')
 else:assert json.loads(dest.read_text())==out
 print(json.dumps({'rows':len(rows),'native_executions':len(rows)*2,'instructions':len(h.VISITED),'sha256':hashlib.sha256(dest.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
