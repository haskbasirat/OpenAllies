"""Actual runtime-bound tutorial TERRAIN7AA9B0 plus7ABC30 at all3PC/bothPE.

The captured actual source Bank/terrain supplies static flags and the typed
depth witness; stream/version transport and allocation are named seams.
This does not run complete7A0D40 or substitute any AI player configuration.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-navigation'))
import topology_oracle as old
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
LEAVES={0x8589b0,0x6f7810,0x6f7430}
def run(c,image):
 data=c['input'];prior=old.base.P;old.base.P=old.original if image=='original'else old.modified
 try:m=old.Machine(bytes.fromhex(data['suffix_hex']),data['layout'],data['flags20'],c['precision'])
 finally:old.base.P=prior
 pins={};leaves={}
 def code(u,pc,size,user):
  if old.original.base<=pc<old.original.base+old.original.image_size:
   raw=bytes(u.mem_read(pc,size));assert raw==old.original.range(pc,size)==old.modified.range(pc,size)
   (leaves if pc in LEAVES else pins)[pc]=raw.hex()
 def guard(u,access,address,size,value,user):
  if address<0x1000:
   pc=u.reg_read(UC_X86_REG_EIP);assert bytes(u.mem_read(pc,1))==b'\x64',('nonFS null access',hex(pc),address)
 m.u.hook_add(UC_HOOK_CODE,code);m.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,guard)
 m.call(0x7aa9b0,m.graph,[m.stream-0x3c,m.grid]);consumed=m.r.at
 primary=m.read(m.graph+0x2c);primary_island=m.read(primary)if primary else None
 diagonal=m.read(m.graph+0x30)
 setting=m.alloc(4);m.write(setting,data['depth']);m.write(0xb40e44,setting);m.write(0xb40e48,1)
 m.call(0x7abc30,m.graph);count=m.read(m.graph+0x18);areas=[];flags=[];neighbors=[]
 for i in range(count):
  node=m.read(m.read(m.graph+0x14)+4*i);areas.append(m.read(node+12));flags.append(bool(m.u.mem_read(node+0x6c,1)[0]))
  neighbors.append([m.read(m.read(m.read(node+0x74)+4*j))for j in range(m.read(node+0x70))])
 cache=m.read(m.graph+0x40);rows=[]
 for i in range(m.read(cache)):
  row=m.read(cache+4)+8*i;rows.append(list(m.u.mem_read(m.read(row+4),m.read(row))))
 output=dict(consumed=consumed,prepared=bool(m.u.mem_read(m.graph+5,1)[0]),areas=areas,bottlenecks=flags,
  neighbors=neighbors,route_cache=dict(rows=rows),primary_island=primary_island,diagonal_bits=diagonal)
 return output,pins,leaves
def main():
 source=HERE/'actual-map-input.json';capture=json.loads(source.read_text());rows=[];pins={};leaves={}
 for c in capture['cases']:
  a,ap,al=run(c,'original');b,bp,bl=run(c,'v14');assert a==b==c['output'],(c['precision'],a,c['output'])
  pins.update(ap|bp);leaves.update(al|bl);rows.append(dict(precision=c['precision'],input=c['input'],output=a))
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),actual_capture_sha256=H(source.read_bytes()),rows=rows,machines=2*len(rows),
  original_sha256=H(old.original.b),v14_sha256=H(old.modified.b),
  executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],intercepted_entries=[dict(va=hex(a),bytes=b)for a,b in sorted(leaves.items())],
  stream_seam='0x2e00000 transport',resolved_setting='typed depth cache pointer supplied; nativeSVarconstructor notrerun',
  dependencies=[dict(path=p,sha256=H((ROOT/p).read_bytes()))for p in ['research/phase7-navigation/topology_oracle.py','research/phase6-mapheader/native_header.py']])
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-actual-map.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),executed_pins=len(pins),intercepted_entries=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
