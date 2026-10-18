"""Native list construction/zero-count source load retained-head proof."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
import capstone
O,V=h.P,h.V
rows=[]
for label,image in [('original',O),('v14',V)]:
 h.P=image
 for pc in h.PRECISION:
  m=h.Machine(pc);p=m.alloc(8);m.w32(p,0xaaaaaaaa,0xbbbbbbbb)
  m.call(0x79e130,p);assert m.r32(p,2)==[0,0] and m.final_esp==h.STACK+4
  calls=[]
  def count(m):calls.append('5B4220:count0');m.ret(4,value=0)
  m.seams[0x5b4220]=count
  m.call(0x42bf10,0,[0,0,p]);assert m.r32(p,2)==[0,0] and m.final_esp==h.STACK+16
  names=[]
  def enumeration(m):
   key=m.arg(1);text=[]
   while m.r32(key)&65535:text.append(m.r32(key)&65535);key+=2
   names.append({'index':m.arg(),'name':''.join(chr(v)for v in text),'offset':hex(0xbc+8*m.arg())});m.ret(12)
  m.seams[0x4976b0]=enumeration
  m.call(0x4979d0,p);assert m.final_esp==h.STACK+4
  assert [n['name']for n in names]==['WhenCreated','WhenDestroyed','WhenSelected','WhenCommanded','WhenCommandedAggressive','WhenConstructed']
  actor=m.alloc(0x80);m.w32(actor+0x20,0x3f800000,0x40000000,0x40400000);m.w32(0xb36534,0)
  m.call(0x42a590,p,[actor,1]);assert m.final_esp==h.STACK+12 and m.fpu_top()==0
  rows.append({'image':label,'precision':pc,'constructed_head_tail':m.r32(p,2),'source_count_calls':calls,'skipped_FXC_load':True,'enum_lists':names,'actual42A590_429080_empty_return':True})
h.P=O
pins=[]
for va,b in sorted(h.VISITED.items()):
 raw=bytes.fromhex(b);assert O.range(va,len(raw))==V.range(va,len(raw))==raw;pins.append({'va':hex(va),'bytes':b})
md=capstone.Cs(capstone.CS_ARCH_X86,capstone.CS_MODE_32)
static=[]
for va,n in [(0x49c760,0x180),(0x4979d0,0xa0)]:
 for ins in md.disasm(O.range(va,n),va):
  assert O.range(ins.address,ins.size)==V.range(ins.address,ins.size)
  static.append({'va':hex(ins.address),'bytes':ins.bytes.hex(),'instruction':ins.mnemonic+' '+ins.op_str})
  if ins.mnemonic.startswith('ret'):break
out={'scope':'Constructor79E130 actual zero head/tail plus42BF10 count0 source no-write path; actual4979D0 six-name enum calls and42A590->429080 empty return with null world. Static49C760 six-list array base+BC. Single source declaration required by runtime binding; shared FX reload/appends are not certified.','cases':rows,'instructions':pins,'static_instructions':static,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
p=HERE/'native-empty-fx-source.json';p.write_text(json.dumps(out,indent=2)+'\n');print('cases',len(rows),'pins',len(pins),'sha256',hashlib.sha256(p.read_bytes()).hexdigest())
