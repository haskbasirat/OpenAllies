"""Actual7791F0 fresh human refresh +77E5E0/76B420 constructor fields.
Native full descendant SetPlayerKingdom, selection, parameter lookups and
positive authored Nation/Faction no-write paths execute. Windows/allocator,
CString and successful diagnostic leaves reuse named bootstrap seams.
Synthetic parameter inputs are explicit, not actual source-map captures.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
from unicorn.x86_const import UC_X86_REG_ECX
class Machine(BootstrapMachine):
 def hook(self,u,pc,size,user):
  if self.bootstrap_ready and pc in (0x404f90,0x404f60):self.ret();return
  super().hook(u,pc,size,user)
 def parameters(self,rows):
  params=self.alloc(0x1c);records=self.alloc(max(1,len(rows))*0x6c)
  self.w(params+0x10,records);self.w(params+0x14,len(rows));self.w(params+0x18,len(rows))
  for i,row in enumerate(rows):
   p=records+i*0x6c;self.u.mem_write(p,b'\0'*0x6c)
   for off,text in ((0,row['id']),(4,'label'),(8,'text'),(0xc,''),(0x14,''),(0x2c,''),(0x58,'')):self.cs(p+off,text)
   self.b(p+0x10,row['independent']);self.b(p+0x28,row['human']);self.b(p+0x29,1)
   self.w(p+0x18,self.nation);self.w(p+0x1c,self.faction);self.w(p+0x24,self.handicap)
  return params,records
def main():
 rows=[];visited={}
 variants=[[],[{'id':'A','independent':0,'human':1}],
  [{'id':'skip','independent':1,'human':1},{'id':'A','independent':0,'human':1}],
  [{'id':'disabled','independent':0,'human':0},{'id':'A','independent':0,'human':1}],
  [{'id':'A','independent':0,'human':1},{'id':'B','independent':0,'human':1}]]
 for image in ('original','v14'):
  m=Machine(image)
  #Inherited lobby tests stub AutoAssign. This proof must execute it.
  del m.seams[0x778d10]
  for pc in PC:
   for variant,parameters in enumerate(variants):
    m.fpu(pc);m.begin(1,list(range(16)),0)
    data=m.alloc(0x58);m.u.mem_write(data,b'\xa5'*0x58);m.call(0x77e5e0,this=data);m.w(m.S+0x14,data)
    before=bytes(m.u.mem_read(data,0x58));m.call(0x76ec70,this=m.S);m.first_player('Human','identity')
    assert bytes(m.u.mem_read(data,0x58))==before
    for off in (0,4,8,0xc,0x10,0x14,0x1c,0x20):assert m.r(data+off)==0
    m.nation=m.alloc(0x50);m.faction=m.alloc(0x50);m.handicap=m.alloc(0x20)
    creator=m.alloc(0x54);source_params,source_records=m.parameters(parameters)
    actual_params,actual_records=m.parameters(parameters)
    #The embedded Creator+4 WorldParameters is a separate allocation's value.
    m.u.mem_write(creator+4,bytes(m.u.mem_read(actual_params,0x1c)))
    header=m.alloc(0x68);m.w(header+8,source_params);m.w(m.S+0x68,header);m.w(m.S+0x70,creator)
    m.w(m.S+0x58,2);m.w(m.S+0x54,2);m.w(m.S+0x60,0);m.b(m.S+0x169,0);m.b(m.S+0xc8,0)
    #Actual4981A0 initialized WorldRuleMode0 payload flag=false, read by511860.
    table=m.alloc(4);descriptor=m.alloc(12);payload=m.alloc(2)
    m.w(table,descriptor);m.w(descriptor+8,payload);m.b(payload,0);m.w(0xb3b198,table)
    rules=m.alloc(0x1c);m.w(rules+4,0);m.w(header+4,rules)
    #7788B0 reads current Creator WorldRules+4 through76C6B0.
    m.w(creator+0x24,0)
    player=m.r(m.r(m.S+0xbc));old_38=m.rb(player+0x38);m.call(0x7791f0,this=m.S)
    chosen=next((r['id']for r in parameters if not r['independent']and r['human']),None)
    actual=m.rs(player+0x24);assert actual==[ord(c)for c in(chosen or'')],(image,pc,variant,actual,chosen,hex(m.r(creator+0x14)),m.r(creator+0x18),[hex(x)for x in m.visited if 0x778d10<=x<0x778f10])
    assert m.rb(player+0x2c)==1
    assert m.r(player+0x3c)==0xffffffff and m.r(player+0x40)==0xffffffff
    assert m.rb(player+0x38)==(1 if chosen is not None else old_38)
    for i,row in enumerate(parameters):
     assert m.r(source_records+i*0x6c+0x24)==m.handicap
     assert m.r(actual_records+i*0x6c+0x24)==(0 if row['id']==chosen else m.handicap)
    assert bytes(m.u.mem_read(data,0x58))==before
    rows.append({'image':image,'precision':pc,'variant':variant,'parameters':parameters,
     'kingdom':actual,'flag_2c':bool(m.rb(player+0x2c)),'observer_38':bool(m.rb(player+0x38)),
     'request_3c':m.r(player+0x3c),'request_40':m.r(player+0x40),'changed_c8':bool(m.rb(m.S+0xc8)),
     'constructor_staging_lists_unchanged':True,'source_parameter_handicaps_unchanged':True})
    visited.update(m.visited)
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 out={'passed':True,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-refresh.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'native fresh-human refresh rows')
if __name__=='__main__':main()
