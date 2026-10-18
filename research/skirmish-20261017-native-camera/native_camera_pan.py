"""Actual700500 full planar calculation after real constructor/publication/reset;
6FF130 is an explicit tail-call observation transport with stable float args.
CurrentUVar scalePan10/ScrollSpeed2 are laboratory numeric inputs. Native
FILD unsigned32 correction, x87 store order run at all PCs on both images.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import probe_camera_pan as P
rows=[];pins={}
for image,pc,case in itertools.product(('original','v14'),P.R.PRECISIONS,[(0,[0,0]),(16,[P.R.f32(1),P.R.f32(-1)]),(0xffffffff,[P.R.f32(.5),P.R.f32(.25)])]):
 millis,direction=case;out=P.probe(0,0,True,image,pc,True,millis,direction);assert out['passed'],out
 pins.update(out.pop('instructions'));out.pop('hardware_transports')
 rows.append(dict(image=image,precision=pc,millis=millis,direction=direction,target=out['pan_target'],basis=out['basis'],center=out['center']))
for a,b in pins.items():assert P.R.original.range(int(a,16),len(bytes.fromhex(b))).hex()==P.R.modified.range(int(a,16),len(bytes.fromhex(b))).hex()==b
doc=dict(scope=__doc__,rows=rows,instructions=pins,probe_sha256=hashlib.sha256((HERE/'probe_camera_pan.py').read_bytes()).hexdigest())
p=HERE/'native-camera-pan.json'
if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
else:assert json.loads(p.read_text())==doc
print(json.dumps(dict(rows=len(rows),native_exec=len(rows),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
