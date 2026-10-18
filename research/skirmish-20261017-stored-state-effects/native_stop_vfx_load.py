"""Unchanged42C900/42AF90 STOP_VFX derived loader and partial target store.
Completed common42C1B0, typed vfx_ids getter, CString transport and same local
first-VFX directory lookup are explicit services. No source parser/registry
completion or initialized actor is implied. Actual attached validation executes.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
class Required(Exception):pass
def cases():
 return [dict(name='attached',ids='Smoke',targets=[True],mapping={'smoke':0}),
 dict(name='casefold',ids='sMoKe',targets=[True],mapping={'smoke':0}),
 dict(name='unattached_store_before_error',ids='smoke',targets=[False],mapping={'smoke':0}),
 dict(name='first_map_entry_retained',ids='smoke',targets=[True,False],mapping={'smoke':0}),
 dict(name='first_unattached_is_not_replaced',ids='smoke',targets=[False,True],mapping={'smoke':0}),
 dict(name='missing',ids='missing',targets=[True],mapping={'smoke':0}),
 dict(name='empty_ids',ids='',targets=[True],mapping={'smoke':0}),
 dict(name='empty_directory',ids='smoke',targets=[],mapping={}),
 dict(name='required_ids',ids=None,targets=[True],mapping={'smoke':0})]
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 p=m.alloc(0x40);m.u.mem_write(p,b'\xcd'*0x40);m.call(0x42af90,p)
 source=m.alloc(4);context=m.alloc(4);events=[]
 targets=[]
 for attached in c['targets']:
  target=m.alloc(0x70);m.wb(target+0x65,int(attached));targets.append(target)
 def base(mm):
  assert mm.ecx()==p and mm.arg(1)==source and mm.arg(2)==context
  events.append('completed_common');mm.ret(pop=8)
 m.seam(0x42c1b0,base)
 def text(pointer):
  units=[]
  while True:
   unit=bytes(m.u.mem_read(pointer,2));pointer+=2
   if unit==b'\0\0':return b''.join(units).decode('utf-16-le')
   units.append(unit)
 def getter(mm):
  assert mm.ecx()==source and text(mm.arg(1))=='vfx_ids'
  events.append('vfx_ids')
  if c['ids'] is None:raise Required('required_ids')
  header=m.alloc(16+len(c['ids'])*2+2);m.w(header+12,100)
  m.u.mem_write(header+16,(c['ids']+'\0').encode('utf-16-le'));mm.ret(m.array([header+16]),4)
 m.seam(0x5b54d0,getter)
 m.seam(0x4012c0,lambda mm:mm.ret(mm.arg(1)))
 def find(mm):
  assert mm.ecx()==context
  name=text(m.r(mm.arg(1)));events.append(['find',name])
  index=c['mapping'].get(name.lower())
  mm.ret(0 if index is None else m.array([0,targets[index]]),4)
 m.seam(0x4154d0,find)
 def failure(mm):
  caller=mm.r(mm.u.reg_read(nm.UC_X86_REG_ESP))
  assert caller in(0x42c979,0x42c991)
  raise Required('unknown_vfx'if caller==0x42c979 else'unattached')
 m.seam(0x410c40,failure)
 outcome='ok'
 try:m.call(0x42c900,p,[source,context])
 except Required as error:outcome=str(error)
 target=m.r(p+0x3c)
 return dict(outcome=outcome,target=targets.index(target)if target in targets else None,complete=outcome=='ok',events=events),m.visited
def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   pair=[]
   for image in('original','v14'):
    output,visited=run(c,image,pc);pair.append(output);pins.update(visited)
   assert pair[0]==pair[1];rows.append(dict(precision=pc,input=c,output=pair[0]))
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-stop-vfx-load.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
