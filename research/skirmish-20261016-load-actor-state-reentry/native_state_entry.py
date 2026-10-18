"""Fresh full4D4A90 in its reached null/empty branch; no intercepted bodies.

Executes45E620/4B4590 (BodyNULL),45E600 null old property,4F0460,
4D0A00 null FX and actual generation checks. Positive children are not stubbed.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_EBP
H=lambda b:hashlib.sha256(b).hexdigest()
def run(c,pc,image):
 m=h.Machine(pc)
 if image is h.V:
  old=(h.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
  if new>old:m.u.mem_map(image.base+old,new-old)
  m.u.mem_write(image.base,image.b[:4096])
  for vs,r,n,o in image.sections:
   if n:m.u.mem_write(image.base+r,image.b[o:o+min(n,max(vs,n))])
 actor,definition,desc,world,resource,manager=[m.alloc(n)for n in [0x200,0x400,0x40,0x100,0x100,0x60000]]
 # Bounded constructor store interval with the actual constructor's explicit
 # register inputs; this is not execution of full4624B0/TLS/base construction.
 m.u.mem_write(definition+0x188,b'\xff'*28)
 for reg,value in [(UC_X86_REG_ESI,definition),(UC_X86_REG_EBX,0),(UC_X86_REG_EBP,1)]:m.u.reg_write(reg,value)
 m.u.emu_start(0x462531,0x462543,count=20)
 assert m.r32(definition+0x188,3)==[0,0,1]
 m.call(0x79e130,definition+0x194);m.call(0x79e130,definition+0x19c)
 assert m.r32(definition+0x194,4)==[0]*4
 m.w8(desc+0x1f,0xff)
 m.u.reg_write(UC_X86_REG_ESI,desc);m.u.reg_write(UC_X86_REG_EBX,0)
 m.u.emu_start(0x4742f6,0x4742f9,count=5)
 assert m.r8(desc+0x1f)==0
 m.w32(actor+4,definition);m.w32(actor+0x15c,c['old_view']);m.w32(actor+0x164,c['group'])
 m.w32(actor+0x100,*c.get('state_bits',[1,0]));m.w32(actor+8,0x20000100)
 m.w32(actor+0x60,m.alloc(0x20)if c['body']else 0)
 m.w32(actor+0x78,m.alloc(0x20)if c.get('properties_present',False)else 0)
 m.w32(definition+0x18c,c['stance_count'])
 array=m.alloc(4*max(1,c['stance_count']));m.w32(definition+0x188,array)
 for i in range(c['stance_count']):
  rule=m.alloc(0x20);m.w32(array+4*i,rule);m.w32(rule+0xc,0)
 m.w32(desc+12,c.get('index',0));m.w8(desc+0x1f,int(c['remove_group']))
 m.w32(0xb36534,world);m.w32(world+0x28,resource);m.w32(resource+0xa8,c['mode']);m.w32(0xb364c0,manager)
 # All generations/slots are genuinely empty in this explicit fixture.
 m.call(0x4d4a90,actor,(desc,0));assert m.final_esp==h.STACK+12 and m.fpu_top()==0
 return dict(view=(lambda x:x if x<0x80000000 else x-0x100000000)(m.r32(actor+0x15c)),flags=m.r32(actor+8),constructor_interval_stance_count=0,constructor_interval_remove_group=0,constructor_function_state_fx_heads_tails=[0]*4)
def inputs():
 for count in [0,1,2]:
  for body,old in ([(False,-1),(True,-1)]if count==0 else[(False,-1),(False,0)]):
   for mode,group in itertools.product([0,1],[0,1,0x10001]):
    yield dict(stance_count=count,body=body,old_view=old,mode=mode,group=group,remove_group=group!=0)
def main():
 rows=[]
 for pc in h.PRECISION:
  for c in inputs():
   a=run(c,pc,h.P);b=run(c,pc,h.V);assert a==b
   rows.append(dict(precision=pc,input=c,output=a))
 actual=HERE/'actual-3172-frozen.json'
 actual_rows=[]
 if actual.exists():
  for c in json.loads(actual.read_text())['cases']:
   assert c['stance_count']==0 and c['enter_fx_empty']
   components=c['component_presence'];assert not components['Organization'] and not components['Economy'] and not components['Character']
   for state in c['states']:
    source=state['source'];assert not source['properties']
    remove=source['mind']['remove_from_group']['Boolean']['value']if source['mind']else False
    inp=dict(stance_count=c['stance_count'],body=components['Body'],properties_present=components['Properties'],old_view=c['old_view'],mode=c['map_resource_mode'],group=c['group_reference'],remove_group=remove,index=state['index'],state_bits=c['state_bits'])
    a=run(inp,c['precision'],h.P);b=run(inp,c['precision'],h.V);assert a==b and a['view']==c['view_after']
    actual_rows.append(dict(precision=c['precision'],input=inp,output=a,source_id=source['id'],scope='actual saved fields; explicit null component/list pointers certified by source capture; full Init prefix separate'))
 pins=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())]
 assert not any(int(p['va'],16)in h.ALLOCATORS for p in pins)
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,actual_input_sha256=H(actual.read_bytes())if actual.exists()else None,cases=rows,actual_rows=actual_rows,executed_address_byte_pins=pins,seams={})
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-state-entry.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),actual_rows=len(actual_rows),machines=2*(len(rows)+len(actual_rows)),executed_pins=len(pins),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
