"""Execute original553CE0 including real553BA0 and full native shape geometry.
Only existing prepared-query allocation/policy leaves are intercepted. Captured
query calls record inputs, then continue through their unchanged original bodies.
"""
from pathlib import Path
import copy,json,sys,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-spatial'))
from original_query import Query,bits,W,H,original,modified
from unicorn.x86_const import *
class Sweep(Query):
 def __init__(self,row,cw):self.query_calls=[];super().__init__(row,cw);self.query_calls=[]
 def descriptor(self,a):
  return {'placement':{'shape':self.shape(a),'facing':[self.r(a+0x44),self.r(a+0x48)],'position':[self.r(a+0x4c),self.r(a+0x50)]},
   'required_actor_flags':self.r(a+0x38),'mask':self.r(a+0x3c),'source_definition':self.r(self.r(a+0x1c)+4)==self.r(a+0x18)if self.r(a+0x1c)else False,
   'ignore_all':bool(self.u.mem_read(a+0x30,1)[0]),'custom_filter':bool(self.r(a+0x34))}
 def hook(self,u,pc,size,user):
  if pc==0x553ba0:self.query_calls.append(self.descriptor(self.r(u.reg_read(UC_X86_REG_ESP)+8)))
  super().hook(u,pc,size,user)
 def sweep(self,goal):
  self.reset(0);self.calls=[];self.query_calls=[];self.u.reg_write(UC_X86_REG_ECX,self.STATE);self.u.mem_write(self.STACK+4,W(self.OUT,self.QUERY,*goal));self.u.emu_start(0x553ce0,self.STOP,count=1000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==self.STACK+20
  words=list(struct.unpack('<4I',self.u.mem_read(self.OUT,16)))
  return {'prepared':self.prepared(),'hit':{'kind':words[0],'reference':words[1],'cell':[struct.unpack('<i',W(x))[0]for x in words[2:]]},'calls':self.calls,'queries':self.query_calls}
def main():
 base=json.loads((ROOT/'research/phase7-spatial/native-query.json').read_text());rows=[];visited={}
 for i,c in enumerate(base['cases']):
  if i%3:continue
  row=copy.deepcopy(c['input']);row['pre_marked']=[]
  for n,m in enumerate(row['members']):m['actor']['position']=list(map(bits,[[5,5],[9,5],[7,7],[7,5],[12,5],[9,9]][n]))
  for goal in [[5,5],[5.009,5],[5.01,5],[7,5],[14,10],[-1,1],[9,9],[6.25,6.75]]:
   try:m=Sweep(row,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);out=m.sweep(list(map(bits,goal)))
   except Exception as e:raise RuntimeError((i,goal))from e
   rows.append({'precision':c['precision'],'input':row,'goal':list(map(bits,goal)),'geometry':m.geometry,'initial':m.prepared_initial,'output':out});visited.update(m.visited)
 result={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'query_producer_sha256':H((ROOT/'research/phase7-spatial/original_query.py').read_bytes()),'query_fixture_sha256':H((ROOT/'research/phase7-spatial/native-query.json').read_bytes()),'original_sha256':H(original.b),'accepted_sha256':H(modified.b),'cases':rows,'executed_windows':[{'va':hex(a),'bytes':original.range(a,n).hex()}for a,n in sorted(visited.items())]}
 p=HERE/'native-sweep.json';p.write_text(json.dumps(result,indent=2)+'\n');print({'cases':len(rows),'sha256':H(p.read_bytes())})
if __name__=='__main__':main()
