"""4CF0F0 presence thunk and real570020 null-owner early return.
Original/v14 separately execute at all PCs. Owner virtual+108 is explicit;
entry-stop cases mark570020 required rather than inventing initialized flows.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from harness import UC_HOOK_CODE,UC_X86_REG_ECX
ORIGINAL=h.P
class Dependency(Exception):pass
def run(c,image,pc):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(pc);actor=m.alloc(0x200);economy=m.alloc(0x80);vt=m.alloc(0x200);events=[]
 m.w32(actor,vt);m.w32(actor+0x84,economy if c['present'] else 0);m.w32(economy+4,actor)
 before=list(m.u.mem_read(economy,0x80))
 def owner(_):events.append(['owner_108']);assert m.ecx()==actor;m.ret(0,0)
 stub=m.stub_next;m.stub_next+=16;m.seams[stub]=owner;m.w32(vt+0x108,stub)
 def observe(u,pc,size,_):
  if pc==0x4cf0f0:events.append(['economy_presence',c['present']]);assert m.ecx()==actor
  elif pc==0x570020:
   events.append(['570020']);assert m.ecx()==economy
   if c['stop']:raise Dependency('570020')
 m.u.hook_add(UC_HOOK_CODE,observe)
 outcome='ok'
 try:m.call(0x4cf0f0,actor)
 except Dependency as e:outcome=str(e)
 assert before==list(m.u.mem_read(economy,0x80))
 return {'outcome':outcome,'events':events,'economy_unchanged':True}
def main():
 rows=[]
 for pc in h.PRECISION:
  for c in [{'label':'absent','present':False,'stop':False},{'label':'present_null_owner','present':True,'stop':False},{'label':'present_required_entry','present':True,'stop':True}]:
   a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b;rows.append({'input':c,'precision':pc,'output':a})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'rows':len(rows),'native_executions':len(rows)*2,'cases':rows,'instructions':[{'va':hex(a),'bytes':b} for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-owner-economy.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'native_executions':len(rows)*2,'instruction_pins':len(h.VISITED),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
