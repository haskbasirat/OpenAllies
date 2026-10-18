"""Original GActor seen-table creation and false-argument activity predicate."""
import json,sys
from state_oracle import Machine,ROOT,OUT,H,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
def main():
 source=json.loads((ROOT/'research/phase7-navigation/synthetic-topology.json').read_text())['fixtures'][0]
 m=Machine(bytes.fromhex(source['input_hex']),source['layout'],[0x200]*64);actor=m.alloc(0x200);vt=m.alloc(0x200);m.write(actor,vt);m.write(vt+0x8c,0x4ea3d0);world=m.alloc(0x200);m.write(0xb36534,world);tables=[]
 for old in [-1,0,1,3]:
  for major in [0,1,4,20]:
   arr=m.alloc(12)
   for i,v in enumerate([17,0,23]):m.write(arr+4*i,v)
   m.write(actor+0x58,old);m.write(actor+0x5c,arr);m.write(world+0x158,major);m.call(0x4ea3e0,actor)
   count=m.read(actor+0x58);array=m.read(actor+0x5c);n=3 if old>0 else major
   tables.append({'input':{'count58':old,'entries':[17,None,23]},'major_count':major,'native':{'count58':count,'entries':[m.read(array+4*i)or None for i in range(n)]}})
 activities=[];component=m.alloc(0x40);contained=m.alloc(0x40);contained_vt=m.alloc(0x40);m.write(component+16,contained);m.write(contained,contained_vt)
 def child_hook(u,pc,size,_):m.ret(4,m.child_result)
 m.u.hook_add(UC_HOOK_CODE,child_hook,begin=0x2e00100,end=0x2e00100)
 for flags in [0,1,4,5]:
  for child in [None,False,True]:
   for mask in range(16):
    c={'flags8':flags,'contained_object_result':child,'component6c':bool(mask&1),'componentac':bool(mask&2),'componenta0':bool(mask&4),'component88':bool(mask&8)};m.write(actor+8,flags);m.write(actor+0x80,0 if child is None else component);m.write(contained_vt+32,0x2e00100);m.child_result=int(bool(child))
    for off,key in [(0x6c,'component6c'),(0xac,'componentac'),(0xa0,'componenta0'),(0x88,'component88')]:m.write(actor+off,int(c[key]))
    m.call(0x4cf120,actor,[0]);activities.append({'input':c,'native':bool(m.u.reg_read(UC_X86_REG_EAX)&255)})
 pins=[]
 for a,n in sorted(m.executed.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 (OUT/'native-actor-leaves.json').write_text(json.dumps({'schema':1,'passed':True,'producer_sha256':H(__import__('pathlib').Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),'seen_tables':tables,'activity':activities,'native_basic_block_pins':pins,'scope':'Actual4EA3E0→GActor4EA3D0 seen-table allocation and4CF120(false) with explicit contained-object virtual boolean fixture leaf. No actor classification transition aggregates or visibility population claim.'},indent=2)+'\n');print({'passed':True,'seen_tables':len(tables),'activity':len(activities)})
if __name__=='__main__':main()

