"""Full original4F4590->4F3640 with captured first-Team input fields.

Explicit six-zero point words are a geometry transport probe, NOT a proof of
the global's startup seed. Runtime admission separately requires PointShape.
Actual Team/TeamData vtables and leaves execute; Object/Actor storage reads
and all five positive placement child calls are forbidden.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE, UC_HOOK_MEM_READ
ORIGINAL=h.P
H=lambda b:hashlib.sha256(b).hexdigest()
FORBIDDEN={0x4f1660,0x4f0ae0,0x4f2220,0x4f0a70,0x4f0b70}

def run(c,extents,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 d=m.alloc(0x600);o=m.alloc(0x200);world=m.alloc(0x600);settings=m.alloc(0x200);map_=m.alloc(0x100)
 m.w32(0xb36534,world);m.w32(world+0x28,settings);m.w32(settings+0xa8,1);m.w32(world+0x30,map_)
 # This first Team is at XY0. Native point outside checks need exact extents;
 # the skipped cell-size/snap branches must not read GMap+8.
 m.w32(map_+0xc,*extents)
 m.w32(0xb3a8c4,*([0]*6))
 m.w32(d,0xa20770);m.w32(d+0x1c,c['definition_flags'])
 m.w32(o,0xa29780,d,c['runtime_flags_before_init'])
 m.w32(o+0x20,*c['position_bits']);m.w32(o+0x2c,*c['facing_bits'])
 allocator=m.alloc(0x20);vt=m.alloc(0x20);header=m.alloc(0x20)
 m.w32(allocator,vt);m.w32(header,allocator,0,16,1)
 events=[];gacd=None;before=None
 def stub(fn):
  a=m.stub_next;m.stub_next+=16;m.u.mem_write(a,b'\xc3');m.seams[a]=fn;return a
 def alloc(_):assert m.arg()==2;m.ret(0,allocator)
 def empty(_):events.append('empty');m.ret(0,header)
 def release(_):assert m.arg()==header;events.append('release');m.ret(4,0)
 m.seams[0x856370]=alloc;m.w32(vt+0xc,stub(empty));m.w32(vt+4,stub(release))
 objbefore=bytes(m.u.mem_read(o,0x200));defbefore=bytes(m.u.mem_read(d,0x600))
 def observe(u,a,n,_):
  nonlocal gacd,before
  assert a not in FORBIDDEN,hex(a)
  if a==0x4f3640:
   gacd=m.arg();before=bytes(m.u.mem_read(gacd,0x53))
   assert [m.arg(i)for i in range(1,7)]==[o,o,0,0,0,0]
   assert m.r32(gacd+4)==d and m.r32(gacd+0x28)==0
   assert m.r32(gacd+8,3)==c['position_bits']
   assert m.r32(gacd+0x14,2)==c['facing_bits'][:2]
 def read(u,access,address,size,value,_):
  assert not(o+0x38<=address<o+0x200),'read invented Object/Actor storage'
  assert not(d+0x20<=address<d+0x600),'read invented ObjectData/ActorData storage'
  assert address!=map_+8,'Team unexpectedly reads snap/cell-size branch'
 m.u.hook_add(UC_HOOK_CODE,observe);m.u.hook_add(UC_HOOK_MEM_READ,read)
 result=m.call(0x4f4590,world,[o,0,0,0,0])
 assert result==0 and m.fpu_top()==0 and m.final_esp==h.STACK+24
 assert events==['empty','release']and m.r32(header+0xc)==0
 assert bytes(m.u.mem_read(gacd,0x53))==before
 assert bytes(m.u.mem_read(o,0x200))==objbefore
 assert bytes(m.u.mem_read(d,0x600))==defbefore
 return dict(result=result,descriptor_unchanged=True,thing_unchanged=True,source_unchanged=True,
             positive_child_calls=0,object_actor_storage_reads=0,name_events=events)

def main():
 capture=ROOT/'research/skirmish-20261016-live-map-startup/actual-first-records-v2.json'
 inputs=json.loads(capture.read_text());rows=[]
 for map_ in inputs['maps']:
  for record in map_['records']:
   if record['native_class']!=5:continue
   for pc in h.PRECISION:
    a=run(record,map_['world_extents_bits'],'original',pc)
    b=run(record,map_['world_extents_bits'],'v14',pc);assert a==b
    rows.append(dict(map=map_['map'],map_source=map_['map_source'],input=record,
                     precision=pc,point_transport_probe=[0]*6,output=a))
 assert len(rows)==6
 doc=dict(schema=1,scope=__doc__,capture_sha256=H(capture.read_bytes()),
          original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),cases=rows,
          native_executions=len(rows)*2,
          instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 path=HERE/'native-team-placement.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=H(path.read_bytes()))))
if __name__=='__main__':main()
