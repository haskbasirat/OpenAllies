"""Whole776580 implicit-AI selection,773710/7661D0 player publication,
770280 offline current-rule branch,766560 unique names and7664D0 assignment.
Current source-profile Find is an explicit canonical registry transport seam;
the same source manager binding is independently covered by runtime SAI tests.
The StringManager/CRT/diagnostic/heap successful seams are explicitly inherited.
No network transport, map loading, BeginPlaying, or controller startup claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges,NODE
from unicorn.x86_const import UC_X86_REG_ECX
class Machine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if self.bootstrap_ready and getattr(self,'active',False):
   if pc==0x48d500:
    name=self.text(self.arg());self.finds.append(name);self.ret(4,self.profiles.get(name.lower(),0));return
   if pc==0x405d60:
    fmt=self.text(self.arg(1));assert fmt=='%s (%d)',fmt
    self.cs(self.arg(),self.text(self.arg(2))+' ('+str(self.arg(3))+')');self.ret(value=self.arg());return
   if pc in(0x404f60,0x404f90,0x404ed0):self.ret();return
  if self.bootstrap_ready and pc==0x96cab0:self.ret(0,0);return
  super().hook(u,pc,size,user)
 def parameters(self,rows):
  creator=self.alloc(0x54);records=self.alloc(max(1,len(rows))*0x6c)
  self.w(creator+0x14,records);self.w(creator+0x18,len(rows));self.w(creator+0x1c,len(rows))
  self.w(creator+0x2c,self.alloc(12));self.w(creator+0x30,0);self.w(creator+0x34,0);self.w(creator+0x38,0)
  for i,row in enumerate(rows):
   p=records+i*0x6c
   for off,text in((0,row['id']),(4,'label'),(8,''),(0xc,''),(0x14,''),(0x2c,row['profile']),(0x58,'')):self.cs(p+off,text)
   self.b(p+0x10,row['independent']);self.b(p+0x28,1);self.b(p+0x29,row['enabled'])
  return creator
def main():
 rows=[];visited={}
 variants=[[],[{'id':'K0','profile':'','independent':0,'enabled':1}],
  [{'id':'K0','profile':'','independent':0,'enabled':1},{'id':'K1','profile':'','independent':0,'enabled':1}],
  [{'id':'skip','profile':'Known','independent':1,'enabled':1},{'id':'off','profile':'Known','independent':0,'enabled':0}],
  [{'id':'A','profile':'Known','independent':0,'enabled':1},{'id':'B','profile':'known','independent':0,'enabled':1},
   {'id':'C','profile':'Missing','independent':0,'enabled':1}]]
 for image in('original','v14'):
  m=Machine(image);m.nonorder.discard(0x9e7400);m.formats['networking_duplicate_name_formate|sd']='%s (%d)'
  for pc,phase,variant in itertools.product(PC,(12,7),range(len(variants))):
   m.active=False;m.fpu(pc);m.begin(1,list(range(16)),0);m.call(0x9e7400)
   m.call(0x76ec70,this=m.S);m.first_player('Human','local-source-identity')
   m.w(NODE+0x2c4,NODE);m.w(NODE+0x2c8,NODE)
   creator=m.parameters(variants[variant]);m.w(m.S+0x70,creator);m.w(m.S+0x58,2);m.w(m.S+0x54,2)
   launch=m.alloc(0x28);m.call(0x40f7d0,this=launch);m.w(launch,phase);m.w(m.S+0x80,launch)
   gd=m.alloc(0x600);registry=m.alloc(0x30);count=m.alloc(4);m.w(count,1);m.w(registry+0x24,count);m.w(gd+0x564,registry);m.w(0xb36530,gd)
   profile=m.alloc(0x30);m.cs(profile+8,'Known');m.cs(profile+0x10,'Native AI');m.profiles={'known':profile};m.finds=[]
   m.b(m.S+0xc8,0);m.active=True;m.call(0x776580,this=m.S)
   players=m.read_after()['players'];eligible=[r for r in variants[variant]if not r['independent']and(phase==7 or r['enabled'])]
   assert len(players)==1+len(eligible),(image,pc,variant,players)
   assert m.finds==[r['profile']for r in eligible]
   for p,r in zip(players[1:],eligible,strict=True):
    assert p['ai']and p['implicit']and not p['film']and p['status']==2
    assert p['kingdom']==[ord(c)for c in r['id']]and p['flag_2c']
    assert p['text_14']==[ord(c)for c in('Known'if r['profile'].lower()=='known'else'')]
   rows.append({'image':image,'precision':pc,'launch_kind':phase,'parameters':variants[variant],
    'players':players,'last_player_id':m.r(m.S+0xe8),'changed_c8':bool(m.rb(m.S+0xc8)),
    'finds':m.finds,'created_id':players[-1]['id']if eligible else players[0]['id']})
   visited.update(m.visited)
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-implicit.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'implicit-AI rows')
if __name__=='__main__':main()
