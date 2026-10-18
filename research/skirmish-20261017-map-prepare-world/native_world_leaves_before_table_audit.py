"""Actual map-preparation dependencies, not full replacement execution.

Runs 4EFBE0 over explicit valid GTID tables (including non-Actors and high
generation bits), actual Data vtable+38 leaves, and 4FC4A0 empty drain. Positive
drain stops BEFORE 4FBA50. No successful seams. Factory jump entries, constructor
vtable stores and dispatch pointers are inspected/pinned separately, not claimed
as executed constructor or full factory coverage. Synthetic tables do not prove
the live-world producer; the Rust adapter borrows the retained actual registry.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P;V14=h.V
class Required(Exception):pass
TABLES={1:0xa13318,2:0xa05540,18:0xa133b0,19:0xa09b30,20:0xa13448,21:0xa0f6e0}

def run(case,image,pc):
 h.P=ORIGINAL if image=='original'else V14;h.VISITED={};m=h.Machine(pc)
 if case['kind']=='iterator':
  registry=m.alloc(0x60010);cursor=m.alloc(4);reverse={}
  m.w32(registry+0x6000c,case['highest']);m.w32(registry+0x60004,len(case['entries']))
  for entry in case['entries']:
   p=m.alloc(0x18);m.w32(p+0x14,entry['id']);reverse[p]=entry['id']
   m.w32(registry+0x20004+4*(entry['id']&0xffff),p)
   # Deliberately different Actor classification table. 4EFBE0 must never
   # consult this; it enumerates both kinds from +20004.
   m.w32(registry+4+4*(entry['id']&0xffff),p if entry['actor']else 0)
  m.w32(cursor,case['cursor']);rows=[]
  for _ in range(case['calls']):
   value=m.call(0x4efbe0,registry,[cursor]);rows.append(dict(id=reverse.get(value),cursor=m.r32(cursor)))
  out=dict(answers=rows,active_count=m.r32(registry+0x60004),highest=m.r32(registry+0x6000c))
 elif case['kind']=='virtual38':
  table=TABLES[case['native_kind']];p=m.alloc(0x600);m.w32(p,table)
  fn=m.r32(table+0x38);m.u.mem_write(p+4,b'\xcd'*0x5fc)
  out=dict(native_kind=case['native_kind'],table=hex(table),target=hex(fn),eax=m.call(fn,p))
 elif case['kind']=='drain':
  queue=m.alloc(24);m.w32(queue,0xdead1000);m.w32(queue+4,case['objects']);m.w32(queue+8,8)
  m.w32(queue+12,0xdead2000);m.w32(queue+16,case['actors']);m.w32(queue+20,8)
  def stop(mm):raise Required('4FBA50 actual replacement producer required')
  m.seams[0x4fba50]=stop;stopped=None
  before=bytes(m.u.mem_read(queue,24)).hex()
  try:m.call(0x4fc4a0,queue)
  except Required as e:stopped=str(e)
  out=dict(stopped=stopped,queue_unchanged=bytes(m.u.mem_read(queue,24)).hex()==before)
 else:raise AssertionError(case)
 return out,h.VISITED.copy()

def cases():
 tables=[[],[dict(id=1,actor=False)],[dict(id=0x80000002,actor=True)],
  [dict(id=0x12340001,actor=False),dict(id=0xffff0003,actor=True),dict(id=65535,actor=False)],
  [dict(id=0x80000007,actor=False),dict(id=0xfffd0080,actor=False),dict(id=0x9999fffe,actor=True)]]
 for entries in tables:
  high=max([e['id']&0xffff for e in entries],default=0)
  for cursor in [0,0x12340002,0xffffffff]:
   yield dict(kind='iterator',entries=entries,highest=high,cursor=cursor,calls=len(entries)+2)
 yield dict(kind='iterator',entries=[],highest=200,cursor=0,calls=2)
 for native_kind in TABLES:yield dict(kind='virtual38',native_kind=native_kind)
 for objects,actors in [(0,0),(1,0),(0,1)]:yield dict(kind='drain',objects=objects,actors=actors)

def inspected():
 # Each factory jump selects the constructor; these bytes are inspected only.
 # Constructor stores below identify the concrete vtables used by the getters.
 result=[]
 for kind,table in TABLES.items():
  jump=ORIGINAL.u32(0x4bd028+4*kind);raw=ORIGINAL.range(jump,55)
  assert raw==V14.range(jump,55)
  # Locate the actual vtable store inside the known constructor by exact
  # bytes; keep a narrow bounded search, independent of disassembler labels.
  windows={1:(0x49c760,0x160),2:(0x4624b0,0x170),18:(0x49cc20,0x60),
           19:(0x469980,0x60),20:(0x49cc80,0x90),21:(0x491ef0,0x37)}
  start,size=windows[kind];needle=b'\xc7\x06'+struct.pack('<I',table)
  region=ORIGINAL.range(start,size);hits=[start+i for i in range(len(region))if region.startswith(needle,i)]
  assert len(hits)==1,(kind,hits)
  addr=hits[0];assert ORIGINAL.range(addr,6)==V14.range(addr,6)
  pointer=ORIGINAL.range(table+0x38,4);assert pointer==V14.range(table+0x38,4)
  result.append(dict(native_kind=kind,factory_jump_slot=hex(0x4bd028+4*kind),factory_jump=hex(jump),
   factory_branch_bytes=raw.hex(),constructor_vtable_store=dict(va=hex(addr),bytes=ORIGINAL.range(addr,6).hex()),
   getter_slot=dict(va=hex(table+0x38),bytes=pointer.hex())))
 return result

def main():
 rows=[];pins={}
 for pc in h.PRECISION:
  for case in cases():
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   pins.update(pa);pins.update(pb);rows.append(dict(input=case,precision=pc,output=a))
 executed=[]
 for va,raw in sorted(pins.items()):
  if va==0x4fba50:continue
  assert ORIGINAL.range(va,len(bytes.fromhex(raw)))==V14.range(va,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
  executed.append(dict(va=hex(va),bytes=raw))
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,cases=rows,separate_machine_runs=2*len(rows),
  executed_instructions=executed,successful_seams=[],required_boundary=dict(va='0x4fba50',bytes=ORIGINAL.range(0x4fba50,6).hex()),
  inspected_data_dispatch=inspected())
 path=HERE/'native-world-leaves.json'
 if '--write'in sys.argv:
  with path.open('x',encoding='utf-8')as f:f.write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),machine_runs=2*len(rows),executed_pins=len(executed),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
