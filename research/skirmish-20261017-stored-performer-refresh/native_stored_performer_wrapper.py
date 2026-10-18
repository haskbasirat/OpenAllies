"""Actual57F4C0 wrapper+57F1E0+57E430 empty/disabled-table return path.
Explicit isolated fields are guard inputs, not source-constructed live actors.
The ignored Property argument is poisoned. No successful child seams are used;
real source requirements, positive CV and stored admission have separate tests.
"""
from pathlib import Path
import sys,json,hashlib,itertools
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
ORIGINAL=nm.original

def run(image,pc,primary,secondary,biases):
 nm.original=image;m=nm.Machine(pc)
 actor=m.alloc(0x180);definition=m.alloc(0x220);m.w(actor+4,definition)
 props=m.alloc(0x100);array=m.alloc(4*max(1,len(biases)))
 for index,bits in enumerate(biases):
  instance=m.alloc(0x20);source=m.alloc(0x40);m.w(source+0x28,bits);m.w(instance+0xC,source);m.w(array+4*index,instance)
 m.w(props+0x10,array);m.w(props+0x14,len(biases));m.w(actor+0x78,props)
 def table(count):
  array=m.alloc(4*max(1,count));handles=[]
  for index in range(count):
   ability=m.alloc(0x30);m.w(ability+8,0);m.wb(ability+0x18,0);m.w(array+4*index,ability);handles.append(ability)
  return array,handles
 performer=m.alloc(0x80);m.w(performer+4,actor);p,ph=table(primary);m.w(performer+0x14,p);m.w(performer+0x18,primary);m.w(performer+0x60,nm.f32(99.0))
 if secondary is not None:
  defender=m.alloc(0x40);d,dh=table(secondary);m.w(defender+0x10,d);m.w(defender+0x14,secondary);m.w(actor+0xB0,defender)
 else:dh=[]
 m.call(0x57F4C0,performer,args=(0xDEADBEEF,))
 assert not m.events
 output={'value':m.r(performer+0x60),'primary_enabled':[m.rb(v+0x18)for v in ph],'defender_enabled':[m.rb(v+0x18)for v in dh]}
 assert output['value']==0
 assert 0x57F4C0 in m.visited and 0x57F4C5 in m.visited and 0x57E430 in m.visited
 return output,m.visited

def main():
 rows=[];pins={}
 for pc,primary,secondary,biases in itertools.product(nm.PRECISIONS,[0,1,3],[None,0,2],[[],[nm.f32(0.1)],[nm.f32(1.0),nm.f32(-2.0)]]):
  a,pa=run(ORIGINAL,pc,primary,secondary,biases);b,pb=run(nm.modified,pc,primary,secondary,biases)
  assert a==b and pa==pb;pins.update(pa)
  rows.append({'precision':pc,'primary_count':primary,'defender_count':secondary,'property_biases':biases,'output':a})
 for va,raw in pins.items():assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 result={'schema':1,'scope':__doc__,'paired_cases':len(rows),'native_executions':len(rows)*2,'successful_child_seams':[],
  'source_images':[{'name':name,'sha256':hashlib.sha256(image.b).hexdigest()}for name,image in [('original',ORIGINAL),('v14',nm.modified)]],
  'cases':rows,'executed_instruction_pins':{f'{va:08x}':raw.hex()for va,raw in sorted(pins.items())}}
 path=HERE/'native-stored-performer-wrapper.json'
 if '--write'in sys.argv:
  with path.open('x')as f:json.dump(result,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_bytes())==result
 print(json.dumps({'paired_cases':len(rows),'native_executions':len(rows)*2,'executed_pins':len(pins),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
