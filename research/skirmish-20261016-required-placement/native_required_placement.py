"""Untouched4F1660/46E080/553370/4CF210/54ECE0, original/v14 and all3PC.
Explicit seams: definition/actor virtuals, tile iterator, zone query/overlap,
46EF30 overlap result,4D1120 mobility and GetCurrentThreadId. Actual node,
source-field, metadata and generation-table reads execute. No startup proof.
"""
from pathlib import Path
import copy, hashlib, json, sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from unicorn import UC_HOOK_CODE
ORIGINAL=h.P
class Required(Exception):pass

def run(c,image,pc):
 h.P=ORIGINAL if image=='original' else h.V
 m=h.Machine(pc);c=copy.deepcopy(c);events=[]
 world=m.alloc(0x100);map_=m.alloc(0x100);tiles=m.alloc(0x300)
 manager=m.alloc(0xa0);app=m.alloc(0x100);sim=m.alloc(0x100);gtids=m.alloc(0x60000)
 m.w32(0xb39f58,manager);m.w32(manager+0x74,c['radius'])
 m.w32(0xb36550,app);m.w32(app+0xe4,c['game_mode'])
 m.w32(0xb364d4,sim);m.w32(sim+0x48,77);m.w32(0xb364c0,gtids)
 m.w32(0xb28158,c['global_mode']);m.w32(world+0x30,map_)
 m.w32(map_+4,2,h.f2b(4),*c['extents']);m.w32(map_+0x1c,8,8);m.w32(map_+0x28,tiles)
 definitions={n:m.alloc(0x600) for n in range(1,6)}
 actors={n:m.alloc(0x600) for n in range(1,5)}
 shapes={n:m.alloc(24) for n in actors}
 revd={v:k for k,v in definitions.items()};reva={v:k for k,v in actors.items()}
 revshape={v:k for k,v in shapes.items()}
 cursor=0;iterator=None
 def event(name,*args):
  events.append([name,*args])
  if c.get('failure')==name:raise Required(name)
  mutation=c.get('mutation')
  if mutation and mutation['event']==name and ('args' not in mutation or mutation['args']==list(args)):
   actor=mutation['actor']
   for field,offset in [('flags',8),('reference',0x50)]:
    if field in mutation:
     value=mutation[field];c['actors'][str(actor)][field]=value;m.w32(actors[actor]+offset,value)
 def stub(fn):
  p=m.stub_next;m.stub_next+=16;m.u.mem_write(p,b'\xc3');m.seams[p]=fn;return p
 def dvirt(_):event('definition_actor',1);m.ret(value=c['definition_actor'])
 for n,d in definitions.items():
  vt=m.alloc(0x50);m.w32(d,vt);m.w32(vt+0x38,stub(dvirt))
  m.w32(d+0x174,c['mask'] if n==1 else 0)
  m.w32(d+0x4e0,definitions[c['required']] if n==(2 if c['settlement'] else 1) and c['required'] else 0)
  if n==1:
   settlement=m.alloc(8);m.w32(settlement+4,definitions[2]);m.w32(d+0x464,settlement)
 for n,a in actors.items():
  value=c['actors'][str(n)];d=definitions[value['definition']]
  m.w32(a+4,d,value['flags']);m.w32(a+0x20,h.f2b(n),h.f2b(n));m.w32(a+0x2c,0,0xbf800000)
  m.w32(a+0x50,value['reference']);m.w32(d+0x430,value['word430']);m.w8(d+0x1a8,value['byte1a8'])
  m.w32(shapes[n],1,h.f2b(1),0,0,0,0)
  vt=m.alloc(0x200);m.w32(a,vt)
  for slot in [0xb0,0xb4,0x15c,0x100,0x140,0xc]:
   def virtual(_,n=n,slot=slot):
    value=c['actors'][str(n)];event('virtual',n,slot)
    if slot==0xc:result=shapes[n]
    elif slot in [0x15c,0x100]:result=actors.get(value['parent15c' if slot==0x15c else 'parent100'],0)
    else:result=value[{0xb0:'b0',0xb4:'b4',0x140:'enemy'}[slot]]
    m.ret(4 if slot==0x140 else 0,result)
   m.w32(vt+slot,stub(virtual))
 for key,entry in c['registry'].items():
  reference=int(key);slot=reference&0xffff;generation=reference>>16
  m.u.mem_write(gtids+4+slot*2,((generation+entry['generation_delta'])&0xffff).to_bytes(2,'little'))
  m.w32(gtids+0x20004+slot*4,actors.get(entry['actor'],0))
 for i,t in enumerate(c['tiles']):
  tile=tiles+i*0x30;variation=m.alloc(0x24);terrain=m.alloc(0x60)
  m.w32(tile,variation);m.w32(variation+0x1c,terrain);m.w8(terrain+0x4c,t['land'])
  head=0
  for n in reversed(t['nodes']):node=m.alloc(8);m.w32(node,actors[n],head);head=node
  m.w32(tile+8,head)
 zone=m.alloc(8);m.w32(zone+4,actors[3])
 def begin(_):
  nonlocal iterator,cursor
  iterator=m.ecx();cursor=0;event('begin')
  m.w32(iterator,map_);m.w32(iterator+0x10,0);m.w32(iterator+0x24,0);m.w32(iterator+0x28,0 if c['tiles'] else 1)
  m.ret(16,iterator)
 def advance(_):
  nonlocal cursor
  event('advance');cursor+=1;m.w32(iterator+0x24,cursor);m.w32(iterator+0x28,0 if cursor<len(c['tiles']) else 1);m.ret(4,0)
 def query(_):
  event('zone',cursor,reva.get(m.arg(1),0),revd.get(m.arg(2),0));assert m.arg(0)==2
  m.ret(12,zone if c['tiles'][cursor]['zone'] else 0)
 def zone_overlap(_):event('zone_overlap',reva[m.arg(0)]);m.ret(8,c['zone_overlap'])
 def overlap(_):
  actor=revshape[m.r32(m.arg())];event('overlap',actor);m.ret(4,c['actors'][str(actor)]['overlap'])
 def mobile(_):actor=reva[m.ecx()];event('mobile',actor);m.ret(value=c['actors'][str(actor)]['mobile'])
 def thread(_):event('thread');m.ret(value=77 if c['simulation_thread'] else 88)
 m.seams.update({0x554dd0:begin,0x554330:advance,0x54ef80:query,0x4f0ae0:zone_overlap,
                 0x46ef30:overlap,0x4d1120:mobile})
 m.w32(0x9f01e4,stub(thread))
 def observe(u,address,size,_):
  if address==0x54ece0:event('terrain',cursor)
 m.u.hook_add(UC_HOOK_CODE,observe)
 result=None;outcome='ok'
 try:
  result=m.call(0x4f1660,world,[*c['position'],definitions[1] if c['definition'] else 0,c['owner'],
      actors.get(c['other'],0),c['argument7'],c['argument8']])
  assert m.final_esp==h.STACK+32
  assert m.fpu_top()==0
 except Required as e:outcome=str(e)
 return {'result':result,'outcome':outcome,'events':events}

def cases():
 actor=dict(definition=4,flags=0,b0=1,b4=1,parent15c=0,parent100=0,reference=0,byte1a8=0,word430=0,overlap=1,mobile=0,enemy=0)
 b=dict(label='empty',radius=h.f2b(2),position=[h.f2b(10),h.f2b(10)],extents=[h.f2b(32)]*2,
        definition=True,definition_actor=1,mask=0,settlement=False,required=0,owner=17,other=0,argument7=0,argument8=0,
        actors={str(n):dict(actor,definition=4 if n==2 else 5) for n in range(1,5)},
        registry={},tiles=[],zone_overlap=1,game_mode=0,global_mode=0,simulation_thread=False)
 def row(label,**kw):
  c=copy.deepcopy(b);c.update(kw);c['label']=label;return c
 yield row('empty')
 yield row('outside',position=[h.f2b(-1),h.f2b(10)])
 yield row('inside_empty',tiles=[dict(land=1,zone=False,nodes=[])])
 yield row('water',tiles=[dict(land=2,zone=False,nodes=[])])
 for a7 in [1,0x100,0xffffffff]:yield row('argument7_'+str(a7),argument7=a7,tiles=[dict(land=0,zone=False,nodes=[])])
 for da in [0,0x100]:yield row('definition_actor_'+str(da),definition_actor=da,tiles=[dict(land=1,zone=True,nodes=[])])
 for game,thread in [(0,False),(3,False),(3,True)]:
  yield row(f'zone_exclusion_{game}_{thread}',required=3,game_mode=game,simulation_thread=thread,tiles=[dict(land=1,zone=True,nodes=[])])
 yield row('settlement_required',mask=1<<14,settlement=True,required=3,tiles=[dict(land=1,zone=True,nodes=[])])
 yield row('zone_no_overlap',zone_overlap=0,tiles=[dict(land=1,zone=True,nodes=[]),dict(land=2,zone=False,nodes=[])])
 yield row('null_definition',definition=False,tiles=[dict(land=1,zone=False,nodes=[])])
 for ref,delta,target in [(0x10002,0,4),(0x10002,1,4),(0x10002,0,0)]:
  c=row(f'reference_{delta}_{target}',other=1,tiles=[dict(land=1,zone=True,nodes=[])])
  c['actors']['1']['reference']=ref;c['registry'][str(ref)]=dict(generation_delta=delta,actor=target);yield c
 for b0,b4,parent in [(0,0,0),(1,0,0),(1,1,3),(0x100,0x100,3)]:
  c=row(f'other_{b0}_{b4}_{parent}',other=1,tiles=[dict(land=1,zone=False,nodes=[])])
  c['actors']['1'].update(b0=b0,b4=b4,parent15c=parent);yield c
 base=row('object',tiles=[dict(land=1,zone=False,nodes=[2])])
 base['actors']['2']['flags']=1<<10
 yield copy.deepcopy(base)
 for field,values in [('flags',[0,1<<10,(1<<10)|(1<<11),(1<<10)|(1<<29)]),('b4',[0,0x100]),
                      ('byte1a8',[1]),('overlap',[0]),('mobile',[1]),('word430',[1]),('enemy',[1])]:
  for value in values:
   c=copy.deepcopy(base);c['label']=f'actor_{field}_{value}';c['actors']['2'][field]=value;yield c
 for a8 in [0,1,0x100,0xffffffff]:
  c=copy.deepcopy(base);c['label']='enemy_'+str(a8);c['argument8']=a8
  c['actors']['2'].update(flags=(1<<10)|(1<<29),enemy=1);yield c
 c=copy.deepcopy(base);c['label']='nonactor_global_enemy';c['global_mode']=3;c['actors']['2'].update(b4=0,flags=(1<<10)|(1<<29));yield c
 c=copy.deepcopy(base);c['label']='same_actor';c['other']=2;yield c
 c=copy.deepcopy(base);c['label']='same_definition';c['required']=4;yield c
 c=copy.deepcopy(base);c['label']='same_cached_reference';c['other']=1;c['actors']['1']['reference']=0x10002;c['registry'][str(0x10002)]=dict(generation_delta=0,actor=2);yield c
 c=copy.deepcopy(base);c['label']='same_parent';c['other']=1;c['actors']['2']['parent100']=1;yield c
 c=copy.deepcopy(base);c['label']='mobile_required';c['actors']['2'].update(mobile=1,word430=1);yield c
 c=copy.deepcopy(base);c['label']='late_solid_capture';c['mutation']=dict(event='mobile',actor=2,flags=(1<<10)|(1<<11));yield c
 c=copy.deepcopy(base);c['label']='late_bit29_capture';c['actors']['2'].update(flags=(1<<10)|(1<<29),enemy=1);c['mutation']=dict(event='virtual',args=[2,0x140],actor=2,flags=1<<10);yield c
 c=copy.deepcopy(base);c['label']='mobile_changes_bit29';c['actors']['2']['enemy']=1;c['mutation']=dict(event='mobile',actor=2,flags=(1<<10)|(1<<29));yield c
 c=row('reference_changes_between_tiles',other=1,tiles=[dict(land=1,zone=True,nodes=[]),dict(land=1,zone=True,nodes=[])])
 c['actors']['1']['reference']=0x10002;c['registry'][str(0x10002)]=dict(generation_delta=0,actor=4)
 c['mutation']=dict(event='advance',actor=1,reference=0x20002);yield c
 c=copy.deepcopy(base);c['label']='node_order';c['tiles'][0]['nodes']=[1,2,3];c['actors']['2']['flags']|=1<<11;yield c
 # One failing prefix for every reached service-event name.
 prefixes=[row('failure_zone',game_mode=3,tiles=[dict(land=1,zone=True,nodes=[])]),base]
 seen=set()
 for c in prefixes:
  for event in run(c,'original','Double53')['events']:
   name=event[0]
   if name in seen:continue
   seen.add(name);v=copy.deepcopy(c);v.update(label='failure_'+name,failure=name);yield v

def main():
 rows=[];inputs=list(cases())
 for pc in h.PRECISION:
  for c in inputs:
   a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b,(pc,c['label'],a,b)
   rows.append(dict(precision=pc,input=c,output=a))
 leaves={0x554dd0,0x554330,0x54ef80,0x4f0ae0,0x46ef30,0x4d1120}
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
          producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
          cases=rows,native_executions=len(rows)*2,
          executed_image_instructions=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items()) if a not in leaves],
          intercepted_image_entries=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items()) if a in leaves])
 path=HERE/'native-required-placement.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,executed_addresses=len(doc['executed_image_instructions']),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
