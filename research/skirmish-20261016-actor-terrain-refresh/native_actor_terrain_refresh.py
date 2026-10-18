"""Actual4D4130 plus4D10A0/570560/4D12C0/4EA200/54ECE0, both images/allPC.
Current/prior terrain identities and list nodes are explicit fixture owners.
Required Organization579EB0/Weapon581810/Property5035E0 stop BEFORE bodies;
null-terrain diagnostics stop before unported logging, after the F8 store.
No successful native body is intercepted. This is not a source Property
compiler or an owned Weapon property-instance implementation.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as nm
from unicorn import UC_HOOK_CODE
class Required(Exception):pass

def run(image,pc,c):
 m=nm.Machine(pc)
 old=(nm.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 actor=m.alloc(0x200,0xA5);world=m.alloc(0x200,0xA5);gmap=m.alloc(0x80,0xA5);tiles=m.alloc(0x60,0xA5)
 terrains=[m.alloc(0xC0,0xA5) for _ in range(2)];variations=[m.alloc(0x40,0xA5) for _ in range(2)]
 def terrain_ptr(which):return 0 if which is None else terrains[which]
 def snapshot(which):return None if which==0 else terrains.index(which)
 def list_head(values):
  head=0
  for value in reversed(values):
   node=m.alloc(8,0xA5);m.w(node,value);m.w(node+4,head);head=node
  return head
 prop=m.alloc(0x100,0xA5)
 for i,t in enumerate(terrains):
  m.w(t+0xB0,list_head([prop if v else 0 for v in c['properties'][i]]))
  m.w(variations[i]+0x1C,t);m.w(tiles+i*0x30,variations[i])
 m.w(0xB36534,world);m.w(world+0x30,gmap);m.w(gmap+0x1C,2);m.w(gmap+0x28,tiles)
 m.w(actor+8,c['flags']);m.w(actor+0x100,c['state_low']);m.w(actor+0x104,c['state_high'])
 m.w(actor+0x44,c['tile']);m.w(actor+0x48,0);m.w(actor+0xF8,terrain_ptr(c['old']))
 m.w(actor+0x7C,m.alloc(0x200,0xA5) if c['organization'] else 0)
 m.w(actor+0x78,m.alloc(0x200,0xA5) if c['weapon'] else 0);m.w(actor+0x80,0)
 if c['element']!='absent':
  e=m.alloc(0x40,0xA5);parent=m.alloc(0x200,0xA5)
  m.w(actor+0x80,e);m.w(e+4,actor);m.w(e+0x10,0 if c['element']=='unowned' else parent)
  m.wb(e+0x1C,0 if c['element']=='unready' else 1)
  m.w(parent+8,(1<<29) if c['element']=='disabled_parent' else 0)
  m.w(parent+0x100,0);m.w(parent+0x104,0);m.w(parent+0xF8,terrain_ptr(c['parent_terrain']))
 pins={};stops={};events=[]
 boundaries={0x579EB0:'required_Organization579EB0',0x581810:'required_Weapon581810',0x5035E0:'required_Property5035E0',0x4D4211:'required_null_terrain_diagnostic'}
 def observe(_,address,size,__):
  if image.base<=address<image.base+image.image_size:
   raw=image.range(address,size);assert raw==nm.P.range(address,size)==nm.V.range(address,size)
   if address in boundaries:stops[f'{address:08x}']=raw.hex()
   else:pins[f'{address:08x}']=raw.hex()
  if address in [0x4D10A0,0x570560,0x4D12C0,0x4EA200,0x54ECE0]:events.append(f'{address:08x}')
  if address in boundaries:raise Required(boundaries[address])
 m.u.hook_add(UC_HOOK_CODE,observe)
 outcome='returned'
 try:m.call(0x4D4130,actor,[],0)
 except Required as e:outcome=str(e)
 assert not m.null_accesses and m.outcome is None
 return dict(outcome=outcome,current_terrain=snapshot(m.r(actor+0xF8)),events=events),pins,stops

def inputs():
 base=dict(flags=0,state_low=0,state_high=0,tile=1,old=None,organization=False,weapon=False,properties=[[],[]],element='absent',parent_terrain=0)
 for old in [None,0,1]:
  for tile in [0,1]:
   for org in [False,True]:yield {**base,'old':old,'tile':tile,'organization':org}
 for props in [[[],[]],[[1],[]],[[],[1]],[[0],[1]],[[0],[0]]]:
  for old in [None,0,1]:
   for tile in [0,1]:yield {**base,'old':old,'tile':tile,'weapon':True,'properties':props}
 for kind in ['unowned','unready','active_parent','disabled_parent']:
  for parent in [None,0,1]:yield {**base,'element':kind,'parent_terrain':parent,'weapon':True}
 for flags,low,high in [(1<<29,0,0),(1<<12,0,0),(0,1<<24,0),(0,1<<25,0),(0,1<<26,0),(0,0,4),(0,0,16),(0,1,1)]:yield {**base,'flags':flags,'state_low':low,'state_high':high}

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 cases=[];pins={};stops={}
 for pc in ['Single24','Double53','Extended64']:
  for c in inputs():
   a,pa,sa=run(nm.P,pc,c);b,pb,sb=run(nm.V,pc,c);assert a==b and pa==pb and sa==sb
   pins.update(pa);stops.update(sa);cases.append(dict(precision=pc,input=c,output=a))
 out=dict(schema=1,scope=__doc__,paired_cases=len(cases),native_executions=len(cases)*2,cases=cases,executed_instruction_pins=pins,required_boundary_entry_pins=stops,source_images=[dict(name=name,sha256=hashlib.sha256(image.b).hexdigest()) for name,image in [('original',nm.P),('v14',nm.V)]])
 target=HERE/'native-actor-terrain-refresh.json'
 if args.write:
  assert not target.exists();target.write_text(json.dumps(out,indent=2)+'\n')
 else:assert json.loads(target.read_bytes())==out
 print(json.dumps(dict(paired_cases=len(cases),native_executions=len(cases)*2,executed_pins=len(pins),required_boundary_pins=len(stops),sha256=hashlib.sha256(target.read_bytes()).hexdigest())))
if __name__=='__main__':main()
