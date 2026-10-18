"""Native40CF00→5C3000 argument transport, with explicit required children.

The reused UI oracle transports its actual child calls (camera, VWorld Init,
publication and enqueue). It is not a full VWorld or live GPU acceptance.
40CF00,5C3000,the derived task ctor and current menu's bare E8 RET execute.
The word is independent of managed World identity; B8's actual first store is
separately pinned by native-vworld-prefix and its current exact-source replay.
"""
from pathlib import Path
import sys,importlib.util,itertools,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
dependency=ROOT/'research/skirmish-20261017-ui-world-completion/native_loading.py'
spec=importlib.util.spec_from_file_location('loading_argument_dependency',dependency);h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
invoke=h.invoke
def wrapper(m,pc,p,args=()):
 if pc==0x5c3000:
  m.w(0xb3653c,p);state=m.alloc(8);menu=m.alloc(4)
  m.w(menu,0xa7edd0);m.w(state+4,menu);m.w(0xb3979c,6);m.w(0xb3975c+6*4,state)
  return invoke(m,0x40cf00,0,args)
 return invoke(m,pc,p,args)
h.invoke=wrapper
def main():
 rows=[];pins={}
 for precision,word in itertools.product(h.PRECISIONS,[0,7,0xffffffff]):
  outputs=[]
  for image in ['original','v14']:
   m=h.Machine(image,precision);out,actual=h.run(m,dict(op='ui_init',old=0x55667788,kingdoms=[7,9],revision=word,fail=None));pins.update(actual);outputs.append(out)
   assert ['view_init',word]in out['trace'];assert out['error']is None
  assert outputs[0]==outputs[1];rows.append(dict(precision=precision,word=word,output=outputs[0]))
 for pc,raw in pins.items():assert bytes.fromhex(raw)==h.original.range(pc,len(bytes.fromhex(raw)))==h.modified.range(pc,len(bytes.fromhex(raw)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),dependency_sha256=hashlib.sha256(dependency.read_bytes()).hexdigest(),rows=rows,instructions=[dict(va=hex(pc),bytes=raw)for pc,raw in sorted(pins.items())])
 p=HERE/'native-ui-argument.json';data=(json.dumps(doc,indent=2)+'\n').encode()
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(pairs=len(rows),runs=len(rows)*2,executed_pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
