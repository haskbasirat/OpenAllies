"""Original5687E0/568070 with actual per-node OR instructions.
Queue50AC60 and Zone4D69A0/515940/515F40 are declared required children.
Stops occur before native source reads/stores or child entry, preserving real
completed mask stores. Both PE images execute at all three arithmetic PCs.
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
 definition=m.alloc(0x600);actor=m.alloc(0x200);component=m.alloc(0x100);world=m.alloc(0x300)
 m.w(actor+4,definition);m.w(component+4,actor);m.w(component+0x38,c['mask']);m.w(component+0xc,int(c['pending']))
 m.w(world+0x2c,c['version']);m.w(0xb36534,world);m.w(definition+0x364,c['guard']);m.w(definition+0x35c,0x123400 if c['inventory']else 0)
 m.w(actor+8,c['flags']);m.w(actor+0x20,c['xy'][0]);m.w(actor+0x24,c['xy'][1]);m.w(actor+0x14c,0);m.w(actor+0x148,0)
 for offset,value_offset,next_offset,values in [(0x34c,8,12,c['organization']),(0x354,4,28,c['groups'])]:
  head=0
  for v in reversed(values):
   node=m.alloc(0x20);m.w(node+value_offset,v);m.w(node+next_offset,head);head=node
  m.w(definition+offset,head)
 def event(label,payload=None):
  key=f'{label}:{counts.get(label,0)}';counts[label]=counts.get(label,0)+1
  if stop==key:raise Stop(key)
  events.append([key,payload]);return key
 def queue():event('queue',[1,0]);m.ret(4)
 def create():
  event('create',5)
  if c['zone']:
   arr=m.alloc(9*4);z=m.alloc(0x80);m.w(arr+20,z);m.w(actor+0x148,arr);m.w(actor+0x14c,9)
  m.ret(4,int(c['create_result']))
 def flag():
  event('zone_flag',[m.arg(0),m.arg(1)]);m.ret(8)
 def move():
  event('move',[m.arg(0),m.arg(1),m.arg(2)]);m.ret(12)
 m.seam(0x50ac60,queue);m.seam(0x4d69a0,create);m.seam(0x515f40,move)
 #A null515940 receiver executes its real first dereference, never a
 #successful child seam. The nonnull zone mutation remains declared service.
 if c['zone']:m.seam(0x515940,flag)
 labels={0x5687e6:'head34c',0x5687f1:'value34c',0x5687f4:'or34c',0x5687f7:'next34c',0x568801:'head354',0x568810:'value354',0x568813:'or354',0x568816:'next354',0x568822:'version',0x56882b:'pending',0x568835:'guard',0x568848:'inventory',0x568080:'guard',0x568093:'flags',0x5680a0:'mask',0x5680a6:'zone_for_flag',0x5680cf:'guard',0x5680dd:'position',0x5680f4:'zone_for_move'}
 def before(u,at,size,_):
  if at in labels:event(labels[at])
 m.u.hook_add(UC_HOOK_CODE,before)
 stopped=None
 try:m.call(0x5687e0 if c['phase']=='init'else 0x568070,component)
 except Stop as e:stopped=str(e)
 except RuntimeError as e:
  assert c['phase']=='after'and not c['zone']and c['mask']&32 and 'native null-page access'in str(e),str(e)
  stopped='null_zone_flag'
 return {'events':events,'mask':m.r(component+0x38),'stopped':stopped},m.windows()
def cases():
 base=dict(phase='init',mask=0x100,organization=[],groups=[],version=27,pending=False,guard=0,inventory=False,flags=0,xy=[0x42060000,0xc14c0000],zone=True,create_result=True)
 for values in [([],[]),([2,64,8],[16,128]),([0,2,2],[0,0xffffffff])]:
  for version,pending,guard,inventory in [(27,False,0,False),(21,True,0x3f800000,True),(21,False,0,False),(21,False,0x80000000,True),(21,False,0x3f800000,False),(21,False,0x7fc00001,False),(21,False,0xbf800000,False)]:
   yield dict(base,organization=values[0],groups=values[1],version=version,pending=pending,guard=guard,inventory=inventory)
 for guard,flags,mask,zone,create_result in [(0,0,0,False,False),(0x80000000,0,32,False,True),(0x3f800000,1,32,False,True),(0x3f800000,0,0,True,True),(0xbf800000,0,32,True,True),(0x7fc00001,0,32,True,False),(0x3f800000,0,0,False,False),(0x3f800000,0,32,False,False)]:
  yield dict(base,phase='after',guard=guard,flags=flags,mask=mask,zone=zone,create_result=create_result)
def main():
 rows=[];pins={}
 for c in cases():
  baseline,_=run(c,'original','Double53')
  stops=[None]+[event[0]for event in baseline['events']]
  for stop in stops:
   for pc in PC:
    a,pa=run(c,'original',pc,stop);b,pb=run(c,'v14',pc,stop);assert a==b
    rows.append(dict(input=c,pc=pc,stop=stop,output=a));pins.update({v['address']:v for v in pa+pb})
 out=dict(schema=1,scope=__doc__,cases=rows,blocks=[pins[k]for k in sorted(pins)])
 raw=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-denizen-init.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
