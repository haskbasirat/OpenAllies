"""Unchanged actual-source4D6BB0 plus identity/scale4E9FC0, and4C2BC0.

Only CString allocator/lifetime, heap and profiler transports are seams.
Explicit direct unowned constructor probes are not initialized map creation.
The direction table startupPC is a declared probe input, not inferred from
definition/source completionPC. No component/InitCreate success seam exists.
"""
from pathlib import Path
import json,hashlib,sys,copy,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-actor-creation'))
from actor_machine import ActorMachine,DIRECTIONS,PCS
from native_actor_creation import make_definition,make_params
import actor_machine as am
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
# actor_machine already imported another native_machine; use its own PE domain.
P,V=am.P,am.V
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_FPCW
H=lambda b:hashlib.sha256(b).hexdigest()
def image(m,which):
 if which=='v14':
  old=(P.image_size+4095)&~4095;new=(V.image_size+4095)&~4095
  if new>old:m.u.mem_map(P.base+old,new-old)
  for vs,rva,raw,off in V.sections:m.u.mem_write(V.base+rva,V.b[off:off+raw])
def one(row,which):
 c=row['input'];m=ActorMachine(row['precision']);image(m,which)
 m.w(0xb36534,m.gworld);m.w(0xb36530,m.gamedata);m.w(0xb364c0,m.idmgr)
 m.w(m.gworld+0xe0,c['clock']);m.w(m.gworld+0xd4,c['seed']);m.w(m.gworld+0xd8,6)
 scene=m.alloc(0x30);m.w(m.gworld+0x130,scene)
 table=m.alloc(2048*8);i=PCS[c['direction_table_precision']]
 m.u.mem_write(table,DIRECTIONS[i*2048*8:(i+1)*2048*8]);obj=m.alloc(16)
 m.w(obj+4,2048);m.w(obj+8,table);m.w(0xb39af8,obj)
 d=make_definition(m,c['id'],dict(flags=c['flags'],mask=0,scale=c['scale'],variations=int(c['variations']),aspects=c['weights'],total_weight=c['total_weight']))
 #The actual bounded probe supplies no owner/required object or nonblocking.
 assert c['owner']==0 and c['required']==0
 m.owner=0;p=make_params(m,dict(position=c['position'],direction=c['direction'],variant=c['variant'],owner=False),d)
 a=m.alloc(0x180,0xa5)
 m.call(0x4d6bb0,a,[p],4);assert m.outcome is None,m.outcome
 def snapshot(initial):
  return dict(flags=m.r(a+8),position=[m.r(a+0x20+4*i)for i in range(3)],facing=[m.r(a+0x2c+4*i)for i in range(3)],time=m.r(a+0x1c),word_c=m.r(a+0xc),word_10=m.r(a+0x10),name=m.cstring_units(m.r(a+0x18)),scale=m.r(a+0x3c),variant=struct.unpack('<i',struct.pack('<I',m.r(a+0x40)))[0],byte4c=m.rb(a+0x4c),words50=[m.r(a+0x50),m.r(a+0x54)],owner=m.r(a+0xe8),required=m.r(a+0x158),state=[m.r(a+0x100),m.r(a+0x104)],actions=[m.r(a+0xf0),m.r(a+0xf4)],components=sum(m.r(a+0x60+4*i)!=0 for i in range(31)),list108=m.r(a+0x108)!=0,zones=m.r(a+0x14c),id=None if initial else m.r(a+0x14),params_direction=[m.r(p+0x14),m.r(p+0x18)],params_owner=m.r(p+0x28),rng=m.r(m.gworld+0xd4))
 before=snapshot(True);m.call(0x4e9fc0,a,[p],4);assert m.outcome is None,m.outcome
 assert m.u.reg_read(UC_X86_REG_EAX)&255==1
 after=snapshot(False)
 return dict(precision=row['precision'],input=c,before=before,after=after)
def main():
 doc=json.loads((HERE/'actual-constructor-inputs.json').read_text());rows=[]
 for row in doc['rows']:
  a=one(row,'original');b=one(row,'v14');assert a==b
  assert a==row,(row['input']['id'],row['precision'],a,row)
  rows.append(a)
 #Every visited instruction came from both unchanged images (enforced by
 #the imported harness). Static read-only data are the actual loaded source.
 pins=[dict(va=hex(a),bytes=b)for a,b in sorted(am.VISITED.items())]
 out=dict(schema=1,scope=__doc__,rows=rows,instructions=pins,original_sha256=H(P.b),v14_sha256=H(V.b),producer_sha256=H(Path(__file__).read_bytes()))
 path=HERE/'native-constructor.json';raw=(json.dumps(out,indent=2,sort_keys=True)+'\n').encode()
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw,'immutable constructor fixture differs'
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,instructions=len(pins),sha256=H(raw))))
if __name__=='__main__':main()
