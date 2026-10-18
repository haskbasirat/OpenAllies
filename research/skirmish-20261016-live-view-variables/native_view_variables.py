"""Native FogOfWar/CheatFog source-default reset and mutable publication.
Actual401940/4017D0 constructors,474C50 WorldRule defaults,4B3580 snapshot
reset,512200/512160 bounded Int and41C670 Bool assignments execute. Only
CString allocation/assignment, named-Var lookup and diagnostics are seams;
the provided WorldRule descriptor is a source input, not a TGI loader claim.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
HASH=lambda b:hashlib.sha256(b).hexdigest()

def run(c,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 fog=m.alloc(0x30);cheat=m.alloc(0x30);fast=m.alloc(0x30);events=[]
 def text(s):
  header=m.alloc(0x100);m.w32(header+12,2);p=header+16;m.u.mem_write(p,s.encode('utf-16le')+b'\0\0');return p
 names={k:text(k)for k in ['FogOfWar','CheatFog','ZoneBuilderFastDetectionZones']}
 allocator=m.alloc(0x20);vt=m.alloc(0x20);m.w32(allocator,vt)
 stub=m.stub_next;m.stub_next+=16;m.u.mem_write(stub,b'\xc3');m.w32(vt+12,stub)
 empty=m.alloc(0x20)
 m.seams[stub]=lambda _:m.ret(value=empty)
 m.seams[0x856370]=lambda _:m.ret(value=allocator)
 m.seams[0x4012c0]=lambda _:m.ret(value=m.arg())
 def assign(_):m.w32(m.ecx(),m.r32(m.arg()));m.ret(4,m.ecx())
 m.seams[0x405b80]=assign
 cells={k:m.alloc(4)for k in names}
 for name,var in [('FogOfWar',fog),('CheatFog',cheat),('ZoneBuilderFastDetectionZones',fast)]:m.w32(cells[name],var)
 def lookup(_):
  assert m.ecx()==0xb39abc,'lookup left the actual current Settings hash-map subobject'
  nameptr=m.r32(m.arg());name=next(k for k,v in names.items()if v==nameptr)
  events.append('lookup:'+name);m.ret(4,cells[name])
 m.seams[0x405370]=lookup
 m.seams[0x404f60]=lambda _:m.ret()
 m.call(0x401940,fog,[names['FogOfWar'],2,0,2,0]);assert m.final_esp==h.STACK+24
 m.call(0x4017d0,cheat,[names['CheatFog'],0,0]);assert m.final_esp==h.STACK+16
 m.call(0x4017d0,fast,[names['ZoneBuilderFastDetectionZones'],c['fast_source'],0]);assert m.final_esp==h.STACK+16
 ctor=dict(fog=m.r32(fog),cheat=m.r8(cheat),fog_type=m.r32(fog+0x24),fog_flags=m.r32(fog+0x28),cheat_type=m.r32(cheat+0x24),cheat_flags=m.r32(cheat+0x28))
 # Registered FogOfWar enum descriptor; real474C50 sets bounds and default.
 manager=m.alloc(0x600);array=m.alloc(4);definition=m.alloc(0x60)
 m.w32(0xb36530,manager);m.w32(manager+0x118,array,1);m.w32(array,definition)
 m.w32(definition+8,names['FogOfWar']);m.w32(definition+0x20,3)
 m.w32(definition+0x38,3);m.w32(definition+0x40,c['source_default'])
 m.call(0x474c50);assert m.final_esp==h.STACK+4
 source_fog=m.r32(fog)
 # Source snapshot is the real post-WorldRule Var contents, not constructor2.
 saved_fog=m.alloc(0x30);saved_cheat=m.alloc(0x30);saved_fast=m.alloc(0x30)
 m.u.mem_write(saved_fog,bytes(m.u.mem_read(fog,0x2c)))
 m.u.mem_write(saved_cheat,bytes(m.u.mem_read(cheat,0x2c)))
 m.u.mem_write(saved_fast,bytes(m.u.mem_read(fast,0x2c)))
 snapshot=m.alloc(0x20);sources=m.alloc(12);m.w32(sources,saved_fog,saved_cheat,saved_fast);m.w32(snapshot+8,sources,3)
 # Demonstrate that reset reaches current vars rather than retaining stale UI values.
 m.w32(fog,c['before_reset_fog']);m.w8(cheat,c['before_reset_cheat']);m.w8(fast,c['fast_source']^1)
 m.call(0x4b3580,0xb39aa8,[snapshot]);assert m.final_esp==h.STACK+8
 reset=dict(fog=m.r32(fog),cheat=m.r8(cheat),fast=m.r8(fast))
 # The real WorldRules UpdateSVars runs after RULES Load and before TERRAIN.
 rules=m.alloc(0x20);entry=m.alloc(12);m.w32(rules+12,entry,1)
 m.w32(entry,definition,c['locked'],c['rule_value'])
 m.call(0x512200,rules);assert m.final_esp==h.STACK+4
 published=m.r32(fog)
 # Later live commands target the same actual variable allocations.
 m.call(0x512160,0xb39aa8,[names['FogOfWar'],c['later_fog']]);assert m.final_esp==h.STACK+12
 m.call(0x41c670,0xb39aa8,[names['CheatFog'],c['later_cheat']]);assert m.final_esp==h.STACK+12
 m.call(0x41c670,0xb39aa8,[names['ZoneBuilderFastDetectionZones'],c['later_fast']]);assert m.final_esp==h.STACK+12
 ptr=m.call(0x4b2e60,0xb39aa8,[names['ZoneBuilderFastDetectionZones'],1]);assert ptr==fast and m.final_esp==h.STACK+12
 assert m.fpu_top()==0
 return dict(ctor=ctor,source_fog=source_fog,reset=reset,published_fog=published,
  current_fog=m.r32(fog),current_cheat=m.r8(cheat),current_fast=m.r8(fast),fog_enabled=m.r32(fog)!=0,cheat_enabled=m.r8(cheat)!=0,fast_enabled=m.r8(fast)!=0,events=events)

def cases():
 for value in [0,1,2,3,0xffffffff,0x80000000,0x7fffffff]:
  for lock in [0,1,255]:
   for fast in [0,1]:
    yield dict(source_default=1,before_reset_fog=2,before_reset_cheat=255,locked=lock,rule_value=value,later_fog=value^1,later_cheat=(value^0x80)&255,fast_source=fast,later_fast=value&255)
 for default in [0,1,2,3,0xffffffff]:
  yield dict(source_default=default,before_reset_fog=0xeeeeeeee,before_reset_cheat=128,locked=1,rule_value=1,later_fog=2,later_cheat=1,fast_source=1,later_fast=0)

def main():
 rows=[]
 for pc in h.PRECISION:
  for c in cases():
   a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b
   rows.append(dict(precision=pc,input=c,output=a))
 # Fixed source-registration and producer timing bytes. These are static pins,
 # not a claim that complete process startup or the enclosing GWorld executes.
 ranges=[(0x507cef,0x37),(0x508890,0x33),(0x4902a2,0x14),(0x4f281c,0xf),(0x4fc7fb,0x63)]
 data=[]
 for a,n in ranges:
  raw=ORIGINAL.range(a,n);assert raw==h.V.range(a,n);data.append(dict(address=hex(a),bytes=raw.hex()))
 strings=[]
 for a,name in [(0xa2fce8,'FogOfWar'),(0xa2f50c,'CheatFog'),(0xa36964,'ZoneBuilderFastDetectionZones')]:
  raw=(name+'\0').encode('utf-16le');assert ORIGINAL.range(a,len(raw))==raw==h.V.range(a,len(raw))
  strings.append(dict(address=hex(a),bytes=raw.hex(),name=name))
 result=dict(schema=1,scope=__doc__,original_sha256=HASH(ORIGINAL.b),v14_sha256=HASH(h.V.b),cases=rows,
  native_executions=len(rows)*2,static_boundary_pins=data,static_string_pins=strings,instructions=[dict(address=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 p=HERE/'native-view-variables.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=1)+'\n')
 else:assert json.loads(p.read_text())==result
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=HASH(p.read_bytes()))))
if __name__=='__main__':main()
