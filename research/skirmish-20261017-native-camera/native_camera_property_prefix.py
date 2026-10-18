"""Native camera property dependency stops BEFORE701B60 C0 load or701EE7 renderer read.
Pre-instruction observers abort at the source read, preserving prior actual
property writes. They do not raise inside a pending write hook.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import probe_camera_property_prefix as P

def main():
 rows=[];pins={}
 for image,pc,cut in itertools.product(('original','v14'),P.R.PRECISIONS,('c0','renderer')):
  out=P.probe(image=image,precision=pc,details=True,property_cut=cut);assert out['passed'],out;pins.update(out.pop('instructions'));rows.append(dict(image=image,precision=pc,cut=cut,output=out))
 for a,b in pins.items():
  va=int(a,16);assert P.R.original.range(va,len(bytes.fromhex(b))).hex()==P.R.modified.range(va,len(bytes.fromhex(b))).hex()==b
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),probe_sha256=hashlib.sha256((HERE/'probe_camera_property_prefix.py').read_bytes()).hexdigest(),rows=rows,instructions=pins)
 path=HERE/'native-camera-property-prefix.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
