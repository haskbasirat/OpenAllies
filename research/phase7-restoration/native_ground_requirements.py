"""Whole original4CFB00 for all2048 subsets of the11 supported concrete ground component vtables. Actual virtual+20 constant leaves execute; missing-requirement logging is the only diagnostic seam."""
from pathlib import Path
import sys,json,hashlib
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'research/phase6-kingdoms'))
from native_kingdoms import Machine,P,V,VISITED
from unicorn.x86_const import *
source_path=ROOT/'research/phase7-actors/component-dispatch.json'
source=json.load(open(source_path));slots=[0,2,3,6,7,8,9,10,15,17,22];tables={r['slot']:int(r['vtable'],16)for r in source['rows']if r['slot']in slots};rows=[]
for subset in range(1<<len(slots)):
 m=Machine(b'',28);mask=0
 for i,slot in enumerate(slots):
  if subset&(1<<i):p=m.alloc(0x100);m.write(p,tables[slot]);m.write(m.object+0x60+slot*4,p);mask|=1<<slot
 m.write(m.stack,m.stop);m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ECX,m.object);m.u.emu_start(0x4cfb00,m.stop,count=10000)
 assert m.u.reg_read(UC_X86_REG_EIP)==m.stop and m.u.reg_read(UC_X86_REG_ESP)==m.stack+4
 rows.append({'mask':mask,'valid':bool(m.u.reg_read(UC_X86_REG_EAX)&255)})
table_pins=[]
for slot,address in tables.items():
 raw=P.range(address+0x20,4);assert raw==V.range(address+0x20,4);table_pins.append({'slot':slot,'va':hex(address+0x20),'bytes':raw.hex()})
p=Path(__file__).with_name('native-ground-requirements.json');p.write_text(json.dumps({'scope':__doc__,'constructor_dispatch_source_sha256':hashlib.sha256(source_path.read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'cases':rows,'vtable_pins':table_pins,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(VISITED.items())]},indent=2)+'\n');print({'cases':len(rows),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
