"""Whole770280 current-rule offline branch and exact online first boundary.
Native76C6B0/511A60/511AF0 execute on explicit synthetic current-rule owners.
Online mode0/2 stops at76B490, before its unresolved GameType/property/CRC
dependencies; this is a required prefix, never a completed online update.
Windows/heap/CString successful transport leaves inherit BootstrapMachine.
ASCII wcsicmp is an explicit CRT transport seam. No actual map-load claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import UC_X86_REG_EIP
class RequiredOnline(Exception):pass
class Machine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if self.bootstrap_ready:
   if pc==0x76b490:
    self.online=True;raise RequiredOnline()
   if pc==0x511a60:self.lookup=True
   if pc==0x96cad6:
    self.ret(value=int(self.text(self.arg()).lower()!=self.text(self.arg(1)).lower()));return
  super().hook(u,pc,size,user)
def main():
 rows=[];visited={}
 variants=[('missing',None,0,0),('bool0','RATED',0,0xaabbcc00),
  ('bool1','rAtEd',0,0xaabbcc01),('bool128','RATED',0,0xaabbcc80),
  ('bool255','RATED',0,0xaabbccff),('wrong-kind','RATED',1,0x11223380)]
 for image in ('original','v14'):
  m=Machine(image)
  for pc,mode,game_type,bc,variant in itertools.product(PC,(0,1,2,3,5,7),(0,1,2,5),(0,128),variants):
   label,ids,kind,word=variant
   m.fpu(pc);m.begin(1,list(range(16)),0)
   flags=m.alloc(0xc0);m.b(flags+0xbc,bc);m.w(0xb3654c,flags)
   m.w(m.S+0x58,mode);m.w(m.S+0x54,game_type);m.b(m.S+0xc9,0)
   creator=m.alloc(0x54)
   for off in (0x6c,0x70,0x74,0x7c):m.w(m.S+off,creator)
   rules=creator+0x20;data=m.alloc(12);descriptor=m.alloc(0x44)
   m.cs(descriptor+8,ids or'Other');m.w(descriptor+0x20,kind)
   m.w(rules+0xc,data);m.w(rules+0x10,int(ids is not None));m.w(rules+0x14,int(ids is not None));m.w(rules+0x18,0xffffffff)
   m.w(data,descriptor);m.w(data+8,word);m.online=False;m.lookup=False
   try:m.call(0x770280,this=m.S)
   except RequiredOnline:assert m.u.reg_read(UC_X86_REG_EIP)==0x76b490
   online=bc!=0 and mode in(0,2)
   lookup=not online and mode in(0,1,2,5)and game_type in(1,2)
   changes=lookup and ids is not None and (word&255)!=0
   stored=changes and kind==0
   assert m.online==online and m.lookup==lookup
   assert bool(m.rb(m.S+0xc9))==changes
   assert m.r(data+8)==(word&~255 if stored else word)
   assert m.r(rules+0x18)==(0 if stored else 0xffffffff)
   rows.append({'image':image,'precision':pc,'mode':mode,'game_type':game_type,'internet_bc':bc,'variant':label,
    'online_reached':m.online,'rule_lookup_reached':m.lookup,'changed_c9':bool(m.rb(m.S+0xc9)),
    'before_word':word,'after_word':m.r(data+8),'changes_after':m.r(rules+0x18),'completed':not online})
   visited.update(m.visited)
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),
  'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-rated.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'native rated rows')
if __name__=='__main__':main()
