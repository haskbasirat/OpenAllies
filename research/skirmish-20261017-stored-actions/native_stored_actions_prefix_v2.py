"""Original/v14 whole4D1350 on actual captured stored Actor field inputs.

These are explicit direct-call saved-prefix probes, not an initialized map or
natural fresh-HQ grant. Actual current component/source fields are transported
into fixture memory. State registration is reused;4B2E60 returns the captured
source Bool cell. Positive5827B0, owner-view and renderer dependencies STOP at
entry; they never return success. Ground contributors and publication execute
their real native bodies. No source loader/lifecycle completion is claimed.
"""
from pathlib import Path
import sys,json,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
from unicorn import UC_HOOK_CODE
REG=ROOT/'research/phase7-actors/native-actor-states.json'
class Required(Exception):pass
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()

def one(row,image):
 original=n.original;n.original=original if image=='original'else n.modified
 try:m=n.Machine(row['precision'])
 finally:n.original=original
 c=row['input'];actor=m.alloc(0x200);data=m.alloc(0x600);vt=m.alloc(0x200)
 m.w(actor,vt);m.w(actor+4,data);m.w(actor+8,c['flags'])
 for off,entry in {0x34:0x4d1f90,0x100:0x4ced60,0x108:0x4d1e80,0x158:0x4ce980,0x15c:0x4ce9a0}.items():m.w(vt+off,entry)
 for i,v in enumerate(c['actions']):m.w(actor+0xf0+i*4,v)
 for i,v in enumerate(c['state']):m.w(actor+0x100+i*4,v)
 m.w(actor+0xe8,m.alloc(0x400)if c['owner']else 0);m.wb(actor+0x4c,c['object4c'])
 m.w(actor+0xc,m.alloc(0x80)if c['renderer']else 0)
 components={}
 for slot in c['components']:
  component=m.alloc(0x200);m.w(component+4,actor);m.w(actor+0x60+4*slot,component);components[slot]=component
 if c.get('body')is not None:
  m.w(components[0]+0x10,c['body'][0]);m.w(components[0]+0x14,c['body'][1])
 if c.get('element')is not None:
  v=c['element'];assert v['organization']is None,'positive Organization needs same full context'
  m.w(components[8]+0x14,1 if v['definition']else 0);m.w(components[8]+0x18,0);m.wb(components[8]+0x1c,int(v['ready']))
 if c.get('mind')is not None:
  mind_vt=m.alloc(0x100);m.w(components[4],mind_vt)
  m.w(mind_vt+0xb4,{'Element':0x530510,'Regiment':0x521e10,'Aircraft':0x518720}[c['mind']['kind']])
 if c.get('mobile')is not None:
  v=c['mobile'];m.w(data+0x548,1 if v['pack']else 0);m.w(data+0x534,1 if v['unpack']else 0);m.w(data+0x54c,v['health_min'])
 if c['properties_flags']is not None:m.w(components[6]+0x1c,c['properties_flags'])
 if c['structure']is not None:
  s=c['structure'];assert s['parent']==0,'positive parent needs actual second stored Actor fixture'
  m.w(components[13]+0x10,0);m.w(components[13]+0x18,s['queue'])
 if c['construction']is not None:
  v=c['construction'];m.w(data+0x4c0,v['upgrade_count']);m.wb(data+0x4e4,int(v['can_sell']));m.wb(data+0x4e5,int(v['can_raze']))
 if c['research']is not None:m.w(data+0x558,1 if c['research']['head']else 0)
 if c['recruit']is not None:
  v=c['recruit'];m.wb(components[16]+0x10,v['byte10']);m.wb(components[16]+0x11,v['byte11'])
  nodes=[m.alloc(8)for _ in v['entries']];m.w(data+0x4ec,nodes[0]if nodes else 0)
  for i,e in enumerate(v['entries']):
   d=m.alloc(0x600);m.w(nodes[i],d);m.w(nodes[i]+4,nodes[i+1]if i+1<len(nodes)else 0)
   m.w(d+0x184,int(e['location']));m.w(d+0x2f0,m.alloc(0x40)if e['layout']else 0)
 if c['denizen']is not None:
  v=c['denizen'];m.wb(components[10]+0x20,v['byte20']);m.wb(components[10]+0x21,v['byte21'])
  m.w(components[10]+0x38,v['flags38']);m.w(components[10]+0x3c,v['flags3c'])
 cell=m.alloc(4);m.wb(cell,int(c['allow_sell_under_siege']));m.seam(0x4b2e60,lambda mm:mm.ret(cell,8))
 registration=json.loads(REG.read_text())['registration'];gd=m.alloc(0x400);table=m.alloc(42*4)
 m.w(0xb36530,gd);m.w(gd+0x2e4,table);m.w(gd+0x2e8,42)
 m.u.mem_write(0xb3ac40,struct.pack('<31Q',*registration['masks']))
 for r in registration['records']:
  state=m.alloc(0x40);i=r['index'];m.w(table+i*4,state);m.w(state+0xc,i)
  m.w(state+0x28,c['types'][i]);m.w(state+0x2c,r['group']);m.wb(state+0x30,r['exclusive'])
 boundaries={0x5827b0:'recruit_required',0x76c7f0:'owner_views_required',0x4cfd0e:'renderer_required',
   0x530510:'mind_required',0x521e10:'mind_required',0x518720:'mind_required'}
 def observe(u,entry,size,_):
  if entry in boundaries:raise Required(boundaries[entry])
  if entry==0x57b390:raise AssertionError('unexpected Organization scope')
 m.u.hook_add(UC_HOOK_CODE,observe)
 result='ok'
 try:m.call(0x4d1350,actor,[])
 except Required as e:result=str(e)
 out=dict(outcome=result,actions=[m.r(actor+0xf0),m.r(actor+0xf4)])
 pins={a:b for a,b in m.visited.items()if a<0x10000000 and a not in boundaries and a!=0x4b2e60}
 stopped={a:b for a,b in m.visited.items()if a in boundaries}
 return out,pins,stopped

def main():
 source=HERE/'actual-inputs-v2.json';inputs=json.loads(source.read_text());rows=[];pins={};stopped={}
 for row in inputs['rows']:
  a,pa,sa=one(row,'original');b,pb,sb=one(row,'v14');assert a==b
  assert a==row['output'],(row['input']['source'],a,row['output'])
  pins.update(pa);pins.update(pb);stopped.update(sa);stopped.update(sb)
  rows.append(dict(precision=row['precision'],input=row['input'],output=a))
 assert rows
 for a,b in {**pins,**stopped}.items():assert n.original.range(a,len(b))==n.modified.range(a,len(b))==b
 doc=dict(scope=__doc__,input_sha256=sha(source),registration_sha256=sha(REG),producer_sha256=sha(Path(__file__)),rows=rows,
  executed_pins=[dict(va=hex(a),bytes=b.hex())for a,b in sorted(pins.items())],
  required_boundary_entry_pins=[dict(va=hex(a),bytes=b.hex())for a,b in sorted(stopped.items())],
  successful_service_seams=['4B2E60 typed captured Bool cell only'])
 path=HERE/'native-stored-actions-v2.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),required_boundaries=len(stopped),sha256=sha(path))))
if __name__=='__main__':main()
