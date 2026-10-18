"""Bounded original48C050 Nation factions/selectable and48BAE0 Faction public producers.
OriginalTGI parser/getters/tokenizer/vector writes execute. Actual prologue/frame
is retained, unrelated Nation art/audio/Badge islands skipped explicitly. Canonical
Faction Find is a recorded identity service. Not full LoadData.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-world-start'))
import native_technology_source as old
import trace_registry as mapped
from unicorn.x86_const import *
ORIGINAL=old.P;V14=old.V
class Cutoff(Exception):pass
class M(old.TechMachine):
 def __init__(self):self.active=False;super().__init__();self.factions={n:self.alloc(0x40)for n in ['first','second','third']}
 def hook(self,u,pc,size,user):
  if self.active:
   esp=u.reg_read(UC_X86_REG_ESP)
   if pc==0x48c09a:
    node=u.reg_read(UC_X86_REG_EBX);dest=self.r(esp+0x18);u.reg_write(UC_X86_REG_EBP,node);u.reg_write(UC_X86_REG_EBX,dest);u.reg_write(UC_X86_REG_EIP,0x48c8c7);return
   if pc==0x48c9c4:u.reg_write(UC_X86_REG_EIP,0x48cb51);return
   if pc==0x48cb60:raise Cutoff()
   if pc==0x48bb33:
    u.reg_write(UC_X86_REG_EBP,self.node);u.reg_write(UC_X86_REG_EDI,self.dest)
   if pc==0x48bb4d:raise Cutoff()
   if pc==0x48d500:
    token=self.raw_wstr(self.arg());target=self.factions.get(token.lower(),0);self.recorded.append(dict(event='faction_find',token=token,found=bool(target)));self.ret(target,pop=4);return
  super().hook(u,pc,size,user)

def run(c,image,pc):
 mapped.P=ORIGINAL if image=='original'else V14
 try:m=M()
 finally:mapped.P=ORIGINAL
 m.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[pc]);m.visited.clear()
 if c['kind']=='nation':
  template='[Template Nation]\n{string IDS\n string factions = NULL\n flag selectable = true\n}\n'
  text='[Nation template=Nation]\n{IDS = subject\n'+('factions = '+c['tokens']+'\n'if c['tokens']is not None else'')+('selectable = '+str(c['selectable']).lower()+'\n'if c['selectable']is not None else'')+'}\n';start=0x48c050
 else:
  template='[Template Faction]\n{string IDS\n bool public = true\n}\n';text='[Faction template=Faction]\n{IDS = subject\n'+('public = '+str(c['public']).lower()+'\n'if c['public']is not None else'')+'}\n';start=0x48bb33
 m.parse_file('template.tgi',template.encode());parsed=m.parse_file('subject.tgi',text.encode());m.node=m.children(parsed['root'])[0]
 m.dest=m.alloc(0x80);m.u.mem_write(m.dest,b'\x00'*0x80);m.w(m.dest+0x10,m.cstring(''));m.w(m.dest+8,m.cstring('subject'));m.active=True
 try:
  result=m.thiscall(start,m.dest,[m.node]);raise AssertionError(('missing cutoff',result,hex(m.u.reg_read(UC_X86_REG_EIP)),m.last_throw,m.recorded[-8:]))
 except Cutoff:pass
 except AssertionError:
  if not(m.u.reg_read(UC_X86_REG_EIP)==0x96bef5 and m.last_throw):raise
 rev={p:n for n,p in m.factions.items()};out=dict(throw=m.last_throw,events=[e for e in m.recorded if e.get('event')=='faction_find'])
 if c['kind']=='nation':
  out['factions']=[rev[m.r(m.r(m.dest+0x48)+4*i)]for i in range(m.r(m.dest+0x4c))];out['selectable']=bytes(m.u.mem_read(m.dest+0x68,1))[0]
 else:out['public']=bytes(m.u.mem_read(m.dest+0x18,1))[0]
 pins={a:bytes(m.u.mem_read(a,n))for a,n in m.visited.items()if ORIGINAL.base<=a<ORIGINAL.base+ORIGINAL.image_size and a not in {0x48d500,0x48c09a,0x48c9c4,0x48cb60,0x48bb4d}}
 return out,pins

def main():
 cases=[dict(kind='nation',tokens=t,selectable=s)for t,s in itertools.product([None,'NULL','first','first,second,third','second,FIRST,second',' first, second','first,,second','first,missing,third'],[None,False,True])]+[dict(kind='faction',public=v)for v in [None,False,True]]
 rows=[];pins={}
 for pc,c in itertools.product(['Single24','Double53','Extended64'],cases):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;(pins.update(pa),pins.update(pb));rows.append(dict(precision=pc,input=c,output=a))
 for a,b in pins.items():assert ORIGINAL.range(a,len(b))==V14.range(a,len(b))==b,hex(a)
 doc=dict(scope=__doc__,rows=rows,native_executions=2*len(rows),instructions=[dict(va=hex(a),bytes=b.hex())for a,b in sorted(pins.items())])
 p=HERE/'native-nation-source.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()


