"""Bounded full7016A0 ->702380 publication ->700E30 confluence laboratory.

Actual native constructor bodies/CString/list/geometry/property/Ni math execute.
Malloc, Win32/ATL storage, hardware x87 transcendental and named current Bool
lookup/terrain are explicit transports. Heap bytes are laboratory zero-backed;
known masks identify only actual native stores, never malloc-zero semantics.
The pre-publication204 field is EXPLICIT indeterminate-input class, not zero.
This proves only the tested read-only source/environment lifecycle domain;
it does not authorize a Rust completed camera, current VWorld or renderer.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
import probe_camera_constructor as P

def main():
 rows=[];pins={};trans=[]
 for image,pc,lists in itertools.product(('original','v14'),P.R.PRECISIONS,(0,1)):
  reference=None
  for index in (0,1,0x7fffffff,0x80000000,0xffffffff):
   out=P.probe(index,lists,True,image,pc,True)
   assert out['passed'],(image,pc,lists,index,out)
   pins.update(out.pop('instructions'));trans.extend(out.pop('hardware_transports'))
   compare={k:v for k,v in out.items()if k not in('pins','heap_allocated')}
   if reference is None:reference=compare
   else:assert reference==compare,(image,pc,lists,index,[k for k in compare if compare[k]!=reference[k]])
   assert out['camera_words'][0x204//4]==0
   assert out['camera_known'][0x204//4]==0xffffffff
   assert len(out['descriptors'])==15*lists
   rows.append(dict(image=image,precision=pc,render_lists=lists,indeterminate_index_input=index,output=out))
 for a,b in pins.items():
  va=int(a,16)
  assert P.R.original.range(va,len(bytes.fromhex(b))).hex()==P.R.modified.range(va,len(bytes.fromhex(b))).hex()==b
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),probe_sha256=hashlib.sha256((HERE/'probe_camera_constructor.py').read_bytes()).hexdigest(),rows=rows,instructions=pins,hardware_transports=trans)
 path=HERE/'native-camera-constructor.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),full_ctor_publication_reset_executions=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
