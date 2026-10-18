"""Actual46BD40 Audio category manager constructor and497220 named lookup.
Heap/counted CString transports inherited. No caller-created enum table.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPCW
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set()
 def capture(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,capture)
 manager=m.alloc(0x24);m.thiscall(0x46bd40,manager,[1]);rows=[]
 for name in ['SOUND','sound','MUSIC','Speech','unknown','']:
  slot=m.alloc(4);m.w(slot,m.cstring(name));node=m.thiscall(0x497220,manager+0x10,[slot])['eax']
  rows.append(dict(name=name,value=m.r(m.r(node))if node else None))
 table=[dict(value=m.r(m.r(m.r(manager+4)+i*4)),name=m.wstr(m.r(m.r(m.r(manager+4)+i*4)+4)))for i in range(m.r(manager+8))]
 return dict(table=table,lookup=rows),pins,seams
def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,native=a));pins.update(pa);seams|=sa|sb
 for address,raw in pins.items():assert h.P.range(address,len(bytes.fromhex(raw)))==h.V.range(address,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
 doc=dict(scope=__doc__,rows=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 path=HERE/'native-audio-category.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(machines=6,category_lookup_calls=36,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
