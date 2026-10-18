"""58B0E0 + real4D69A0/516640 and borrowed58AE10 coordinator.
Tick state/relay collection/provides/zone solvers are explicit declared children.
Separate primary-state and provides probes execute real leaves. Every code-hook
stop is before its instruction or required-child entry; no memory-write stops.
"""
from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stop(Exception):pass

def run(c,image,pc,stop=None):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);events=[];counts={}
 d=m.alloc(0x600);a=m.alloc(0x200);s=m.alloc(0x40);k=m.alloc(0x300);w=m.alloc(0x300)
 cache=m.alloc(0x300);under=m.alloc(4);rate=m.alloc(4);arr=m.alloc(36);z=m.alloc(0x20);city=m.alloc(0x30)
 m.w(a,0xa273f0);m.w(a+4,d);m.w(a+0x8c,s);m.w(s+4,a);m.w(a+0xe8,k if c['owner']else 0)
 m.w(s,0xa400c0);m.w(s+8,0xa40098);m.w(s+0xc,int(c['pending']))
 m.w(s+0x10,c['radius']);m.u.mem_write(s+0x14,bytes([c['dirty']]));m.w(s+0x18,c['stock']);m.w(s+0x1c,c['relay_id'])
 m.w(a+8,c['flags']);m.w(a+0x100,c['state']);m.w(a+0x104,c['high']);m.w(a+0x20,c['xy'][0]);m.w(a+0x24,c['xy'][1])
 m.w(a+0x14c,9 if c['zone']else 0);m.w(a+0x148,arr if c['zone']else 0);m.w(arr,z if c['zone']else 0)
 m.w(z+8,c['active_radius']);m.w(z+4,a);m.u.mem_write(z+0x14,b'\x01\x00')
 m.u.mem_write(d+0x424,bytes([int(c['relay'])]));m.w(d+0x428,c['maximum']);m.w(k+0x190,c['nonmajor'])
 m.w(a+0xd4,city if c['city']else 0);m.u.mem_write(city+0x20,bytes([c['city_byte']]))
 m.w(0xb36534,w);m.w(w+0x28,m.alloc(0xb0));m.w(m.r(w+0x28)+0xa8,int(c['suspended']))
 m.w(0xb39ab0,cache);m.w(cache+0xfc,under);m.w(cache+0x100,rate);m.w(under,c['under']);m.w(rate,c['rate'])
 def event(label,payload=None):
  key=f'{label}:{counts.get(label,0)}';counts[label]=counts.get(label,0)+1
  if stop==key:raise Stop(key)
  events.append([key,payload])
 def queue():event('queue',[1,m.arg(0)]);m.w(s+0xc,1);m.ret(4)
 def init_tick():event('tick',m.arg(0));m.ret(4)
 def update():event('update');m.ret(0)
 def provides():event('provides');m.ret(0,int(c['provides']))
 def clear():event('clear');m.w(z+8,0xbf800000);m.ret(0)
 def move():
  event('move',[m.arg(0),m.arg(1),m.arg(2)]);m.w(z+8,m.arg(2));m.ret(12)
 def refresh():event('economy');m.ret(0)
 def search():event('search');raise Stop('required_search')
 def state():event('state',[m.arg(0),m.arg(1)&0xff]);raise Stop('required_state_effects')
 phase=c['phase'];labels={}
 if phase=='init':
  m.seam(0x50ac60,queue);m.seam(0x58ae10,init_tick)
  labels={0x58b0e3:'pending',0x58b0f7:'zone',0x4d69a0:'create'}
 elif phase=='tick':
  m.seam(0x4d7ce0,update);m.seam(0x58ace0,provides);m.seam(0x514ce0,clear);m.seam(0x515f40,move);m.seam(0x4cf0f0,refresh)
  #Native owner call is the first mandatory collector operation after1C=0.
  if c['relay']and not c['suspended']:m.seam(0x4d1e80,search)
  labels={0x58ae33:'relay',0x4f0460:'suspended',0x58ae56:'clear_relay',0x58af65:'relay',0x58af76:'out',
   0x58af87:'stock',0x58af97:'stock',0x58afa1:'stock',0x58afb4:'stock',0x58afdb:'stock',0x58aff7:'stock',0x58affd:'stock',
   0x58afd5:'maximum',0x58aff1:'rate',0x58af9e:'store',0x58afb1:'store',0x58affa:'store',0x58b007:'store',
   0x58b018:'active',0x58b074:'dirty',0x58b07b:'radius',0x58b087:'position',0x58b0bc:'clear_dirty'}
 elif phase=='primary':
  m.seam(0x4d64f0,state);labels={0x4d7ce8:'primary_out'}
 elif phase=='provides':
  labels={0x4d1e80:'owner',0x58ad01:'nonmajor',0x58ad11:'high',0x58ad21:'low',0x58ad35:'dead',
   0x58ad46:'under',0x432ef0:'siege',0x58ad60:'relay',0x4d0c90:'stockpile_supplied',0x58ad76:'range',0x58ad89:'city'}
 def before(u,at,size,_):
  if at in labels:event(labels[at])
 m.u.hook_add(UC_HOOK_CODE,before)
 stopped=None;ret=None
 try:
  ret=m.call({'init':0x58b0e0,'tick':0x58ae10,'primary':0x4d7ce0,'provides':0x58ace0,'after':0x4b2830}[phase],s if phase!='primary'else a,[c['seconds']]if phase=='tick'else [])&0xff
 except Stop as e:stopped=str(e)
 count=m.r(a+0x14c);head=m.r(m.r(a+0x148))if count else 0
 return dict(events=events,stock=m.r(s+0x18),relay_id=m.r(s+0x1c),dirty=m.u.mem_read(s+0x14,1)[0],
  zone_count=count,zone_radius=m.r(head+8)if head else None,zone_flags=int.from_bytes(m.u.mem_read(head+0x14,2),'little')if head else None,
  zone_state=m.u.mem_read(head+0x16,1)[0]if head else None,stopped=stopped,result=ret if phase in ('init','primary','provides')else None),m.windows()

def cases():
 base=dict(phase='init',pending=False,zone=False,relay=False,relay_id=0x123456,dirty=1,stock=0x3f800000,maximum=0x41200000,
  radius=0x41c80000,active_radius=0xbf800000,provides=True,suspended=True,state=0,high=0,flags=0,xy=[0x42060000,0xc14c0000],
  owner=True,nonmajor=0,under=0,rate=0x3f800000,city=False,city_byte=0,seconds=0)
 for pending in (False,True):
  for zone in(False,True):yield dict(base,pending=pending,zone=zone)
 for active in (0xbf800000,0,0x41200000):
  for provides in(False,True):
   for dirty in(0,1,255):yield dict(base,phase='tick',zone=True,active_radius=active,provides=provides,dirty=dirty)
 for state,stock,maximum,rate,seconds in [(0,0,0x41200000,0x3f800000,0),(0,0x3f800000,0x41200000,0x3f800000,0x3eaaaaab),
  (0,0x411fffff,0x41200000,0x40400000,0x3f000000),(0,0x41200000,0x41200000,0,0),
  (0x80000,0,0x41200000,0,0),(0x80000,0x80000000,0x41200000,0,0),(0x80000,0x3f800000,0x41200000,0,0),
  (0x80000,0x3f000000,0x41200000,0,0x3f800000),(0x80000,0x3f800000,0x41200000,0,0x3f800000)]:
  yield dict(base,phase='tick',zone=True,relay=True,state=state,stock=stock,maximum=maximum,rate=rate,seconds=seconds)
 yield dict(base,phase='tick',zone=True,relay=True,suspended=False)
 for state in(0,0x80000):yield dict(base,phase='primary',state=state)
 for change in [dict(),dict(owner=False),dict(nonmajor=1),dict(high=128),dict(state=0x200000),dict(flags=1),dict(state=0x100000),dict(state=0x100000,under=1),dict(radius=0x80000000),dict(radius=0xbf800000),dict(radius=0x7fc01234),dict(radius=0x7f800000),dict(city=True,city_byte=1),dict(city=True,city_byte=0),dict(relay=True,state=0x80000,stock=0),dict(relay=True,state=0x80000,stock=0x3f800000)]:
  yield dict(base,phase='provides',**change)
 yield dict(base,phase='after')

def main():
 rows=[];pins={}
 for c in cases():
  baseline,_=run(c,'original','Double53')
  for stop in [None]+[event[0]for event in baseline['events']]:
   for pc in PC:
    a,pa=run(c,'original',pc,stop);b,pb=run(c,'v14',pc,stop);assert a==b,(c,pc,stop,a,b)
    rows.append(dict(input=c,pc=pc,stop=stop,output=a));pins.update({v['address']:v for v in pa+pb})
 #Vtable/cell windows are separate static pins, not execution counts.
 L.IMAGE=ORIGINAL;m=L.Loader();static=[]
 for address,size in [(0xa400c0,0x18),(0xa40098,12),(0xa273f0+0x108,4),(0x6a71d0,3),(0x4b2830,1)]:
  a=bytes(m.u.mem_read(address,size));L.IMAGE=EXPANDED;n=L.Loader();b=bytes(n.u.mem_read(address,size));assert a==b
  static.append(dict(address=hex(address),bytes=a.hex(),sha256=hashlib.sha256(a).hexdigest()))
 out=dict(schema=1,scope=__doc__,cases=rows,blocks=[pins[k]for k in sorted(pins)],static=static)
 raw=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-supply-init.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
