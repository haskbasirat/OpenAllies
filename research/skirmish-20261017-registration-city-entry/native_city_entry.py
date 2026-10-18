"""Whole original515620 slot8 registration -> real564300/5640A0.

Both images run independently at all3 PCs. Laboratory objects are explicit
inputs.6F7810 is allocation transport; guard514DA0/type3 58CEA0/schedule50AC60
are named required cutoffs, never successful callbacks. Null zone accesses
are stopped by the page0 guard and excluded from completed instruction pins.
No actual map startup or arbitrary callback reentry is certified.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
sha=lambda b:hashlib.sha256(b).hexdigest()

def cases():
 base=dict(leaf=0x100,count=9,zone=True,zone_flags=0x100,city=True,target='other',dead=False,owner=True,nonmajor=0,value=0x3f800000,pending=True,accepted=False,candidate=False)
 out=[]
 def add(name,**kw):out.append(dict(base,name=name,**kw))
 add('positive');add('self',target='self');add('null-target',target='null')
 add('dead',dead=True);add('no-owner',owner=False);add('nonmajor',nonmajor=1)
 add('zero-value',value=0);add('negative-value',value=0xbf800000)
 add('zone-type-cleared',zone_flags=0);add('city-null',city=False)
 add('short-vector',count=8);add('null-zone',zone=False)
 add('leaf-no-city',leaf=0);add('marker-detection-null',leaf=0x104)
 add('accepted',accepted=True);add('candidate',candidate=True)
 add('schedule-required',pending=False)
 add('guard-before-city',leaf=0x120);add('type3-before-city',leaf=0x108)
 return out

def run(c,image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 leaf,actor,slots,zone,city,target,kingdom,source,performer,zone3=[m.alloc(n)for n in[0x10,0x200,0x24,0x30,0x24,0x200,0x300,0x600,0xe0,0x30]]
 m.w(leaf+8,actor);m.w(leaf+0xc,c['leaf'])
 m.w(actor,0xa273f0);m.w(actor+0x148,slots);m.w(actor+0x14c,c['count'])
 m.w(slots+32,zone if c['zone'] else 0);m.w(slots+12,zone3)
 m.w(zone+4,actor);m.w(zone+0x14,c['zone_flags']);m.w(zone3+4,actor);m.w(zone3+0x14,8)
 m.w(actor+0xd4,city if c['city'] else 0);m.w(actor+0x90,0x1234)
 m.w(city+4,actor);m.w(city+0xc,int(c['pending']))
 if c['target']=='self':target=actor
 if c['target']=='null':target=0
 if target and c['target']!='self':
  m.w(target,0xa273f0);m.w(target+4,source);m.w(target+8,int(c['dead']));m.w(target+0x14,0x12345678)
  m.w(target+0xe8,kingdom if c['owner'] else 0);m.w(target+0x74,performer);m.w(performer+0x60,c['value'])
 m.w(kingdom+0x190,c['nonmajor'])
 for offset,active in [(0x10,c['accepted']),(0x18,c['candidate'])]:
  if active:
   node=m.alloc(8);m.w(node,0x12345678);m.w(city+offset,node);m.w(city+offset+4,node)
 events=[];terminal=['returned'];allocated=[]
 reads={actor+0x14c:'zone_count',actor+0x148:'zone_vector',slots+32:'zone_slot8',zone+0x14:'zone_flags',zone+4:'zone_actor',actor+0xd4:'city_component',city+4:'city_owner',city+0x10:'accepted',city+0x18:'candidates',city+0xc:'pending'}
 if target:reads.update({target+8:'target_flags',target+0xe8:'target_owner',target+0x14:'target_reference'})
 def read(u,access,address,size,value,data):
  if address in reads:events.append(reads[address])
 def write(u,access,address,size,value,data):
  if address==city+0x18:events.append('store_head')
  if address==city+0x1c:events.append('store_tail')
 m.u.hook_add(UC_HOOK_MEM_READ,read);m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 m.observe(0x564300,lambda _:events.append('admit'))
 def alloc(_):
  assert m.arg(1)==8;events.append('allocate');p=m.alloc(8);allocated.append(p);m.ret(p,0)
 m.seam(0x6f7810,alloc)
 def cutoff(name):
  def stop(_):events.append(name);terminal[0]=name;m.u.emu_stop()
  return stop
 for address,name in [(0x514da0,'guard_required'),(0x58cea0,'type3_required'),(0x50ac60,'schedule_required')]:m.seam(address,cutoff(name))
 try:m.call(0x515620,leaf,[target])
 except AssertionError:
  if m.null_accesses:
   assert not c['zone'] or c['count']<=8
   terminal[0]='null_zone'
   for fault_pc,_,_ in m.null_accesses:m.visited.pop(fault_pc,None)
  elif terminal[0]=='returned':raise
 values=[];node=m.r(city+0x18);last=0
 while node:
  assert node!=last;values.append(m.r(node));last=node;node=m.r(node+4)
 assert m.r(city+0x1c)==last
 return dict(terminal=terminal[0],events=events,candidates=values,allocated=len(allocated)),m.visited

def main():
 p=argparse.ArgumentParser();p.add_argument('--create',action='store_true');args=p.parse_args()
 rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for c in cases():
   pair=[]
   for image in [original,modified]:
    output,visited=run(c,image,pc);pair.append(output)
    for address,raw in visited.items():
     assert raw==original.range(address,len(raw))==modified.range(address,len(raw)),hex(address)
     pins[address]=raw
   assert pair[0]==pair[1],(c,pc,pair)
   rows.append(dict(precision=pc,input=c,native=pair[0]))
 fixture=dict(scope=__doc__,original_sha256=sha(original.b),v14_sha256=sha(modified.b),rows=rows,pins=[dict(address=hex(a),bytes=b.hex())for a,b in sorted(pins.items())])
 path=HERE/'native-city-entry.json'
 if args.create:
  with path.open('x',encoding='utf-8')as f:json.dump(fixture,f,indent=2);f.write('\n')
 else:assert fixture==json.loads(path.read_text(encoding='utf-8'))
 print(json.dumps(dict(paired_rows=len(rows),separate_executions=len(rows)*2,pins=len(pins),fixture_sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
