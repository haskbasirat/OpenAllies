"""Full5A17D0 with real765E00/4BEDF0/50DC00/79EA90/766380/5A1360 and
CString assignments. No successful getter/AI-profile/entry-append seams.
Source fields are explicit fixture storage; preinstruction stop tests qualify
partial effects without claiming native C++ exception unwinding.
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
 header=m.alloc(0x70);params=m.alloc(0x30);session=m.alloc(0x200);node=m.alloc(0x300);manager=m.alloc(0x100)
 m.w(header+8,params);m.w(0xb36550,session);m.w(0xb36558,node);m.w(0xb36528,manager);m.w(0xb36534,header if case['world']else 0)
 players=[];links=[];kingdoms=[];param_rows=[];ai_rows=[]
 for i,row in enumerate(case['players']):
  p=m.alloc(0x60);players.append(p);link=m.alloc(8);links.append(link)
  m.w(p+8,m.cstring(row['display']));m.u.mem_write(p+12,bytes([row['ai']&255]));m.w(p+0x24,m.cstring(row['name']))
  kingdom=0
  if row['kingdom'] is not None:
   kingdom=m.alloc(0x300);m.w(kingdom+0x18,m.cstring(row['kingdom']));kingdoms.append((kingdom,row['kingdom']))
   if row['parameter']:
    param_rows.append((row['kingdom'],'old_profile'))
   for profile in row['profiles']:
    ai=m.alloc(0x30);m.w(ai+8,kingdom);definition=0
    if profile is not None:definition=m.alloc(0x30);m.w(definition+8,m.cstring(profile))
    m.w(ai+0x18,definition);ai_rows.extend([0,ai])
  m.w(p+0x28,kingdom)
 for i,link in enumerate(links):m.w(link,players[i]);m.w(link+4,links[i+1]if i+1<len(links)else 0)
 m.w(session+0xbc,links[0]if links else 0);m.w(node+8,players[case['local']]if case['local']is not None else 0)
 pa=m.alloc(0x6c*max(1,len(param_rows)));m.w(params+0x10,pa);m.w(params+0x14,len(param_rows))
 for i,(name,profile)in enumerate(param_rows):m.w(pa+i*0x6c,m.cstring(name));m.w(pa+i*0x6c+0x2c,m.cstring(profile))
 aa=m.alloc(4*max(1,len(ai_rows)));m.w(manager+0x60,aa);m.w(manager+0x64,len(ai_rows))
 for i,row in enumerate(ai_rows):m.w(aa+4*i,row)
 entries=m.alloc(12*(len(players)+1));m.w(header+0x44,entries);m.w(header+0x48,1);m.w(header+0x4c,len(players)+1)
 for i in range(len(players)+1):m.w(entries+12*i,m.cstring('stale'));m.w(entries+12*i+8,m.cstring('stale'))
 events=[];step=0
 def event(name,**kw):
  nonlocal step
  if step==case.get('stop',-1):raise Stopped(name)
  step+=1;events.append(dict(name=name,**kw))
 hooks={0x5a17f1:'first',0x5a1819:'ai',0x5a18da:'ai',0x765e00:'kingdom',0x4bedf0:'kingdom_name',0x79ea90:'profile',0x5a18dd:'name',0x766380:'display',0x5a1934:'local',0x5a198e:'next'}
 def observe(u,at,size,_):
  if at in hooks:event(hooks[at])
 m.u.hook_add(UC_HOOK_CODE,observe)
 stopped=None
 try:m.call(0x5a17d0,header)
 except Stopped as e:stopped=str(e)
 if stopped is None:
  assert all(m.u.reg_read(r)==0x5a5a0000 for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])
  assert(m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 out=[]
 for i in range(m.r(header+0x48)):
  p=m.r(header+0x44)+12*i;out.append(dict(flag=bytes(m.u.mem_read(p+4,1))[0],string_20=m.text(m.r(p+8)),string_18=m.text(m.r(p)),local=bytes(m.u.mem_read(p+5,1))[0]))
 profiles=[m.text(m.r(pa+i*0x6c+0x2c))for i in range(len(param_rows))]
 return dict(events=events,stopped=stopped,entries=out,profiles=profiles),m.windows()
def player(ai=0,kingdom=None,profiles=[],parameter=True):return dict(ai=ai,kingdom=kingdom,profiles=profiles,parameter=parameter,name='playername',display='display name')
def cases():
 for world in [False,True]:
  for rows in [[],[player()],[player(255,'kingdom',[None])],[player(1,'kingdom',['profileA','profileB']),player(0,'other',[])],[player(1,'kingdom',[],False)]]:
   yield dict(world=world,players=rows,local=0 if rows else None)
 base=dict(world=True,players=[player(1,'kingdom',['profileA','profileB']),player()],local=1)
 n=len(run(base,'original','Double53')[0]['events'])
 for stop in range(n):yield dict(base,stop=stop)
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 doc=dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-entries.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
