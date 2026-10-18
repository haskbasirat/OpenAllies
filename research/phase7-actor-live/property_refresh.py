"""Original581420 refresh captures count and includes disabled duplicates."""
from pathlib import Path
import sys,json,copy
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import resistance
base=resistance.base;H=resistance.H
class Machine(resistance.Machine):
 def run_refresh(self):
  self.run();self.events=[];before=self.snapshot();self.call(0x581420,base.PROP,[self.c['attribute']]);return {'before':before,'after':self.snapshot()}
def main():
 rows=[];visited={}
 cases=list(resistance.cases())
 template=copy.deepcopy(cases[0]);mods=[]
 for i in range(25):mods.append({'Numeric':{'kind':'Add'if i<13 else'Multiply','index':i,'amount':base.bits(.1),'repair':False}})
 template['definitions']={'1':{'flags':0,'modifiers':mods+[{'Flag':{'index':11}},{'Resistance':{'damage_type':1,'amount':base.bits(.2)}}]}}
 template['operations']=[{'Insert':{'handle':i,'definition':1,'kind':0,'created':0,'source_id':0,'disabled':False}}for i in [1,2]]
 for attr in range(27):
  for empty in [False,True]:
   c=copy.deepcopy(template);c['name']=f'attribute{attr}_empty{empty}';c['attribute']=attr
   if empty:c['operations']=[]
   m=Machine(c);rows.append({'input':c,'output':m.run_refresh()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):raw=base.IMAGE.at(a,n);assert raw==base.ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 d={'schema':1,'producer_sha256':H(Path(__file__).read_bytes()),'cases':rows,'original_sha256':base.pe.ORIGINAL_SHA,'v14_sha256':base.pe.ACCEPTED_SHA,'visited_windows':windows,'scope':'Actual581420 after realmixedpropertyinsertions;Numeric actor consequence callback observed, no refresh dispatch hooks. Includesdisabledduplicateinstance andall25numericattributeindices, flags/resistance excluded bynativekindchecks, constructor-emptylist.'}
 p=HERE/'native-property-refresh.json';p.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
