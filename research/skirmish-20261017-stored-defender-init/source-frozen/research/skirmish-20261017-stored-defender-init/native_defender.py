"""Real566DB0+529C30+45D6C0/45D4C0+4D69A0+516640+515940.
Only515F40 zone solver and57E430 Performer are explicit child transports;
heap/vector transports are inherited from the established Loader. Stops occur
before the declared instruction/child. SignallingNaNs are excluded because
Unicorn does not quiet FLD32 as actual x87 does. No full mission claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PCS={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stop(Exception):pass
def run(c,image,pc,stop):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PCS[pc]);events=[];counts={}
 a=m.alloc(0x200);d=m.alloc(0x600);v=m.alloc(0x40);arr=m.alloc(36);z=m.alloc(24)
 m.w(a,0xa273f0);m.w(a+4,d);m.w(v,0xa3bbd4);m.w(v+4,a);m.w(a+0x78,v)
 m.w(a+8,0);m.w(a+0x20,c['xy'][0]);m.w(a+0x24,c['xy'][1]);m.w(a+0xbc,m.alloc(4)if c['aircraft']else 0)
 m.w(a+0x74,m.alloc(0x70)if c['performer']else 0)
 m.w(d+0x174,c['mask']);m.w(d+0x218,c['range218']);m.w(d+0x2fc,c['range2fc']);m.w(v+0x2c,c['old_range'])
 m.w(a+0x14c,9 if c['zone']else 0);m.w(a+0x148,arr if c['zone']else 0);m.w(arr+16,z if c['zone']else 0)
 m.w(z+4,a);m.w(z+8,0xbf800000);m.u.mem_write(z+0x14,b'\x10\x00\x01')
 def event(label):
  key=f'{label}:{counts.get(label,0)}';counts[label]=counts.get(label,0)+1
  if stop==key:raise Stop(key)
  events.append(key)
 def move():event('move');m.ret(12)
 def performer():event('refresh');m.ret(0)
 m.seam(0x515f40,move);m.seam(0x57e430,performer)
 labels={0x566db3:'zone',0x566dd9:'mask',0x45d6c0:'range',0x45d4c0:'range',0x566e08:'store',0x4d69a0:'create',0x566e19:'aircraft',0x566e23:'zone',0x515940:'flag',0x566e42:'position',0x566e7d:'performer'}
 def before(u,at,size,_):
  if at in labels:event(labels[at])
 m.u.hook_add(UC_HOOK_CODE,before)
 stopped=None;result=None
 try:result=m.call(0x566db0,v,[])
 except Stop as e:stopped=str(e)
 count=m.r(a+0x14c);zone=m.r(m.r(a+0x148)+16)if count>4 else 0
 return dict(events=events,range=m.r(v+0x2c),count=count,zone=bool(zone),zone_state=m.u.mem_read(zone+0x16,1)[0]if zone else None,zone_flags=int.from_bytes(m.u.mem_read(zone+0x14,2),'little')if zone else None,stopped=stopped,result=result),m.windows()
def cases():
 base=dict(zone=False,aircraft=False,performer=False,mask=1<<6,range218=0x41200000,range2fc=0x41f00000,old_range=0xa5a5a5a5,xy=[0x42060000,0xc14c0000])
 for zone in(False,True):
  for aircraft in(False,True):
   for performer in(False,True):
    for org in(False,True):yield dict(base,zone=zone,aircraft=aircraft,performer=performer,mask=(1<<6)|(128 if org else 0))
 for bits in(0,0x80000000,1,0x007fffff,0x00800000,0x3eaaaaab,0xbf800000,0x7f7fffff,0x7f800000,0xff800000):
  yield dict(base,range218=bits,performer=True)
  yield dict(base,mask=0xc0,range2fc=bits,aircraft=True)
def main():
 rows=[];pins={}
 for c in cases():
  baseline,_=run(c,'original','Double53',None)
  for stop in [None]+baseline['events']:
   for pc in PCS:
    a,pa=run(c,'original',pc,stop);b,pb=run(c,'v14',pc,stop);assert a==b,(c,pc,stop,a,b)
    rows.append(dict(input=c,pc=pc,stop=stop,output=a));pins.update({p['address']:p for p in pa+pb})
 L.IMAGE=ORIGINAL;m=L.Loader();L.IMAGE=EXPANDED;n=L.Loader();static=[]
 for at,size in[(0xa3bbd4,0x18),(0x4b2830,1),(0x529c30,7),(0x45d6c0,16),(0x45d4c0,16)]:
  a=bytes(m.u.mem_read(at,size));b=bytes(n.u.mem_read(at,size));assert a==b
  static.append(dict(address=hex(at),bytes=a.hex(),sha256=hashlib.sha256(a).hexdigest()))
 out=dict(schema=1,scope=__doc__,cases=rows,blocks=[pins[k]for k in sorted(pins)],static=static)
 raw=(json.dumps(out,indent=1)+'\n').encode();path=HERE/'native-defender.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
