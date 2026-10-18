"""Direct World constructor4F7D00 stores of140/184/194/198 through4F8048.
Inherited embedded-constructor/allocator seams are explicit and disjoint.
This proves only these fields, not whole World construction or any Load.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE/'map-resource-mode'))
import native_tail_constructor as base
h=base.h
class Machine(base.Machine):
 def run(self):
  state=super().run()
  state['counters']={f'{offset:x}':self.r(self.world+offset)for offset in[0x140,0x184,0x194,0x198]}
  return state
def main():
 rows=[];h.VISITED.clear()
 for image,pe in [('original',base.ORIGINAL),('v14',base.V14)]:
  h.P=pe
  for pc in h.CW:
   for fill in[0,0xa5,0xff]:
    state=Machine(fill,pc).run();assert set(state['counters'].values())=={0}
    rows.append(dict(image=image,precision=pc,fill=fill,output=state))
 h.P=base.ORIGINAL
 excluded={0x511df0,0x4ee0a0,0x85d870,0x50aba0,0x50b3a0,0x6f7810,0x4f8048}
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded}
 for a,b in pins.items():assert base.ORIGINAL.range(int(a,16),len(bytes.fromhex(b)))==base.V14.range(int(a,16),len(bytes.fromhex(b)))
 doc=dict(schema=1,scope=__doc__,producer_sha256=h.H(Path(__file__).read_bytes()),parent_producer_sha256=h.H(Path(base.__file__).read_bytes()),machine_sha256=h.H(Path(h.__file__).read_bytes()),rows=rows,pins=pins)
 p=HERE/'native-world-counters.json';data=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),pins=len(pins),sha256=h.H(data))))
if __name__=='__main__':main()
