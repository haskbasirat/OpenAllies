"""Validate original ordinary Actor vtable binding used by runtime adapter.
This executes simple shape/B0/B4 virtuals, not whole actor construction.
Constructor vtable assignment is pinned static evidence.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
vt=0xa273f0;rows=[]
for override in [0,1,2,255]:
 for slot in [0xc,0x10,0x14,0xb0,0xb4]:
  m=Machine();m.reset(0);definition=m.STATE+0x1000;m.w(m.STATE+4,definition);m.w(definition+0x80,override)
  entry=struct.unpack('<I',original.range(vt+slot,4))[0];m.u.emu_start(entry,m.STOP,count=32)
  assert m.u.reg_read(UC_X86_REG_EIP)==m.STOP and m.u.reg_read(UC_X86_REG_ESP)==m.STACK+4
  result=m.u.reg_read(UC_X86_REG_EAX)
  if slot<0x20:
   result-=definition;assert result==({0xc:0x38,0x10:0x50,0x14:0x68 if override else 0x50}[slot])
  else:result=bool(result&255);assert result
  rows.append({'target_override':override,'slot':hex(slot),'entry':hex(entry),'result':result})
ranges={}
for name,a,n in [('load_constructor_vtable',0x4d6dbd,6),('actor_vtable',vt,0xcc),('blocking_shape',0x4ea010,7),('soft_shape',0x4ea020,7),('target_shape',0x4ea030,21),('actor_categories',0x6a71d0,3)]:
 b=original.range(a,n);assert b==modified.range(a,n);ranges[name]={'va':hex(a),'length':n,'sha256':sha(b),'bytes':b.hex()}
doc={'schema':1,'scope':__doc__,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(original.b),'accepted_sha256':sha(modified.b),'cases':rows,'ranges':ranges}
(HERE/'native-binding-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
