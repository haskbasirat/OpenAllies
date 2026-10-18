"""Actual7016A0 auxiliary quad/property mutations with actual C0 and renderer reads.
Default property globals are explicitly labNULL; root owns the real bootstrap.
All12 quad slots are replaced by actual fresh class constructors in the native
camera order. Owned fields exclude raw refcount/vtable/allocator-pointer ABI.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import probe_camera_properties as P

def main():
 rows=[];pins={}
 for image,pc in itertools.product(('original','v14'),P.R.PRECISIONS):
  out=P.probe(image=image,precision=pc,details=True);assert out['passed'],out;pins.update(out['instructions']);rows.append(dict(image=image,precision=pc,properties=out['properties'],counter=out['counter']))
 for a,b in pins.items():
  va=int(a,16);assert P.R.original.range(va,len(bytes.fromhex(b))).hex()==P.R.modified.range(va,len(bytes.fromhex(b))).hex()==b
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),probe_sha256=hashlib.sha256((HERE/'probe_camera_properties.py').read_bytes()).hexdigest(),camera_order=[4,9,3,8,5,2,10,11,7,6,1,0],c0_words=[0x3e4ccccd,0x3ecccccd,0x3f19999a],renderer24c=2,rows=rows,instructions=pins)
 path=HERE/'native-camera-properties.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
