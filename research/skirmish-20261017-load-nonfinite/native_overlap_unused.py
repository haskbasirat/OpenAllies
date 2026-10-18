"""Whole46EF30: raw NaN/Inf facing comparisons on non-box shapes.

Explicit isolated probes. No native calls are seamed. Both images execute
separately at all3 PCs with masked exceptions. Native does read facing for
FUCOMPP equality even on circle/point, but the result cannot select rotated
footprints when the smaller shape is0/1. x87 exception flags are not part of
the existing Rust bool-overlap API and are recorded here without a parity claim.
"""
from pathlib import Path
import argparse,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_FPSW
sha=lambda b:hashlib.sha256(b).hexdigest()

def shape(kind):return dict(kind=kind,radius_bits=0 if kind==0 else 0x3fa00000,bounds=[0xbf800000,0xbf800000,0x3f800000,0x3f800000])
def inputs():
 out=[]
 for left,right in [(0,0),(0,1),(1,0),(1,1),(2,0),(2,1),(0,2),(1,2)]:
  for far in [False,True]:
   base=dict(a=dict(shape=shape(left),position=[0,0],facing=[0,0xbf800000]),
     b=dict(shape=shape(right),position=[0x41200000 if far else 0x3e800000,0],facing=[0,0xbf800000]))
   for name,poison in [('finite',None),('nan',[0xffc00000,0xffc00000]),('infinite',[0x7f800000,0xff800000])]:
    import copy
    c=copy.deepcopy(base)
    if poison is not None:
     for key in ['a','b']:
      if c[key]['shape']['kind']<2:c[key]['facing']=poison
    out.append(dict(name=f'{left}-{right}-{int(far)}-{name}',**c))
 return out
def run(c,image,pc):
 m=Machine(pc)
 old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 a,b,sa,sb=[m.alloc(n)for n in[24,24,24,24]]
 for key,descriptor,source in [('a',a,sa),('b',b,sb)]:
  p=c[key];s=p['shape']
  m.w(source,s['kind']);m.w(source+4,s['radius_bits'])
  for i,word in enumerate(s['bounds']):m.w(source+8+4*i,word)
  for i,word in enumerate([source,*p['facing'],*p['position'],0]):m.w(descriptor+4*i,word)
 reads=[]
 def read(u,access,address,size,value,data):
  for label,descriptor in [('a',a),('b',b)]:
   for off in [4,8]:
    if address==descriptor+off:reads.append(f'{label}.facing{(off-4)//4}')
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 result=m.call(0x46ef30,a,[b])&255
 return dict(result=bool(result),facing_reads=reads,x87_status=m.u.reg_read(UC_X86_REG_FPSW)&0xffff),m.visited
def main():
 args=argparse.ArgumentParser();args.add_argument('--create',action='store_true');create=args.parse_args().create
 rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  baseline={}
  for c in inputs():
   pair=[]
   for image in [original,modified]:
    value,visited=run(c,image,pc);pair.append(value)
    for address,raw in visited.items():
     assert raw==original.range(address,len(raw))==modified.range(address,len(raw));pins[address]=raw
   assert pair[0]==pair[1]
   key=c['name'].rsplit('-',1)[0]
   if c['name'].endswith('-finite'):baseline[key]=pair[0]['result']
   else:assert pair[0]['result']==baseline[key]
   assert pair[0]['facing_reads']
   rows.append(dict(precision=pc,input=c,native=pair[0]))
 doc=dict(scope=__doc__,producer_sha256=sha(Path(__file__).read_bytes()),original_sha256=sha(original.b),v14_sha256=sha(modified.b),
   rows=rows,pins=[dict(address=hex(a),bytes=b.hex())for a,b in sorted(pins.items())],seams=[])
 path=HERE/'native-overlap-unused.json'
 if create:
  with path.open('x')as f:json.dump(doc,f,indent=1);f.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),fresh_executions=len(rows)*2,pins=len(pins),sha256=sha(path.read_bytes()),immutable=not create)))
if __name__=='__main__':main()
