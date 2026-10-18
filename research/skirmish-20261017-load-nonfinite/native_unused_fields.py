"""Whole original4EE4C0/5536B0 with raw nonfinite unused pose words.

Explicit isolated probes, not an actual full saved-map execution. Both images
run separately at all3 PCs. Only8-byte node allocation/delete are transported.
Circle/point membership must preserve raw facing/Z without numeric validation;
box numeric read behavior is separately still a required Rust finite domain.
"""
from pathlib import Path
import copy,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-load-spatial'))
import native_load_spatial as old
sha=lambda b:hashlib.sha256(b).hexdigest()

def inputs():
 out=[]
 for shape in (0,1):
  for name,facing,height,fz in [
   ('finite',[0,0xbf800000],0x3f800000,0),
   ('indefinite-facing',[0xffc00000,0xffc00000],0x3f800000,0),
   ('nan-height',[0,0xbf800000],0x7fc12345,0),
   ('infinite-facing-z',[0,0xbf800000],0x3f800000,0x7f800000),
   ('all-unused-poison',[0x7f800000,0xff800000],0xffc00000,0x7fc00001),
  ]:
   kind=lambda radius:dict(kind=shape,radius_bits=radius if shape else 0,bounds=[0,0,0,0])
   member=dict(key=1,reference=16669,actor=dict(definition=1,flags=0x22242808,
      position=[1102053337,1128267776],facing=facing),height_bits=height,facing_z_bits=fz,
      cached_tile=[0,0],blocking=kind(0x3e4ccccd),soft=kind(0x3f000000),target=kind(0x3f000000),
      actor_category=False,wall_present=False)
   out.append(dict(name=f'{shape}-{name}',native_class=2,member=member,
     grid=dict(dimensions=[128,128],coordinate_shift=1,cell_size_bits=0x40000000),initial_flags=[0x321]*16384))
 return out

def main():
 create='--create' in sys.argv;rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  baseline={}
  for case in inputs():
   case['precision']=pc;pair=[]
   for image in (old.h.P,old.h.V):
    m=old.Machine(case,image);output=m.run()
    raw=list(__import__('struct').unpack('<IIII',m.u.mem_read(m.obj+0x28,16)))
    assert raw==[case['member']['height_bits'],*case['member']['actor']['facing'],case['member']['facing_z_bits']]
    pair.append(output)
   assert pair[0]==pair[1]
   shape=case['member']['soft']['kind']
   if case['name'].endswith('-finite'):baseline[shape]=pair[0]
   else:assert pair[0]==baseline[shape],(case['name'],pc)
   rows.append(dict(input=case,native=pair[0]))
 for address,raw in old.h.VISITED.items():
  assert bytes.fromhex(raw)==old.h.P.range(address,len(bytes.fromhex(raw)))==old.h.V.range(address,len(bytes.fromhex(raw)))
  pins[address]=raw
 for address in [0x4ee4c0,0x5536b0,0x5534f0,0x54f6f0]:assert address in pins,hex(address)
 doc=dict(schema=1,scope=__doc__,producer_sha256=sha(Path(__file__).read_bytes()),
   inherited_machine_sha256=sha(Path(old.__file__).read_bytes()),
   harness_sha256=sha(Path(old.h.__file__).read_bytes()),original_sha256=old.h.ORIGINAL_SHA,v14_sha256=old.h.V14_SHA,
   cases=rows,address_byte_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],
   seams={'0x6f7810':'successful8-byte native node allocation transport','0x6f7430':'delete transport, insertion/no-op probes do not reach'})
 path=HERE/'native-unused-fields.json'
 if create:
  with path.open('x')as f:json.dump(doc,f,indent=1);f.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(cases=len(rows),fresh_executions=len(rows)*2,pins=len(pins),sha256=sha(path.read_bytes()),immutable=not create)))
if __name__=='__main__':main()
