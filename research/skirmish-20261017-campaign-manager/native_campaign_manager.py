"""Fresh static Application Campaign list +13C/+140/+144, real CRT9C79C0
and40CAB0, real GameAppState651810,4F1310/40CEB0 and411790 empty lookup.
Only atexit registration is a successful seam. Positive creation stops at the
actual6F7810 allocator entry BEFORE its body and before list append/resource
lookup; no positive Campaign owner/producer is invented. Application+FC link
is an explicit same-constructed-state fixture store, not full40CB40 startup.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
class Stop(Exception):pass

def zero_tail(image,word):
 rva=word-image.base
 for virtual,base,raw,offset in image.sections:
  if base<=rva<base+max(virtual,raw):
   assert raw<=rva-base and rva-base+4<=virtual
   return dict(va=hex(word),section_rva=hex(base),virtual_size=virtual,raw_size=raw,offset_in_section=rva-base)
 raise AssertionError('word not in image')

def run(case,image,pc):
 h.P=ORIGINAL if image=='original'else V14
 h.VISITED={};m=h.Machine(pc);seams=set();stopped=None;events=[]
 def at_exit(mm):seams.add(0x96cab0);mm.ret(0,0)
 def allocation_stop(mm):raise Stop('6F7810 Campaign allocation required')
 m.seams[0x96cab0]=at_exit;m.seams[0x6f7810]=allocation_stop
 app=0xb39690;words=[app+0x13c,app+0x140,app+0x144]
 assert[m.r32(p)for p in words]==[0,0,0]
 m.call(0x9c79c0);assert[m.r32(p)for p in words]==[0,0,0]
 state=m.alloc(0x14);m.u.mem_write(state,b'\xcd'*0x14);m.call(0x651810,state)
 assert m.r32(state+0xc)==1
 #Actual40CD68 store is independently byte pinned. Its source allocation is
 #the same real651810 result; other Application state slots are outside scope.
 m.w32(app+0xfc,state);m.w32(state+0xc,case['mode'])
 m.w32(0xb39d04,case['settings_pointer'])
 requested=0xdead0000
 current=None;looked_up=None
 try:
  if case['kind']=='capture_pair':
   current=m.call(0x4f1310);events.append(['current',current])
   assert current==0
   looked_up=m.call(0x411790,app,[requested,0]);events.append(['lookup',looked_up])
   assert looked_up==0
   for _ in range(2):assert m.call(0x411790,app,[requested,0])==0
  else:looked_up=m.call(0x411790,app,[requested,1])
 except Stop as e:stopped=str(e)
 out=dict(mode=m.r32(state+0xc),current=current,lookup=looked_up,stopped=stopped,
  table_words=[m.r32(p)for p in words],events=events,actual_game_state_identity=m.r32(app+0xfc)==state)
 return out,h.VISITED.copy(),seams

def cases():
 for mode in [1,2,4,0xffffffff,0x80000000,0]:
  for settings in [0,0xdead1000]:yield dict(mode=mode,settings_pointer=settings,kind='capture_pair')
 for mode in [1,0]:yield dict(mode=mode,settings_pointer=0xdead1000,kind='direct_create')

ORIGINAL=h.P;V14=h.V

def main():
 rows=[];pins={};seams=set();boundaries={0x6f7810}
 zeros=[]
 for word in [0xb397cc,0xb397d0,0xb397d4]:
  a=zero_tail(ORIGINAL,word);assert a==zero_tail(V14,word);zeros.append(a)
 for pc in h.PRECISION:
  for case in cases():
   a,pa,sa=run(case,'original',pc);b,pb,sb=run(case,'v14',pc);assert a==b
   pins.update(pa);pins.update(pb);seams|=sa|sb;rows.append(dict(input=case,precision=pc,output=a))
 executed=[]
 for va,encoded in sorted(pins.items()):
  raw=bytes.fromhex(encoded)
  if va in seams|boundaries:continue
  assert ORIGINAL.range(va,len(raw))==V14.range(va,len(raw))==raw
  executed.append(dict(va=hex(va),bytes=raw.hex()))
 link=ORIGINAL.range(0x40cd68,6);assert link==V14.range(0x40cd68,6)
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  cases=rows,separate_machine_runs=2*len(rows),executed_instructions=executed,
  pe_loader_zero_words=zeros,inspected_application_link=dict(va='0x40cd68',bytes=link.hex()),
  successful_transport_seams=[hex(v)for v in sorted(seams)],required_boundary=dict(va='0x6f7810',bytes=ORIGINAL.range(0x6f7810,6).hex()))
 p=HERE/'native-campaign-manager.json'
 if '--write'in sys.argv:
  with p.open('x',encoding='utf-8')as f:f.write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),machine_runs=2*len(rows),pins=len(executed),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
