from pathlib import Path
import sys,json,struct,hashlib
HERE=Path('research/skirmish-20261017-sai-world-ready').resolve();ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-sai'));import ai_oracle as base
import topology_oracle as topo
from controller_oracle import list_values
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP
H=lambda b:hashlib.sha256(b).hexdigest();O=base.original;V=base.modified
LEAVES={0x6f7810,0x6f7430}

def main():
 fixture=json.loads((ROOT/'research/phase7-sai/native-ai.json').read_text())['fixtures'][2]['input'];rows=[];pins={};leaves={}
 for pc in ['Single24','Double53','Extended64']:
  for flags,low,high,owner_index,twice,members in [(0,0,0,0,False,[]),(1<<29,0,0,0,False,[]),(0,1<<25,0,2,True,[]),
    (0,1<<26,0,5,True,[]),(0,0,4,2,True,[]),(0,0,16,0,True,[]),(0,0,1,1,False,[]),(0,1,0,1,True,[]),
    (0,0,0,2,False,[False]),(0,0,0,2,True,[True]),(0,0,0,0,True,[False,True,None]),(0,0,0,1,True,[True,False])]:
   water=bool(flags&(1<<29)or low&((1<<25)|(1<<26))or high&(4|16)or any(members))
   c=dict(precision=pc,flags=flags,low=low,high=high,members=members,water=water,owner_index=owner_index,twice=twice,actor_id=0x30007)
   results=[]
   for image in ['original','v14']:
    prior=topo.base.P;topo.base.P=O if image=='original'else V
    try:m=base.Machine(bytes.fromhex(fixture['input_hex']),fixture['layout'],[0x200]*64,pc)
    finally:topo.base.P=prior
    m.setup(fixture);region=None;point=None
    for y in range(fixture['layout']['height']):
     for x in range(fixture['layout']['width']):
      rectangle=m.read(m.read(m.grid+0x28)+(x+y*fixture['layout']['width'])*0x30+0x10)
      if rectangle:
       region_id=m.read(rectangle+4);region=m.read(m.read(m.graph+0x14)+4*region_id);scale=1<<fixture['layout']['coordinate_shift'];point=[struct.unpack('<I',struct.pack('<f',(n+.5)*scale))[0]for n in [x,y]];break
     if region:break
    assert region and point
    m.u.mem_write(m.graph+5,b'\1')
    manager=m.alloc(0xb0);actor=m.alloc(0x300);data=m.alloc(0x500);category=m.alloc(0x20);kingdom=m.alloc(0x300)
    m.write(actor,0xa273f0);m.write(actor+4,data);m.write(actor+8,(1<<23)|flags);m.write(actor+0x100,low);m.write(actor+0x104,high);m.write(actor+0x14,c['actor_id'])
    m.write(actor+0x20,point[0]);m.write(actor+0x24,point[1]);m.write(actor+0x84,1);m.write(actor+0xe8,kingdom);m.write(actor+0x140,5)
    m.write(data+0x18,category);m.write(category+0xc,2);m.write(kingdom+0x188,owner_index)
    if members:
     organization=m.alloc(0x40);array=m.alloc(4*len(members));m.write(actor+0x7c,organization);m.write(organization+0x28,array);m.write(organization+0x2c,len(members))
     for i,member in enumerate(members):
      if member is not None:
       p=m.alloc(0x300);raw=0x30008+i;m.write(p+0x14,raw);m.write(p+8,(1<<29)if member else 0);m.write(array+4*i,p)
       m.u.mem_write(m.registry+4+2*(raw&65535),(raw>>16).to_bytes(2,'little'));m.write(m.registry+0x20004+4*(raw&65535),p)
    vp={};vl={}
    def code(u,ip,size,user):
     if O.base<=ip<O.base+O.image_size:
      raw=bytes(u.mem_read(ip,size));assert raw==O.range(ip,size)==V.range(ip,size)
      (vl if ip in LEAVES else vp)[ip]=raw.hex()
    def guard(u,access,address,size,value,user):
     if address<0x1000:assert bytes(u.mem_read(u.reg_read(UC_X86_REG_EIP),1))==b'\x64'
    m.u.hook_add(UC_HOOK_CODE,code);m.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,guard)
    for _ in range(2 if twice else 1):m.call(0x7a0390,manager,[actor,0,0])
    def lists(ptr,count):return [list_values(m,m.read(m.read(ptr)+4*i)and m.read(m.read(m.read(ptr)+4*i)))for i in range(m.read(count))]
    out=dict(category=m.read(actor+0x140),sai_actor=m.read(actor+0x154),land=lists(region+0x28,region+0x2c),water=lists(region+0x34,region+0x38),manager90=lists(manager+0x90,manager+0x94),region_id=m.read(region))
    assert len(out['manager90'])==owner_index+1 and out['manager90'][owner_index]==[c['actor_id']]*(2 if twice else 1)
    assert (out['water']if water else out['land']) and (out['water']if water else out['land'])[owner_index]==out['manager90'][owner_index],(c,out,point,hex(m.read(0xb3652c)),hex(region))
    results.append(out);pins.update(vp);leaves.update(vl)
   assert results[0]==results[1];rows.append(dict(input=c,output=results[0]))
 doc=dict(scope='Full7A0390 category0 through real4CF010 region lookup,79F390/4D4E70/4D12C0,7BC330/470 and manager90 insertion; actual region constructor from full topology load; zero actual manager AI slots and region-player count. Actor154 is constructor-null. No positive controller/SAIActor body claimed.',producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(O.b),v14_sha256=H(V.b),rows=rows,machines=2*len(rows),executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],intercepted_entries=[dict(va=hex(a),bytes=b)for a,b in sorted(leaves.items())],dependencies=[dict(path=p,sha256=H((ROOT/p).read_bytes()))for p in ['research/phase7-sai/ai_oracle.py','research/phase7-sai/state_oracle.py','research/phase7-navigation/topology_oracle.py','research/phase7-sai/native-ai.json']])
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-actor-map-lists.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),intercepted_entries=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
