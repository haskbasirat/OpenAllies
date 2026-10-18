"""Native wildcard filename sorting/iteration and sequential easy boolean SVars."""
from pathlib import Path
import sys,json,struct,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from probe_lists import Machine as Base
from native_ground_definitions import P,V,H
from unicorn.x86_const import *
class Machine(Base):
 def hook(self,u,pc,n,z):
  if pc in[0x404f60,0x404ed0,0x404ea0]:self.ret()
  elif pc==0x5b3b10:
   out=self.arg();self.w(out,self.string(list(map(ord,'fixture'))));self.ret(out,pop=4)
  else:super().hook(u,pc,n,z)
 def make(self,value):
  node=self.node('SVars');parser=self.alloc(0x80);token=self.alloc(0x30);self.w(parser,node);self.w(parser+0x40,self.string(list(map(ord,'UnitSupplyEnabled'))));self.w(parser+0x38,5);self.w(token+0x18,self.string(list(map(ord,value))));self.w(token+0x10,7)
  self.invoke(0x5ba010,parser,[token]);return node
 def settings(self,sequence):
  settings=self.alloc(0x80);self.invoke(0x4b2ce0,settings,[]);out=[]
  for value in sequence:
   self.invoke(0x4b3c10,settings,[self.make(value)]);count=self.r(settings+12);array=self.r(settings+8);var=self.r(array);out.append({'value':self.r(var)&255,'raw_word':self.r(var),'kind':self.r(var+36),'flags':self.r(var+40),'count':count})
  return out
 def sorted_names(self,names):
  array=self.alloc(max(len(names),1)*8)
  for i,name in enumerate(names):self.w(array+i*8,self.string(list(map(ord,name))))
  self.w(self.stack,self.stop)
  for i,value in enumerate([array,len(names),8,0x859a10]):self.w(self.stack+4+i*4,value)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.emu_start(0x96ce20,self.stop,count=1000000)
  enum=self.alloc(0x288);self.w(enum+0x278,array);self.w(enum+0x27c,len(names));out=self.alloc(4);self.w(out,self.string([]));values=[]
  while self.invoke(0x85be40,enum,[out])&255:
   p=self.r(out);values.append(bytes(self.u.mem_read(p,self.r(p-12)*2)).decode('utf-16-le'))
  assert values==sorted(names)
  return values
rows=[];sorts=[];visited={}
for sequence in [('false','true'),('true','false'),('false','true','false'),('true','true')]:
 m=Machine();out=m.settings(sequence);assert [r['value']for r in out]==[int(s=='true')for s in sequence],(sequence,out);assert all(r['count']==1 and r['kind']==1 and r['flags']==0 for r in out);rows.append({'input':sequence,'states':out});visited.update(m.visited)
for names in [[],['Game/SVars_AA.tgi','Game/SVars.tgi'],['Game/SVars.tgi','Game/SVars_AA.tgi'],['B.tgi','a.tgi','A.tgi','b.tgi'],['z','Z','ä','a','Ä']]+[list(p)for p in itertools.permutations(['00.tgi','Z.tgi','A.tgi'])]:
 m=Machine();sorts.append({'input':names,'output':m.sorted_names(names)});visited.update(m.visited)
spans=[]
for a,n in sorted(visited.items()):
 if not P.base<=a<P.base+P.image_size:continue
 if spans and spans[-1][1]>=a:spans[-1][1]=max(spans[-1][1],a+n)
 else:spans.append([a,a+n])
spans += [[0x5ab590,0x5ab73c],[0x85bd30,0x85be40],[0x425e80,0x425f10],[0x48d970,0x48db60],[0x48f436,0x48f46c]]
pins=[]
for a,b in spans:
 raw=P.range(a,b-a);assert raw==V.range(a,b-a);pins.append({'start':hex(a),'end':hex(b),'sha256':H(raw)})
base=HERE.parents[2]/'reconstruction/assets/original/Data';overlay=HERE.parents[2]/'reconstruction/native-game/runtime-expanded-v14/Data'
files={}
for path in ['index_aa.lst','Game/SVars.tgi','Game/SVars_AA.tgi']:
 file=overlay/path if(overlay/path).is_file()else base/path
 files[path]={'sha256':H(file.read_bytes()),'selected_file':str(file),'relevant_lines':[{'line':i+1,'text':line}for i,line in enumerate(file.read_text(encoding='cp1252').splitlines())if 'UnitSupplyEnabled'in line or 'Game/SVars' in line]}
d={'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(P.b),'v14_sha256':H(V.b),'settings_cases':rows,'sort_cases':sorts,'source_evidence':files,'ranges':pins,'scope':'OriginalCRT qsort+859A10 comparator+85BE40 consumption, original typedBool assignment5BA010 and completeSettings4B3C10 sequentialexisting easy-variable updates. Bool/flag source declarations select same typed native boolean; arbitrary hardcoded/restore settings policies are not inferred.','seams':['File enumeration inputs supplied; wildcard match/path-resolution behavior is statically inspected, not OS filesystem traversal emulated.','Successful allocation/string pool; logging/source filename formatting are leaves. Native Settingsctor/hashlookup/new-var/add/overwrite and typedbool field construction/insertion execute.','Index source line and exact sourcefiles pinned. UnitSupplyEnabled false then true is a value overwrite retaining one variable, type1flags0.']};(HERE/'native-settings-order.json').write_text(json.dumps(d,indent=2));print('passed',len(rows),len(sorts),H((HERE/'native-settings-order.json').read_bytes()))
