"""Full native PLAY_SFX compilation of the captured actual Farm source input.
Typed source input/provenance is captured independently by the Rust source
adapter; both native images execute original constructor/base/derived loader.
This certifies compilation only, not audio decoding/playback or battle readiness.
"""
from pathlib import Path
import json,sys,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_fx_sound as core

def main():
 source=HERE/'actual-source-input.json';captured=json.loads(source.read_text());c=captured['input']
 rows=[];pins={}
 for pc in core.nm.PRECISIONS:
  a,pa=core.run(c,'original',pc);b,pb=core.run(c,'v14',pc);assert a==b
  observed=dict(a);observed.pop('error');observed.pop('events');assert a['error']is None
  assert observed==captured['compiled'],(pc,observed,captured['compiled'])
  pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert core.ORIGINAL.range(va,len(raw))==core.nm.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())])
 path=HERE/'native-actual-fx-sound.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
