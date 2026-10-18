"""Actual568750 with actual45D730(null) health reads.50AC60 is the explicit
queue service boundary; production delegates it to the existing real queue
kernel. Read/queue cutoffs observe committed prefixes, not native exceptions.
The typed Inventory list is stable during the call, as in the live adapter.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
ORIGINAL,V14=h.P,h.V
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def run(pc,c):
 m=h.Machine(b'',pc);component=m.alloc(0x94,0xa5);actor=m.alloc(0x180,0xa5);data=m.alloc(0x600,0xa5);params=m.alloc(0x60,0xa5)
 m.w(component+4,actor);m.w(actor+4,data);m.w(actor+0x100,c['state']);m.w(params+0x38,c['mode'])
 m.w(data+0x364,c['guard']);m.w(data+0x35c,0x23456780 if c['head']else 0);m.w(data+0x3a4,c['delay']);m.w(component+0x34,c['old_time'])
 slots=[];targets={};head=0
 for i,health in reversed(list(enumerate(c['health']))):
  target=m.alloc(0x300,0xa5);m.w(target+0x284,health);targets[target+0x284]=i
  slot=m.alloc(0x14,0xa5);m.w(slot+4,target);m.w(slot+8,c['old_health'][i]);m.w(slot+0x10,head);head=slot;slots.insert(0,slot)
 m.w(component+0x18,head);events=[];outcome='ok'
 def event(*values):
  nonlocal outcome
  events.append(list(values))
  if len(events)==c['fail']:
   outcome='required-'+values[0];m.outcome={'kind':outcome};raise h.Throw()
 def reads(u,access,a,size,value,ctx):
  if size!=4:return
  if a==actor+0x100:event('construction',bool(c['state']&0x200000))
  elif a==component+0x18:event('count',len(slots))
  elif a in targets:event('health',targets[a],c['health'][targets[a]])
  elif a==data+0x364:event('guard',c['guard'])
  elif a==data+0x35c:event('head',c['head'])
  elif a==data+0x3a4:event('delay',c['delay'])
 def writes(u,access,a,size,value,ctx):
  if size!=4:return
  if a==component+0x34:events.append(['store-time',value])
  elif a in[slot+8 for slot in slots]:events.append(['store-health',[slot+8 for slot in slots].index(a),value])
 m.u.hook_add(UC_HOOK_MEM_READ,reads);m.u.hook_add(UC_HOOK_MEM_WRITE,writes)
 def schedule(m):
  assert m.u.reg_read(h.UC_X86_REG_ECX)==component+8 and m.arg(0)==0;event('schedule');m.ret(4)
 m.seam(0x50ac60,schedule);m.call(0x568750,component,[0 if c['null_params']else params])
 assert not m.null_accesses,m.null_accesses
 assert not m.calls,m.calls
 return dict(outcome=outcome,events=events,health=[m.r(slot+8)for slot in slots],time=m.r(component+0x34))

def main():
 base=dict(state=0x08000000,mode=0,null_params=False,guard=0,head=False,delay=0,old_time=0x3f123456,health=[],old_health=[],fail=None)
 cases=[]
 for state,mode,null in[(0,0,False),(0x200000,0,False),(0,4,False),(0,0,True)]:
  for guard,head in[(0,False),(0,True),(h.f32(3),False),(h.f32(-1),True),(0x80000000,False),(0x7fc12345,False)]:
   for health in[[],[h.f32(2.5),0x80000000,1]]:
    cases.append(dict(base,state=state,mode=mode,null_params=null,guard=guard,head=head,delay=h.f32(2.75),health=health,old_health=[0x3e800000]*len(health)))
 #Actual reached ger_corps empty-list/state gate shape, not a source default.
 cases.append(base)
 sample=dict(base,health=[h.f32(1),h.f32(2)],old_health=[0x3f123456]*2,head=True,delay=0x80000000)
 events=run('Double53',sample)['events']
 cutoffs=[i for i,e in enumerate(events,1)if not e[0].startswith('store-')]
 cases.extend(dict(sample,fail=i)for i in cutoffs)
 for delay in[0,0x80000000,1,0x007fffff,h.f32(-7.5),0x7f800000,0xff800000,0x7fc12345]:cases.append(dict(base,delay=delay))
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.CW,cases):
  outputs=[]
  for image in(ORIGINAL,V14):h.P=image;outputs.append(run(pc,c))
  assert outputs[0]==outputs[1];rows.append(dict(precision=pc,input=c,output=outputs[0]))
 h.P=ORIGINAL;pins={hex(a):v for a,v in sorted(h.VISITED.items())if a!=0x50ac60}
 #Source3A4 constructor and optional Denizen/Runway loader producer windows.
 windows={hex(a):ORIGINAL.range(a,size).hex()for a,size in[(0x4629f2,12),(0x4674c7,24),(0x46759c,18)]}
 for a,b in windows.items():assert V14.range(int(a,16),len(bytes.fromhex(b)))==bytes.fromhex(b)
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,pins=pins,source_windows=windows,seams=['50AC60 periodic queue service; actual production queue/RNG owner required'],domain='valid stable typed Inventory links, finite target health, source scalar copies including quietNaN comparison/sign. No signallingNaN CPU/exception/unwind equivalence')
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-denizen-init-create.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),cutoffs=len(cutoffs),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
