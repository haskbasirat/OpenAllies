"""Native502440 kind5 aggregation inside actual581630/581810 mixed lifetimes."""
from pathlib import Path
import sys,json,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];sys.path.insert(0,str(HERE));import property_flags as flags
base=flags.base;H=flags.H
class Machine(flags.Machine):
 def __init__(self,c):
  converted=copy.deepcopy(c)
  for d in converted['definitions'].values():d['modifiers']=[{'Flag':{'index':63}}if 'Resistance'in m else m for m in d['modifiers']]
  super().__init__(converted);self.c=copy.deepcopy(c)
  for id,d in c['definitions'].items():
   for i,m in enumerate(d['modifiers']):
    if 'Resistance'in m:r=m['Resistance'];q=base.num(int(id),i);self.w(q,0xa2d708);self.w(q+8,5);self.w(q+12,r['amount']);self.w(q+16,0x3720000+r['damage_type']*0x100)
 def snapshot(self):
  d=super().snapshot();rows=[]
  for i in range(self.r(base.PROP+0x1c+0x78)):
   p=self.r(self.r(base.PROP+0x1c+0x74)+4*i);rows.append({'damage_type':(self.r(p)-0x3720000)//0x100,'factor':self.r(p+4),'resistance':self.r(p+8)})
  d['damage_factors']=rows;return d
def cases():
 r=lambda type,amount:{'Resistance':{'damage_type':type,'amount':base.bits(amount)}}
 f=lambda i:{'Flag':{'index':i}}
 n=lambda k,i,a:{'Numeric':{'kind':k,'index':i,'amount':base.bits(a),'repair':False}}
 one=lambda m,flags=1:{'flags':flags,'modifiers':m}
 ins=lambda h,d,s=0x10001:{'Insert':{'handle':h,'definition':d,'kind':0,'created':base.bits(10),'source_id':s,'disabled':False}}
 rem=lambda d,s=1:{'Remove':{'definition':d,'source':s}}
 rows=[]
 def add(name,defs,ops,minimum=.001):rows.append({'name':name,'definitions':{str(i+1):d for i,d in enumerate(defs)},'operations':ops,'minimum':base.bits(minimum),'mutations':[],'mover':True,'character':True,'economy':True})
 for amount in [-1.5,-1,-.2,-1e-8,-0.0,0,1e-9,.1,.2,.4,.5,.9,1,1.5,5,1e10]:
  add('amount_'+str(amount),[one([r(1,amount)])],[ins(1,1),rem(1)])
 add('same_type_sum',[one([r(1,.1)]),one([r(1,.2)],2),one([r(1,-.3)],4)],[ins(1,1),ins(2,2),ins(3,3),rem(2),rem(1),rem(3)])
 add('ordered_duplicate_descriptors',[one([r(1,.1),r(1,.2),r(2,.4),r(1,-.3)])],[ins(1,1),rem(1)])
 add('duplicate_instance',[one([r(1,.2)])],[ins(1,1),ins(2,1,0x10002),rem(1),rem(1,2)])
 add('disabled_removal',[one([r(1,.2)])],[ins(1,1),ins(2,1,0x10002),rem(1,2),rem(1)])
 add('mixed_order',[one([f(11),r(1,.2),n('Add',3,.1),r(2,.8),n('Multiply',19,.25)]),one([r(1,.3),f(9)],2)],[ins(1,1),ins(2,2),rem(1),rem(2)])
 add('clamp_without_discarding_raw',[one([r(1,.9),r(1,.8)]),one([r(2,-5)],2)],[ins(1,1),ins(2,2),rem(1),rem(2)],.125)
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'name':c['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):raw=base.IMAGE.at(a,n);assert raw==base.ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 d={'schema':1,'original_sha256':base.pe.ORIGINAL_SHA,'v14_sha256':base.pe.ACCEPTED_SHA,'producer_sha256':H(Path(__file__).read_bytes()),'dependencies':{str(Path(flags.__file__).relative_to(ROOT)):H(Path(flags.__file__).read_bytes()),str(Path(base.__file__).relative_to(ROOT)):H(Path(base.__file__).read_bytes())},'cases':rows,'visited_windows':windows,'scope':'Full kind5 502440/5020A0/4FE7B0 plus581630/581810 insertion/removal and mixed descriptor order. No resistance arithmetic hooks. Heap/actor numeric or Flag consequence callbacks remain explicit same seams as property_flags oracle. Resist entry+4 and+8 captured exactly; entry persists after all sources removed. Finite inputs/successful allocation domain.'}
 p=HERE/'native-resistance.json';p.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'sequences':len(rows),'transitions':sum(len(r['output'])for r in rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
