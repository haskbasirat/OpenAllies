"""Unchanged567CD0/4C8650/4BE3F0 on actual completed Defender source powers.

Declared leaves are heap allocation/free only. Native initial capacity3 is an
allocation policy, not a game rule; the portable same-instance vector grows.
These are explicit direct constructors on a fresh laboratory Actor, not a
completed map Assembly/Init or naturally initialized Defender claim.
"""
from pathlib import Path
import json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn import UC_HOOK_MEM_READ
def one(row,image):
 c=row['input'];m=nm.Machine(row['precision'])
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 def malloc(mm):q=mm.alloc(mm.arg(1));mm.u.mem_write(q,b'\xcd'*mm.arg(1));mm.ret(q)
 m.seam(0x6f7810,malloc);m.seam(0x6f7430,lambda mm:mm.ret())
 actor=m.alloc(0x180);data=m.alloc(0x600);component=m.alloc(0x34)
 m.u.mem_write(actor,b'\xcd'*0x180);m.u.mem_write(data,b'\xcd'*0x600);m.u.mem_write(component,b'\xcd'*0x34)
 m.w(actor+4,data);locals=[];definitions=[];bounds=[(actor,0x180,{4,5,6,7}),(data,0x600,set(range(0x21c,0x224)))]
 for power in c['powers']:
  local=m.alloc(0x80);definition=m.alloc(0x100)
  m.u.mem_write(local,b'\xcd'*0x80);m.u.mem_write(definition,b'\xcd'*0x100)
  m.w(local,definition);m.w(definition+0x64,power);locals.append(local);definitions.append(definition)
  bounds += [(local,0x80,set(range(4))),(definition,0x100,set(range(0x64,0x68)))]
 m.w(data+0x21c,m.array(locals));m.w(data+0x220,len(locals))
 def guard(u,access,address,size,value,user):
  for base,length,allowed in bounds:
   if base<=address<base+length:assert all(a-base in allowed for a in range(address,address+size)),(hex(u.reg_read(nm.UC_X86_REG_EIP)),hex(address-base),size)
 m.u.hook_add(UC_HOOK_MEM_READ,guard)
 m.call(0x567cd0,component,[actor]);assert m.r(component+4)==actor
 abilities=[]
 for index in range(m.r(component+0x14)):
  a=m.r(m.r(component+0x10)+4*index);d=definitions.index(m.r(a+4));local=locals.index(m.r(a+8));assert d==local==index
  assert m.r(a+0x24)==0
  abilities.append(dict(handle=index+1,definition=c['definitions'][d],local=c['locals'][local],use_count=m.r(a+0xc),last_use=m.r(a+0x10),word14=m.r(a+0x14),enabled=bool(m.rb(a+0x18)),recharge_bypass=bool(m.rb(a+0x19)),fuzzy_range=m.r(a+0x1c),power=m.r(a+0x20),summoned=[],word28=m.r(a+0x28)))
 assert m.r(component+0xc)==0 and m.r(component+0x24)==0 and m.r(component+0x28)==0
 state=dict(owner=c['owner'],scheduled=None,abilities=abilities,word1c=m.r(component+0x1c),word20=m.r(component+0x20),entries=[],word2c=m.r(component+0x2c),byte30=m.rb(component+0x30))
 return dict(precision=row['precision'],input=c,state=state),m.visited
def main():
 doc=json.loads((HERE/'actual-constructor-inputs.json').read_text());rows=[];pins={}
 for row in doc['defenders']:
  a,va=one(row,'original');b,vb=one(row,'v14');assert a==b==row,(a,b,row)
  for address,raw in va.items():assert raw==nm.original.range(address,len(raw))==nm.modified.range(address,len(raw));pins[address]=raw
  assert va==vb;rows.append(a)
 assert rows,'actual Defender source rows required'
 out=dict(schema=1,scope=__doc__,rows=rows,instructions=nm.instructions(pins),producer_sha256=nm.sha(Path(__file__).read_bytes()))
 raw=(json.dumps(out,indent=2,sort_keys=True)+'\n').encode();path=HERE/'native-defender.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw,'immutable Defender fixture differs'
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,instructions=len(pins),sha256=nm.sha(raw))))
if __name__=='__main__':main()
