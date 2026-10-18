"""Fresh paired full7B4830 and deliberate pre7BB1C0 prefix cutoffs.

Complete18 frozen synthetic source graphs at all3PC, actual constructors and
goal publication/filtering/neighbors. Three prefix cases stop at the first
neighbor walk ENTRY; nine more stop before exploration, recruitment, or the
Kingdom RecruitSingle count read. They establish retained publication state,
not a native exception, throw, missing-source acceptance, or completed startup.
Only inherited allocation/stream/version and name/log leaves are intercepted.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-sai'));import ai_oracle as old
import topology_oracle as topo
from controller_oracle import list_values
from unicorn import UC_HOOK_CODE,UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP
H=lambda b:hashlib.sha256(b).hexdigest();O=old.original;V=old.modified
LEAVES={0x6f7810,0x6f7430,0x8589b0,0x4bedf0,0x404f60}
class Cutoff(Exception):pass
def partial(m):
 controller=m.read(m.player+12);pending=list_values(m,m.read(controller+12))
 return dict(ready=bool(m.u.mem_read(m.player+4,1)[0]),controller_present=bool(controller),
  script_words=[m.read(m.read(m.player+0x144)+4*i)for i in range(m.read(m.player+0x148))],
  words10_14=[m.read(m.player+16),m.read(m.player+20)],
  region_count=m.read(m.player+0x24),pending_kinds=[old.VT[m.read(p)]for p in pending],
  group_counts=[[len(list_values(m,m.read(m.read(m.read(controller+8)+4*i)+8*j)))for j in range(3)]for i in range(23)])
def run(c,pc,image,cutoff):
 prior=topo.base.P;topo.base.P=O if image=='original'else V
 try:m=old.Machine(bytes.fromhex(c['input_hex']),c['layout'],[0x200]*64,pc)
 finally:topo.base.P=prior
 m.setup(c);pins={};leaves={};stopped=False
 def guard(u,access,address,size,value,user):
  if address<0x1000:
   ip=u.reg_read(UC_X86_REG_EIP);assert bytes(u.mem_read(ip,1))==b'\x64',('nonFS null',hex(ip),address)
 def code(u,ip,size,user):
  stop={True:0x7bb1c0,'ability':0x7a4970,'recruitment':0x7a4a60,'owned_units':0x7b4a79}.get(cutoff)
  if stop and ip==stop:raise Cutoff()
  if O.base<=ip<O.base+O.image_size:
   raw=bytes(u.mem_read(ip,size));assert raw==O.range(ip,size)==V.range(ip,size)
   (leaves if ip in LEAVES else pins)[ip]=raw.hex()
 m.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,guard);m.u.hook_add(UC_HOOK_CODE,code)
 try:m.call(0x7b4830,m.player)
 except Cutoff:stopped=True
 assert stopped==bool(cutoff)
 return (partial(m)if isinstance(cutoff,str)else m.summarize(c)),pins,leaves
def main():
 fixtures=json.loads((ROOT/'research/phase7-sai/native-ai.json').read_text())['fixtures'];rows=[];prefix=[];source_prefix=[];pins={};leaves={}
 for pc in ['Single24','Double53','Extended64']:
  for row in fixtures:
   c=row['input'];a,ap,al=run(c,pc,'original',False);b,bp,bl=run(c,pc,'v14',False);assert a==b==row['native']
   pins.update(ap|bp);leaves.update(al|bl);rows.append(dict(precision=pc,input=c,output=a))
  c=fixtures[2]['input'];assert c['neighbor_depth']>0
  a,ap,al=run(c,pc,'original',True);b,bp,bl=run(c,pc,'v14',True);assert a==b
  pins.update(ap|bp);leaves.update(al|bl);prefix.append(dict(precision=pc,input=c,output=a,cutoff='before first7BB1C0 entry instruction'))
  for site in ['ability','recruitment','owned_units']:
   a,ap,al=run(c,pc,'original',site);b,bp,bl=run(c,pc,'v14',site);assert a==b
   pins.update(ap|bp);leaves.update(al|bl);source_prefix.append(dict(precision=pc,input=c,output=a,cutoff=site))
 deps=['research/phase7-sai/ai_oracle.py','research/phase7-sai/state_oracle.py','research/phase7-sai/controller_oracle.py',
  'research/phase7-navigation/topology_oracle.py','research/phase6-mapheader/native_header.py','research/phase7-sai/native-ai.json']
 doc=dict(scope=__doc__,producer_sha256=H(Path(__file__).read_bytes()),original_sha256=H(O.b),v14_sha256=H(V.b),
  rows=rows,prefix_rows=prefix,source_prefix_rows=source_prefix,machines=2*(len(rows)+len(prefix)+len(source_prefix)),executed_pins=[dict(va=hex(a),bytes=b)for a,b in sorted(pins.items())],
  intercepted_entries=[dict(va=hex(a),bytes=b)for a,b in sorted(leaves.items())],
  dependencies=[dict(path=p,sha256=H((ROOT/p).read_bytes()))for p in deps])
 raw=(json.dumps(doc,indent=1)+'\n').encode();out=HERE/'native-ai-startup.json'
 if '--write'in sys.argv:out.write_bytes(raw)
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),prefix_rows=len(prefix),source_prefix_rows=len(source_prefix),machines=2*(len(rows)+len(prefix)+len(source_prefix)),executed_pins=len(pins),intercepted_entries=len(leaves),sha256=H(raw),immutable='--write'not in sys.argv)))
if __name__=='__main__':main()
