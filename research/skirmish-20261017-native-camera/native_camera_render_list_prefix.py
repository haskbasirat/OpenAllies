"""Actual712660 dependency-stop prefixes before reached4B2C30 Bool lookup.
All earlier native stores execute; stop occurs at external lookup seam entry,
not in a pending memory-write hook. CString/OS storage transports as ctor proof.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import probe_camera_render_list_prefix as P

def main():
 rows=[];pins={}
 for image,pc,cut in itertools.product(('original','v14'),P.R.PRECISIONS,(0,3,13)):
  out=P.probe(renderlists=1,image=image,precision=pc,details=True,fail_lookup=cut);assert out['passed'],out;pins.update(out.pop('instructions'));rows.append(dict(image=image,precision=pc,cut=cut,output=out));assert len(out['descriptors'])==cut+2 and not out['descriptors'][-1]['pairs'];assert out['enabled_mask']==(1<<(cut+2))-1
 for a,b in pins.items():
  va=int(a,16);assert P.R.original.range(va,len(bytes.fromhex(b))).hex()==P.R.modified.range(va,len(bytes.fromhex(b))).hex()==b
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),probe_sha256=hashlib.sha256((HERE/'probe_camera_render_list_prefix.py').read_bytes()).hexdigest(),rows=rows,instructions=pins)
 path=HERE/'native-camera-render-list-prefix.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
