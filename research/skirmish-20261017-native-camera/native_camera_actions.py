"""Actual7010D0/7005F0 after bounded full constructor/publication/UI reset.
Native CameraScaleZoom UVar34 laboratory Float has actual flags10; actual
40E0A0/4B29B0 bodies execute. ABI/Windows/hardware/terrain transports are the
same disclosed constructor laboratory. Positive directory1/disabled inputs
are explicit branch inputs, not claims of a new actual second publication.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import probe_camera_actions as P

def main():
 rows=[];pins={}
 cases=[('reset',0,0,0),('zoom',P.R.f32(-1),0,0),('zoom',P.R.f32(.25),0,0),('zoom',P.R.f32(9),0,0),('zoom',P.R.f32(9),0,1),('zoom',P.R.f32(9),1,0)]
 for image,pc,lists,case in itertools.product(('original','v14'),P.R.PRECISIONS,(0,1),cases):
  action,zoom,disabled,ordinal=case
  out=P.probe(0,lists,True,image,pc,True,action,zoom,disabled,ordinal)
  assert out['passed'],out
  pins.update(out.pop('instructions'));out.pop('hardware_transports')
  rows.append(dict(image=image,precision=pc,render_lists=lists,action=action,zoomword=zoom,disabled=disabled,ordinal=ordinal,output=out))
 for address,raw in pins.items():
  a=int(address,16);assert P.R.original.range(a,len(bytes.fromhex(raw))).hex()==P.R.modified.range(a,len(bytes.fromhex(raw))).hex()==raw
 doc=dict(scope=__doc__,probe_sha256=hashlib.sha256((HERE/'probe_camera_actions.py').read_bytes()).hexdigest(),rows=rows,instructions=pins)
 path=HERE/'native-camera-actions.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),native_exec=len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
