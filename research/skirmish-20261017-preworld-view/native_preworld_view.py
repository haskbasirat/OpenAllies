"""Bounded actual442C00 constructor prefix: globalB36538 publishes the
same allocation before direct nativeAD=0, independent of a simulation World.
Base/embedded constructor calls are declared transport seams inherited from
the frozen original prefix oracle. This is not a complete VWorld constructor,
application startup capture, second-init/teardown or battle-ready proof.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10'))
import native_vworld_prefix as base
h=base.h
H=lambda b:hashlib.sha256(b).hexdigest()
def main():
 h.VISITED.clear();rows=[]
 for build,image in [('original',base.base.ORIGINAL),('v14',base.base.V14)]:
  h.P=image
  for pc in h.CW:
   for fill in [0,0xa5,0xff]:
    m=base.Machine(pc,fill);m.w(0xb36538,0xdeadbeef);events=[]
    def published(machine):
     assert machine.r(0xb36538)==machine.view
     events.append(dict(event='published_global',same_allocation=True,ad=machine.b(machine.view+0xad)))
    def ad_cleared(machine):
     assert machine.r(0xb36538)==machine.view and machine.b(machine.view+0xad)==0
     events.append(dict(event='ad_initialized',same_allocation=True,ad=0))
    m.probes[0x442c51]=published;m.probes[0x442cc1]=ad_cleared
    out=m.ctor()
    assert events==[dict(event='published_global',same_allocation=True,ad=fill),dict(event='ad_initialized',same_allocation=True,ad=0)]
    rows.append(dict(build=build,precision=pc,initial_fill=fill,events=events,fields=out))
 h.P=base.base.ORIGINAL
 excluded={0x5f1d00,0x424b90,0x42e730,0x442d0e}
 pins={hex(a):v for a,v in sorted(h.VISITED.items()) if a not in excluded}
 for a,v in pins.items():assert base.base.ORIGINAL.range(int(a,16),len(bytes.fromhex(v)))==base.base.V14.range(int(a,16),len(bytes.fromhex(v)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),base_sha256=H(Path(base.__file__).read_bytes()),machine_sha256=H(Path(h.__file__).read_bytes()),rows=rows,pins=pins,
  exclusions=['5F1D00 base,424B90 embedded,42E730 embedded constructor bodies are supplied nonalias return seams',
   '442D0E cutoff before later constructor allocations; no full442C00/allocator/unwind parity',
   'Rust one-time adoption and weak cell identity are separate portable ownership guarantees, not original-instruction behavior'])
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-preworld-view.json'
 if '--write' in sys.argv:
  with p.open('xb') as f:f.write(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(executions=len(rows),pins=len(pins),sha256=H(data))))
if __name__=='__main__':main()
