"""Native579EB0->4364C0->570550->4D4130 real ordered member terrain refresh.
Both original/v14 and allPC. Explicit native arrays, Element owners/ready flags,
parent F8 and map Variation/Terrain objects are fixture inputs. Child Org and
Properties are actually null in this bounded domain. No successful body is
intercepted. Missing Element stops before its required null-owner read.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'));import native_machine as nm
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX
class Required(Exception):pass

def run(image,pc,c):
 m=nm.Machine(pc);old=(nm.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 parent=m.alloc(0x200,0xA5);org=m.alloc(0x100,0xA5);world=m.alloc(0x200,0xA5);gmap=m.alloc(0x80,0xA5);tiles=m.alloc(0x60,0xA5)
 terrains=[m.alloc(0xC0,0xA5)for _ in range(2)];variations=[m.alloc(0x40,0xA5)for _ in range(2)]
 m.w(0xB36534,world);m.w(world+0x30,gmap);m.w(gmap+0x1C,2);m.w(gmap+0x28,tiles)
 for i,t in enumerate(terrains):m.w(variations[i]+0x1C,t);m.w(tiles+i*0x30,variations[i])
 m.w(parent+8,0);m.w(parent+0x100,0);m.w(parent+0x104,0);m.w(parent+0xF8,terrains[c['parent_terrain']])
 actors=[];mapping={}
 for i in range(3):
  a=m.alloc(0x200,0xA5);e=m.alloc(0x40,0xA5);actors.append(a);mapping[a]=i
  m.w(a+8,c['flags']);m.w(a+0x100,c['state_low']);m.w(a+0x104,0)
  m.w(a+0x44,1);m.w(a+0x48,0);m.w(a+0xF8,0);m.w(a+0x78,0);m.w(a+0x7C,0)
  m.w(a+0x80,0 if i==c['missing_element']else e)
  m.w(e+4,a);m.w(e+0x10,parent);m.wb(e+0x1C,c['ready'])
 members=m.alloc(max(4,len(c['members'])*4),0xA5)
 for i,value in enumerate(c['members']):m.w(members+i*4,0 if value is None else actors[value])
 m.w(org+0x28,members);m.w(org+0x2C,len(c['members']))
 events=[];pins={};boundaries={}
 def observe(_,address,size,__):
  if image.base<=address<image.base+image.image_size:
   raw=image.range(address,size);assert raw==nm.P.range(address,size)==nm.V.range(address,size)
   if address==0x570550 and m.u.reg_read(UC_X86_REG_ECX)==0:
    boundaries[f'{address:08x}']=raw.hex();raise Required()
   assert address not in m.seams,'no native successful seam expected in this domain'
   pins[f'{address:08x}']=raw.hex()
  if address==0x4D4130:events.append(mapping[m.u.reg_read(UC_X86_REG_ECX)])
 m.u.hook_add(UC_HOOK_CODE,observe);outcome='returned'
 try:m.call(0x579EB0,org,[],0)
 except Required:outcome='required_null_Element570550'
 assert not m.null_accesses and m.outcome is None
 f8=[None if m.r(a+0xF8)==0 else terrains.index(m.r(a+0xF8))for a in actors]
 return dict(outcome=outcome,member_calls=events,f8=f8),pins,boundaries

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 rows=[];pins={};boundaries={}
 base=dict(flags=0,state_low=0,parent_terrain=0,ready=0,missing_element=-1)
 inputs=[dict(base,members=members,ready=ready,flags=flags)for members in [[],[None],[0],[None,2,0,None,2,1],[1,None,0]]for ready in [0,1]for flags in [0,1<<29]]
 inputs += [dict(base,members=[0,None,1,2],missing_element=missing,ready=1)for missing in [0,1,2]]
 for pc in ['Single24','Double53','Extended64']:
  for c in inputs:
   a,pa,ba=run(nm.P,pc,c);b,pb,bb=run(nm.V,pc,c);assert a==b and pa==pb and ba==bb
   rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);boundaries.update(ba)
 v=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=len(rows)*2,cases=rows,executed_instruction_pins=pins,required_boundary_entry_pins=boundaries,source_images=[dict(name=name,sha256=hashlib.sha256(image.b).hexdigest())for name,image in [('original',nm.P),('v14',nm.V)]])
 p=HERE/'native-organization-terrain.json'
 if args.write:assert not p.exists();p.write_text(json.dumps(v,indent=2)+'\n')
 else:assert json.loads(p.read_bytes())==v
 print(json.dumps(dict(paired_cases=len(rows),native_executions=len(rows)*2,executed_pins=len(pins),required_boundary_pins=len(boundaries),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
