"""Native510EB0 Kingdom phase through511297, real50F4C0/4E5340/49AE00/
4A4DC0/45F180 and CString/vector growth. 4E93C0 is an explicit glyph child
seam;4BAA20 is an explicit existing-entry lookup seam, after which native
49AE00 writes that actual entry. No trigger-extra/full510EB0 claim.
Preinstruction cutoff rows qualify field/service prefixes, not C++ unwinding.
Native unused-capacity record constructors execute but their immutable global
resource-count reads are not service cutoffs: Rust uses growable storage and
does not preconstruct unused records. Temporary actual-record reads remain.
"""
from pathlib import Path
import hashlib,json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from native_film_teams import cstring,read
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Complete(Exception):pass
class Stopped(Exception):pass

def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
 data=m.alloc(0x100);m.w(0xb36530,data);m.w(data+0x38,case['resources']);m.w(0xb43b48,cstring(m,[]))
 target=m.alloc(0x30);m.call(0x50f890,target);assert m.r(target+8)==0 and m.r(target+0x14)==0
 m.w(target+0x14,1) #Native clear must precede the first Kingdom slot read.
 world=m.alloc(0x300);slots=m.alloc(4*max(1,len(case['slots'])));m.w(world+0x148,slots);m.w(world+0x14c,len(case['slots']))
 definitions={};definition_tokens={};kingdoms={};globals={};entries={}
 def definition(token):
  if token is None:return 0
  if token not in definitions:
   p=m.alloc(0x30);m.w(p+8,cstring(m,[68,token]));definitions[token]=p;definition_tokens[p]=token
  return definitions[token]
 for i,row in enumerate(case['slots']):
  p=0
  if row is not None:
   p=m.alloc(0x300);kingdoms[p]=row;m.w(p,0xa299d8)
   for offset,name in [(0x18,'id'),(0x194,'key'),(0x198,'text'),(0x240,'profile')]:m.w(p+offset,cstring(m,row[name]))
   for offset,name in [(0x190,'minor'),(0x19c,'mode')]:m.w(p+offset,row[name])
   for offset,name in [(0x23b,'allow'),(0x23c,'eligible'),(0x23d,'byte32')]:m.u.mem_write(p+offset,bytes([row[name]&255]))
   for offset,name in [(0x234,'nation'),(0x1e8,'color'),(0x244,'handicap')]:m.w(p+offset,definition(row[name]))
   if row['faction']is not None:
    commander=m.alloc(0x10);m.w(commander+4,definition(row['faction']));m.w(p+0x230,commander)
   if row['team']is not None:
    team=m.alloc(0x40);m.w(team+0x18,cstring(m,row['team']));m.w(p+0x1ec,team)
   key=tuple(row['key'])
   if key not in entries:
    entry=m.alloc(0x20);m.w(entry+4,10000);m.w(entry+0x10,cstring(m,[111,108,100]));entries[key]=entry
  m.w(slots+4*i,p)
 events=[];step=0
 def event(name):
  nonlocal step
  if step==case.get('stop',-1):raise Stopped(name)
  step+=1;events.append(name)
 def lookup():
  event('lookup')
  pair=m.arg(1);key=tuple(read(m,m.r(pair)));entry=entries[key]
  m.w(m.arg(0),entry);m.w(entry+4,m.r(entry+4)+1);m.ret(16,m.arg(0))
 def glyph():
  event('glyph');row=kingdoms[m.u.reg_read(UC_X86_REG_ECX)]
  m.w(m.arg(0),cstring(m,row['glyph']));m.ret(4,m.arg(0))
 m.seam(0x4baa20,lookup);m.seam(0x4e93c0,glyph)
 hooks={0x4f0680:'slots',0x4f06b0:'next',0x5110c9:'key',0x5110d9:'text',0x4e5340:'name',0x49ae00:'set_name',
        0x51116e:'minor',0x51117d:'team',0x5111c9:'nation',0x5111d3:'faction',0x5111e8:'color',0x5111f2:'handicap',
        0x5111fc:'allow',0x511206:'eligible',0x511210:'mode',0x511222:'profile',0x511242:'byte32'}
 def observe(u,at,size,_):
  if at==0x511297:raise Complete()
  if at==0x45f180 and L.STACK<=u.reg_read(UC_X86_REG_ECX)<L.STACK+L.STACK_SIZE:event('resources')
  if at in hooks:event(hooks[at])
  if at==0x4bedf0:event('id'if u.reg_read(UC_X86_REG_ECX)in kingdoms else'team_name')
 m.u.hook_add(UC_HOOK_CODE,observe)
 stopped=None;complete=False
 try:m.call(0x510eb0,target,[world])
 except Complete:complete=True
 except Stopped as e:stopped=str(e)
 out=[]
 for i in range(m.r(target+0x14)):
  p=m.r(target+0x10)+0x6c*i
  result={name:read(m,m.r(p+offset))for offset,name in [(0,'id'),(4,'key'),(8,'text'),(12,'glyph'),(0x14,'team'),(0x2c,'profile')]}
  result.update({name:definition_tokens.get(m.r(p+offset))for offset,name in [(0x18,'nation'),(0x1c,'faction'),(0x20,'color'),(0x24,'handicap')]})
  result.update({name:bytes(m.u.mem_read(p+offset,1))[0]for offset,name in [(0x10,'minor'),(0x28,'eligible'),(0x29,'allow'),(0x30,'byte30'),(0x31,'byte31'),(0x32,'byte32'),(0x68,'byte68')]})
  result['resource_bits']=[m.r(m.r(p+0x34)+4*j)for j in range(m.r(p+0x38))]
  result['entry_count']=m.r(p+0x44);result['definition_count']=m.r(p+0x50);result['property_count']=m.r(p+0x60);result['string58']=read(m,m.r(p+0x58))
  out.append(result)
 return dict(events=events,stopped=stopped,completed_kingdom_phase=complete,kingdoms=out,
             globals=[dict(key=list(k),text=read(m,m.r(e+0x10)))for k,e in entries.items()]),m.windows()

def kingdom(i=1,**kw):
 row=dict(id=[75,i],key=[107,i],text=[116,i],glyph=[60,99,62,0xd800],profile=[112,i],minor=0,mode=1,allow=255,eligible=1,byte32=3,
          nation=1,faction=2,color=3,handicap=4,team=[84,i])
 row.update(kw);return row
def cases():
 for slots in [[],[None],[kingdom()],[None,kingdom(minor=7,eligible=0,team=None,nation=None,faction=None,color=None,handicap=None),None],
               [kingdom(mode=2),kingdom(2,key=[],team=[],allow=0,eligible=255)], [kingdom(i)for i in range(20)]]:
  yield dict(slots=slots,resources=3)
 yield dict(slots=[kingdom()],resources=0)
 base=dict(slots=[None,kingdom(),None,kingdom(2)],resources=9)
 n=len(run(base,'original','Double53')[0]['events'])
 for stop in range(n):yield dict(base,stop=stop)
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 doc=dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-kingdoms.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
