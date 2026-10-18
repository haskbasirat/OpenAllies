"""Full native773E30 caller, real4F48E0 and765E10; AI/diagnostic/rename,
creator lookup and glyph are explicitly labeled leaf seams. Separate49AE00
fixture proves actual pair/global writes. No ready/battle/AI completeness claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,NETNODE,NODE,proof_ranges
from unicorn.x86_const import *
class PrepareMachine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if getattr(self,'prepare_ready',False):
   sp=u.reg_read(UC_X86_REG_ESP);ecx=u.reg_read(UC_X86_REG_ECX)
   if pc==0x4f48e0:self.trace.append(['find_kingdom',self.rs(self.arg())])
   if pc==0x765e10:self.trace.append(['bind_765E10',self.arg()!=0])
   if pc==0x766960:self.trace.append(['create_sai',self.arg()]);self.ret(4);return
   if pc==0x404f60:self.trace.append(['diagnostic']);self.ret();return
   if pc==0x40d960:self.trace.append(['application_mode',self.app_mode])
   if pc==0x4f1310:self.trace.append(['mode_4F1310'])
   if pc==0x50e0d0:
    value=self.rs(sp+4);self.trace.append(['kingdom_label',value]);self.cs(self.K+0x198,''.join(chr(x)for x in value));self.ret(4);return
   if pc==0x50dc00:
    self.trace.append(['creator_find',self.rs(self.arg())]);self.ret(4,self.creator_record);return
   if pc==0x49ae00 and ecx==self.creator_record+4:
    value=self.rs(self.arg());self.trace.append(['creator_name',value]);self.cs(self.creator_record+8,''.join(chr(x)for x in value));self.ret(4);return
   if pc==0x7666c0:
    self.trace.append(['player_glyph',self.glyph is not None]);self.ret(value=0 if self.glyph is None else self.G);return
   if pc==0x405d60 and self.text(self.arg(1))=='%c':
    unit=self.arg(2)&0xffff;self.trace.append(['glyph_format',unit]);self.cs(self.arg(),chr(unit));self.ret();return
   if pc==0x405b80 and ecx==self.creator_record+0xc:
    self.trace.append(['glyph_clear']);
  super().hook(u,pc,size,user)
def main():
 rows=[];visited={}
 probes=[(False,1,False,True,None),(True,1,False,True,None),(True,1,False,True,132),(True,4,False,True,132),(False,4,False,True,None),(True,1,True,True,132),(True,1,True,False,None)]
 for image in ('original','v14'):
  for p in PC:
   for assigned,mode,ai,local,glyph in probes:
    m=PrepareMachine(image);m.fpu(p);m.begin(0,bytes(16),0);m.first_player('Host',[])
    P=m.r(NODE+8);m.b(P+0xc,ai);m.w(P+4,NODE if local else m.alloc(0x30));label=m.alloc(4);m.cs(label,'K'if assigned else'');m.b(P+0x2c,0);m.call(0x7664d0,[label],this=P,callee_pop=4);assert m.rb(P+0x2c)==1
    W=m.alloc(0x300);m.w(0xb36534,W);m.K=m.alloc(0x400);m.cs(m.K+0x18,'K');m.cs(m.K+0x194,'DisplayKey');m.cs(m.K+0x198,'old-label')
    slots=m.alloc(8);m.w(slots,0);m.w(slots+4,m.K);m.w(W+0x148,slots);m.w(W+0x14c,2)
    nation=m.alloc(0x20);m.cs(nation+0x10,'Nation');m.w(m.K+0x234,nation)
    app=m.alloc(0x14);m.call(0x651810,this=app);m.w(app+0xc,mode);m.w(0xb39690+0xfc,app);m.app_mode=mode
    resource=m.alloc(0x94);creator=m.alloc(0x38);m.creator_record=m.alloc(0x6c);m.cs(m.creator_record,'K');m.cs(m.creator_record+4,'SetupKey');m.cs(m.creator_record+8,'old-setup');m.cs(m.creator_record+0xc,'old-glyph');m.w(W+0x28,resource);m.w(resource+0x88,creator)
    m.G=m.alloc(0x20);m.w(m.G+0x18,glyph or 0);m.glyph=glyph
    m.trace=[];m.prepare_ready=True;m.call(0x773e30,this=m.S)
    assert m.r(P+0x28)==(m.K if assigned else 0)
    assert m.r(m.r(P+0x50))==(m.K if assigned else 0)
    expected='Nation'if assigned and mode==4 else'Host'if assigned else'old-label'
    assert m.rs(m.K+0x198)==[ord(c)for c in expected]
    normal=assigned and mode!=4
    assert m.rs(m.creator_record+8)==[ord(c)for c in ('Host'if normal else'old-setup')]
    assert m.rs(m.creator_record+0xc)==([glyph]if normal and glyph is not None else[]if normal else[ord(c)for c in 'old-glyph'])
    rows.append({'image':image,'precision':p,'input':{'assigned':assigned,'mode':mode,'ai':ai,'local_link':local,'glyph':glyph},'bound':assigned,'kingdom_label':m.rs(m.K+0x198),'creator_label':m.rs(m.creator_record+8),'creator_glyph':m.rs(m.creator_record+0xc),'trace':m.trace.copy()})
    visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-prepare-players.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'prepare caller rows')
if __name__=='__main__':main()
