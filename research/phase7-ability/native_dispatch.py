"""Original57E030→4C9E90→4C9550 delivery with real DamageEffect/Body.
Verifies source impulse, active-start timestamp and unconditional post-use mana debit.
World leaves are inherited explicitly from native_execution.
"""
from native_execution import *
PERF=0x20c0000
class Machine(Machine):
 def __init__(self,c):
  super().__init__(c);self.w(PERF+0x20,c['mana']);self.w(PERF+0x34,body.WEAPON if c['active_ability'] else 0);self.w(PERF+0x3c,c['started_at']);self.w(body.WEAPON_DATA+0x40,c['mana_cost'])
 def run(self):
  self.w(self.STACK,self.STOP);self.w(self.STACK+4,body.WEAPON);self.w(self.STACK+8,weapon.CONTEXT);self.u.reg_write(UC_X86_REG_ECX,PERF);self.u.reg_write(UC_X86_REG_ESP,self.STACK)
  regs={r:0x89ab0000+i for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
  for r,v in regs.items():self.u.reg_write(r,v)
  try:self.u.emu_start(0x57e030,self.STOP,count=2000000)
  except Exception as e:raise RuntimeError(hex(self.u.reg_read(UC_X86_REG_EIP)))from e
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+12
  assert all(self.u.reg_read(r)==v for r,v in regs.items())
  return{'mana':self.r(PERF+0x20),'context':self.context(),'use_count':self.r(body.WEAPON+0xc),'last_use':self.r(body.WEAPON+0x10),'actions':self.action_events,'requests':self.deliveries,'body':self.state(),'environment':self.env,'events':self.events}
def cases():
 from native_execution import cases as original_cases
 rows=[]
 for i,c in enumerate(original_cases()):
  if c['ability']['definition']==0:continue
  c.update(mana=bits([.3,30,-3][i%3]),mana_cost=bits([.3333333,2.77,0,-.5][i%4]),active_ability=i%2==0,started_at=bits(3.17))
  if i%2==0:c['actors']['1']['definition_flags168']|=2;c['definition']['impact_impulse']=bits(.3717)
  rows.append(c)
 return rows
def main():
 rows=[];visited={}
 for c in cases():
  try:m=Machine(c);out=m.run()
  except Exception as e:raise RuntimeError(c['name'])from e
  rows.append({'input':c,'output':out});visited.update(m.executed)
 doc={'schema':1,'passed':True,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':body.pe.ORIGINAL_SHA,'accepted_sha256':body.pe.ACCEPTED_SHA,'method':__doc__,'cases':rows,'visited_windows':[{'address':hex(a),'bytes':body.IMAGE.at(a,size).hex()}for a,size in sorted(visited.items())]}
 (HERE/'native-dispatch-fixtures.json').write_text(json.dumps(doc,indent=2)+'\n',encoding='utf8');print(json.dumps({'passed':True,'cases':len(rows)}))
if __name__=='__main__':main()
