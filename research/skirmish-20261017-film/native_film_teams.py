"""Native50F890 constructor and510EB0..511020 full Team phase. Real
50D6F0/50E790/4BEDF0/CString/510250 including vector growth. Stop at511020
BEFORE Kingdom count reset; no fake success for the unported Kingdom phase.
"""
from pathlib import Path
import hashlib,json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Complete(Exception):pass
class Stopped(Exception):pass
def cstring(m,words):
 d=m._data(len(words));m.w(d+4,len(words));m.w(d+12,0x10000);m.u.mem_write(d+16,struct.pack('<'+'H'*len(words),*words)+b'\0\0');return d+16
def read(m,p):
 n=m.r(p-12);return list(struct.unpack('<'+'H'*n,bytes(m.u.mem_read(p,n*2))))if n else[]
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
 data=m.alloc(0x100);m.w(0xb36530,data);m.w(data+0x38,3);m.w(0xb43b48,cstring(m,[]))
 target=m.alloc(0x30);m.call(0x50f890,target);assert m.r(target+8)==0 and m.r(target+0x14)==0
 #Native active Kingdom count must remain unchanged throughout this slice.
 m.w(target+0x14,7);m.w(target+8,1)
 world=m.alloc(0x300);slots=m.alloc(4*max(1,len(case['slots'])));m.w(world+0x16c,slots);m.w(world+0x170,len(case['slots']))
 for i,row in enumerate(case['slots']):
  team=0
  if row is not None:
   team=m.alloc(0x70)
   for offset,key in [(0x18,'id'),(0x3c,'key'),(0x40,'text')]:m.w(team+offset,cstring(m,row[key]))
  m.w(slots+4*i,team)
 events=[];step=0
 def event(name):
  nonlocal step
  if step==case.get('stop',-1):raise Stopped(name)
  step+=1;events.append(name)
 def observe(u,at,size,_):
  if at==0x511020:raise Complete()
  if at in {0x50d6f0,0x4bedf0,0x510f6d,0x510f7a}:event({0x50d6f0:'slots',0x4bedf0:'id',0x510f6d:'key',0x510f7a:'text'}[at])
 m.u.hook_add(UC_HOOK_CODE,observe)
 stopped=None;complete=False
 try:m.call(0x510eb0,target,[world])
 except Complete:complete=True
 except Stopped as e:stopped=str(e)
 out=[]
 for i in range(m.r(target+8)):
  p=m.r(target+4)+12*i;out.append({key:read(m,m.r(p+offset))for offset,key in [(0,'id'),(4,'key'),(8,'text')]})
 return dict(events=events,stopped=stopped,completed_team_phase=complete,kingdom_count=m.r(target+0x14),teams=out),m.windows()
def row(i):return dict(id=[84,i],key=[107,i],text=[0,0xd800,i,0xffff])
def cases():
 for slots in [[],[None],[row(0)],[None,row(1),None,row(2),None],[row(i)for i in range(20)]]:yield dict(slots=slots)
 base=dict(slots=[None,row(1),None,row(2)]);n=len(run(base,'original','Double53')[0]['events'])
 for stop in range(n):yield dict(base,stop=stop)
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 doc=dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-teams.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
