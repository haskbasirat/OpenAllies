"""5A26E0 current-header coordinator. Real CString assignment/clear; FILETIME,
41D130/510EB0/Campaign manager/5A17D0 are mandatory explicit service seams.
Preinstruction source-read stops preserve partial stores, not SEH unwinding.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stopped(Exception):pass
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
 header=m.alloc(0x70);world=m.alloc(0x300);rules=m.alloc(0x20);parameters=m.alloc(0x30)
 current=m.alloc(0x20);lookup=m.alloc(0x20);manager=m.alloc(0x100)
 m.w(header+4,rules);m.w(header+8,parameters)
 for offset in [0x18,0x1c,0x30,0x34,0x40]:m.w(header+offset,0xcccccccc)
 m.w(header+0x3c,m.cstring('old campaign'));m.w(header+0x38,m.cstring('unchanged original id'))
 for offset in [4,8,0x18]:m.w(rules+offset,0xcccccccc)
 m.w(world+0x10,0x11223344);m.w(world+0x14,0x55667788);m.w(world+0x24,0x12345678)
 m.w(world+0x3c,0x80000000);m.w(world+0x40,0x7fc01234)
 m.w(current+8,m.cstring('CurrentCampaign'));m.w(lookup+0x14,0xaabbccdd)
 m.w(0xb36534,world);m.w(0xb39d04,manager)
 events=[];step=0;parameter_marker=0;entries_marker=0
 def event(name,**words):
  nonlocal step
  if step==case.get('stop',-1):raise Stopped(name)
  events.append(dict(name=name,**words));step+=1
 def time():
  event('time');m.u.reg_write(UC_X86_REG_EDX,0x11223344);m.ret(0,0x55667788)
 def copy():
  assert m.u.reg_read(UC_X86_REG_ECX)==rules+12 and m.arg(0)==world+24
  event('copy_rules');m.w(rules+12,0xabc);m.w(world+0x24,0xdeadbeef);m.ret(4)
 def params():
  nonlocal parameter_marker
  assert m.u.reg_read(UC_X86_REG_ECX)==parameters and m.arg(0)==world
  event('parameters');parameter_marker=0x789;m.w(world+0x3c,0x7f800001);m.w(world+0x40,0xff800000);m.ret(4)
 def get_current():
  assert m.u.reg_read(UC_X86_REG_ECX)==world;event('current_campaign');m.ret(0,current if case['current']else 0)
 def get_lookup():
  assert m.u.reg_read(UC_X86_REG_ECX)==0xb39690 and m.arg(0)==manager+0x50
  create=m.arg(1)&255;assert create==int(case['current']);event('lookup_campaign',create=bool(create));m.ret(8,lookup if case['lookup']else 0)
 def entries():
  nonlocal entries_marker
  assert m.u.reg_read(UC_X86_REG_ECX)==header;event('entries');entries_marker=0x456;m.ret(0)
 for at,f in [(0x6fb1f0,time),(0x41d130,copy),(0x510eb0,params),(0x4f1310,get_current),(0x411790,get_lookup),(0x5a17d0,entries)]:m.seam(at,f)
 reads={0x5a26fd:'mode',0x5a2703:'game_type',0x5a2715:'rule_extra',0x5a2724:'width',0x5a272a:'height',0x5a2761:'campaign_name',0x5a276d:'campaign_index'}
 def observe(u,at,size,_):
  if at in reads:event(reads[at])
 m.u.hook_add(UC_HOOK_CODE,observe)
 stopped=None
 try:m.call(0x5a26e0,header,[world])
 except Stopped as e:stopped=str(e)
 if stopped is None:
  assert all(m.u.reg_read(r)==0x5a5a0000 for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])
  assert(m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 out=dict(events=events,stopped=stopped,time=[m.r(header+0x18),m.r(header+0x1c)],mode=m.r(rules+4),game_type=m.r(rules+8),rule_list=m.r(rules+12),rule_extra=m.r(rules+0x18),parameters=parameter_marker,extents=[m.r(header+0x30),m.r(header+0x34)],campaign=m.text(m.r(header+0x3c)),index=m.r(header+0x40),entries=entries_marker,original_id=m.text(m.r(header+0x38)))
 return out,m.windows()
def cases():
 for current in [False,True]:
  for lookup in [False,True]:yield dict(current=current,lookup=lookup)
 base=dict(current=True,lookup=True);n=len(run(base,'original','Double53')[0]['events'])
 for stop in range(n):yield dict(base,stop=stop)
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 doc=dict(schema=1,scope=__doc__,original_sha256=L.pe.ORIGINAL_SHA,v14_sha256=L.pe.ACCEPTED_SHA,cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-capture.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
