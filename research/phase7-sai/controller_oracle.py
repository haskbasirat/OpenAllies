"""Original controller constructor and complete major Kingdom registration."""
from pathlib import Path
import json,sys
from state_oracle import Machine,ROOT,OUT,H,original,modified
from unicorn.x86_const import *
def base_goal(m,p,controller):
 return {'controller':controller,'category8':m.read(p+8),'words0c_14':[m.read(p+x)for x in [12,16,20]],'scalar18':m.read(p+24),'words1c_20':[m.read(p+x)for x in [28,32]],'word24':m.read(p+36),'word28':m.read(p+40),'byte2c':bool(m.u.mem_read(p+44,1)[0]),'scalars30_38':[m.read(p+x)for x in [48,52,56]]}
def list_values(m,n):
 out=[]
 while n:out.append(m.read(n));n=m.read(n+4)
 return out
def controller(m,p,object,player):
 groups=[]
 for i in range(23):
  group=m.read(m.read(p+8)+i*4);groups.append([list_values(m,m.read(group+8*j))for j in range(3)])
 goals=[]
 for g in list_values(m,m.read(p+12)):
  assert m.read(g)==0xa9e560
  goals.append({'base':base_goal(m,g,object),'target_id':m.read(g+60),'words40_44':[m.read(g+64),m.read(g+68)]})
 auxiliary=m.read(p+36)
 return {'object':object,'player':player,'initialized':bool(m.u.mem_read(p,1)[0]),'groups':groups,'pending_kingdom_goals':goals,'words14_1c':[m.read(p+x)for x in [20,24,28]],'byte20':bool(m.u.mem_read(p+32,1)[0]),'hash_buckets':[list_values(m,m.read(m.read(p+40)+4*i))for i in range(m.read(p+44))],'hash_entry_count':m.read(p+48),'hash_flag34':bool(m.u.mem_read(p+52,1)[0]),'auxiliary':{'controller':object,'pending':list_values(m,m.read(auxiliary+4)),'hash_buckets':[list_values(m,m.read(m.read(auxiliary+12)+4*i))for i in range(m.read(auxiliary+16))],'hash_entry_count':m.read(auxiliary+20),'hash_flag18':bool(m.u.mem_read(auxiliary+24,1)[0])}}
def main():
 source=json.loads((OUT/'native-state.json').read_text())['fixtures'][0]['input'];rows=[];visited={}
 for variant in range(6):
  m=Machine(bytes.fromhex(source['input_hex']),source['layout'],[0x200]*64);m.call(0x7aa9b0,m.graph,[m.stream-0x3c,m.grid]);sai=m.alloc(0x100)
  sv=m.alloc(0x200);val=m.alloc(4);m.write(0xb39ab0,sv);m.write(sv+0x108,val)
  kingdoms=[];pointers=[]
  for i,category in enumerate([0,0,1,0]):
   k={'object':i+1,'native_id':0x12340000+i,'major_index':i,'category':category};kingdoms.append(k);p=m.alloc(0x300);pointers.append(p);m.write(p+0x14,k['native_id']);m.write(p+0x18c,i);m.write(p+0x190,category)
  slots=[None,{'kingdom':1,'controller':None},{'kingdom':2,'controller':{'object':100,'player':200}},{'kingdom':None,'controller':{'object':101,'player':201}}];sp=m.alloc(16);m.write(sai+0x60,sp);m.write(sai+0x64,4);controllers=[]
  for i,slot in enumerate(slots):
   if slot is None:continue
   player=m.alloc(0x100);m.write(sp+i*4,player);m.write(player+8,pointers[slot['kingdom']-1]if slot['kingdom']else 0)
   if slot['controller']:
    p=m.alloc(0x38);m.call(0x7a5750,p,[player]);m.write(player+12,p);controllers.append((i,p))
  sequence=[0,1,2,3,0,None][:variant+1];states=[]
  for ki in sequence:
   if ki is None:
    for _,p in controllers:m.call(0x7a48f0,p,[0])
   else:m.call(0x79e170,sai,[pointers[ki]])
   states.append([controller(m,p,slots[i]['controller']['object'],slots[i]['controller']['player'])for i,p in controllers])
  rows.append({'input':{'kingdoms':kingdoms,'slots':slots,'sequence':sequence},'native':states});visited.update(m.executed)
 pins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 (OUT/'native-controller.json').write_text(json.dumps({'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),'fixtures':rows,'native_basic_block_pins':pins,'scope':'Whole7A5750/7CCDC0,79E170 including7BC130 and7A48F0→7BE920→7A66E0→7A4580. No AIPlayer7B4830 or initialized goal dispatch claim. Only allocation/stream/version seams.'},indent=2)+'\n');print({'passed':True,'sequences':len(rows)})
if __name__=='__main__':main()
