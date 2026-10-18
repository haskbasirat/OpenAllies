"""Actual474C50 default reset and512200 three-map publication on all231
canonical source Vars. Input source fields are independently compared against
full5065D0+bothfiles native proof before use. Actual source-bound WorldRule
records are explicit descriptor transports, not a whole descriptor constructor.
405370 named lookup is a transport to SAME logical Vars. Typed setters and
474C50/512200 bodies execute. No missing-var or changed-policy success claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net-integration'));from native_launch_assignment import Machine as Base
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPCW
class Machine(Base):
 def __init__(self,image,pc,source,rules):
  self.intercepted=set();super().__init__(False,image);self.mode='channels';self.u.reg_write(UC_X86_REG_FPCW,h.PRECISION[pc]);self.lookup={};self.vars=[]
  self.seams[0x405370]=self.named
  array=self.alloc(4*len(source));self.w(0xb39ab0,array);self.w(0xb39ab4,len(source))
  for row in source:
   v=self.alloc(0x2c);self.w(array+4*len(self.vars),v);self.vars.append(v);self.cs(v+0x1c,row['name']);self.w(v+0x20,row['ordinal']);self.w(v+0x24,row['kind']);self.w(v+0x28,row['flags']);self.w(v+0x14,row['bounds'][0]);self.w(v+0x18,row['bounds'][1])
   if row['kind']in(1,2,3):self.w(v,row['words'][0])
   elif row['kind']==5:
    for i,x in enumerate(row['words']):self.w(v+4+i*4,x)
   else:self.cs(v+0x10,row['text'])
   node=self.alloc(4);self.w(node,v);self.lookup[row['name'].lower()]=node
  data=self.alloc(0x500);self.w(0xb36530,data);ptrs=self.alloc(len(rules)*4);self.w(data+0x118,ptrs);self.w(data+0x11c,len(rules));self.descriptors=[]
  for i,row in enumerate(rules):
   d=self.alloc(0x50);self.w(ptrs+4*i,d);self.cs(d+8,row['name']);self.w(d+0x20,row['kind']);self.w(d+0x38,row['enum_count']);self.w(d+0x40,row['default']);self.descriptors.append(d)
  self.rules_owner=self.alloc(0x20);a=self.alloc(12*len(rules));self.w(self.rules_owner+0xc,a);self.w(self.rules_owner+0x10,len(rules))
  for i,(d,row)in enumerate(zip(self.descriptors,rules)):self.w(a+12*i,d);self.w(a+12*i+4,0);self.w(a+12*i+8,row['value'])
 def named(self):
  name=''.join(map(chr,self.rs(self.arg(0)))).lower();assert name in self.lookup,name;self.ret(4,self.lookup[name])
 def hook(self,u,at,n,z):
  super().hook(u,at,n,z)
  if u.reg_read(UC_X86_REG_EIP)!=at:self.intercepted.add(at)
 def snapshot(self,source):
  out=[]
  for v,row in zip(self.vars,source):
   k=self.r(v+0x24);out.append(dict(ordinal=self.r(v+0x20),name=row['name'],kind=k,flags=self.r(v+0x28),bounds=[self.r(v+0x14),self.r(v+0x18)],words=[self.r(v)&255]if k==1 else[self.r(v)]if k in(2,3)else[self.r(v+4+i*4)for i in range(3)]if k==5 else[],text=self.rs(v+0x10)if k==4 else None))
  return out

def compare_source(actual):
 p=ROOT/'research/skirmish-20261017-svar-field-order/native-full-sources.json';full=json.loads(p.read_bytes())
 for pc in full['rows']:
  rows=pc['native']['stages'][-1]['registrations'];assert len(rows)==len(actual)==231
  for a,n in zip(actual,rows):
   for key,nkey in[('ordinal','index'),('name','name'),('kind','kind'),('flags','flags')]:assert a[key]==n[nkey],(key,a,n)
   assert a['words']==(n['value']if a['kind']!=4 else[]),(a,n)
   if a['kind']==4:assert a['text']==list(map(ord,n['value']))
   if a['kind']in(2,3):assert a['bounds']==n['bounds']
 return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 p=HERE/'actual-current-inputs.json';data=json.loads(p.read_bytes());source=data['source'];native_source_sha=compare_source(source);rows=[];pins={};seams=set()
 for pc in h.PRECISION:
  machines={i:Machine(i,pc,source,data['cases'][0]['rules'])for i in('original','v14')}
  baseline=None
  for i,m in machines.items():
   m.call(0x474c50,[],callee_pop=0);reset=m.snapshot(source)
   assert reset==data['reset'],('reset',i,pc,[(a,b)for a,b in zip(reset,data['reset'])if a!=b][:1])
   if baseline is None:baseline=reset
   else:assert reset==baseline
   for case in data['cases']:
    #SAME descriptor allocations/order; only actual retained Rules values change.
    arr=m.r(m.rules_owner+0xc)
    for j,r in enumerate(case['rules']):m.w(arr+12*j+8,r['value'])
    m.call(0x512200,[],this=m.rules_owner,callee_pop=0);current=m.snapshot(source)
    assert current==case['current'],('publish',i,pc,case['map'],[(a,b)for a,b in zip(current,case['current'])if a!=b][:1])
    rows.append(dict(image=i,precision=pc,map=case['map'],reset=reset,current=current))
   pins.update(m.visited);seams|=m.intercepted
 for a,n in pins.items():
  if a not in seams and 0x401000<=a<0x9ef000:assert h.P.range(a,n)==h.V.range(a,n)
 out=dict(schema=1,scope=__doc__,actual_inputs_sha256=hashlib.sha256(p.read_bytes()).hexdigest(),full_source_native_sha256=native_source_sha,rows=rows,executed=[dict(address=hex(a),bytes=h.P.range(a,n).hex())for a,n in sorted(pins.items())if a not in seams and 0x401000<=a<0x9ef000],intercepted=[hex(a)for a in sorted(seams)])
 target=HERE/'native-reset-publication.json';raw=(json.dumps(out,indent=1)+'\n').encode()
 if '--write'in sys.argv:target.write_bytes(raw)
 else:assert target.read_bytes()==raw
 print(json.dumps(dict(source_current_comparisons=231*3,worldrule_cases=len(rows),worldrule_executions=24,pins=len(out['executed']),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
