"""Independent raw-header layout inspection; native reader offsets define fields."""
from pathlib import Path
import sys,struct,json,hashlib
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools'))
from map_extract import read_chunks
class Reader:
 def __init__(self,b):self.b=b;self.at=0
 def take(self,n):
  a=self.at;self.at+=n
  assert self.at<=len(self.b),(a,n,len(self.b))
  return self.b[a:self.at]
 def u32(self):return struct.unpack('<I',self.take(4))[0]
 def byte(self):return self.take(1)[0]
 def string(self):return self.take(struct.unpack('<H',self.take(2))[0]*2).decode('utf-16-le')
 def localized(self):
  # Original MAP_HEADER stream uses external localized strings (mode0); the
  # alternate stream mode with inline fallbacks is a distinct native contract.
  key=self.string();return [key,None]
 def count(self):
  n=self.u32();assert n<100000,n
  return n
def rules(r):
 rv=r.u32();assert rv==4,('rules',rv)
 mode=r.u32();game=r.string();rules=[]
 for _ in range(r.count()):
  key=r.string();enabled=r.byte();kind=r.u32();assert kind<=3,('ruletype',key,kind,r.at)
  value=r.byte() if kind==0 else r.u32();rules.append([key,enabled,kind,value])
 return {'version':rv,'mode':mode,'game':game,'values':rules}
def parameters(r):
 pv=r.u32();assert pv in (11,12,13),('parameters',pv,r.at)
 teams=[]
 for _ in range(r.count()):teams.append([r.string(),r.localized()])
 kingdoms=[]
 for _ in range(r.count()):
  k={'name':r.string(),'byte10':r.byte(),'nation':r.string(),'faction':r.string(),'color':r.string(),'handicap':r.string(),
     'label':r.localized(),'string0c':r.string() if pv>=12 else None,'string2c':r.string(),'byte29':r.byte(),'byte28':r.byte(),'string14':r.string()}
  k['choices']=[r.localized() for i in range(r.count())]
  k['definitions']=[r.string() for i in range(r.count())]
  k['string58']=r.string();k['properties']=[r.string() for i in range(r.count())]
  k['byte31']=r.byte();k['byte32']=r.byte();k['resources']=[[r.string(),r.u32()] for i in range(r.count())] if pv>=13 else []
  kingdoms.append(k)
 return {'version':pv,'teams':teams,'kingdoms':kingdoms}
def decode(b):
 r=Reader(b);version=r.u32();assert version<=7,version
 first_rules=rules(r);rules_end=r.at
 first_parameters=parameters(r);parameters_end=r.at
 words=[r.u32(),r.u32()];labels=[r.localized(),r.localized()]
 width,height=(r.u32(),r.u32()) if version>=7 else (None,None)
 metadata=[r.u32(),r.u32(),r.string(),r.string(),r.string(),r.byte() if version>=4 else None]
 paths=[r.string(),r.string()];word40=r.u32() if version>=6 else None
 more=[r.u32(),r.u32()];entries=[]
 while r.byte():entries.append([r.byte(),r.string(),r.string(),r.byte() if version>=5 else None])
 prefix_bytes=r.at;creator_class=r.string();assert creator_class=='KKC_WorldCreator',creator_class
 creator_version=r.u32();assert creator_version==3,creator_version
 creator_rules=rules(r);creator_parameters=parameters(r)
 creator_size=[r.u32(),r.u32()];biome=r.string();weather=r.string();seed=r.u32()
 return {'version':version,'rules':first_rules,'rules_end':rules_end,
         'parameters':first_parameters,'parameters_end':parameters_end,
         'size_bits':[width,height],'size':list(struct.unpack('<ff',struct.pack('<II',width,height))) if width is not None else None,
         'words0c10':words,'labels':labels,'metadata':metadata,'paths':paths,'word40':word40,'words181c':more,'entries':entries,
         'prefix_bytes':prefix_bytes,'creator_class':creator_class,'creator_version':creator_version,'creator_rules':creator_rules,
         'creator_parameters':creator_parameters,'creator_size_bits':creator_size,'biome':biome,'weather':weather,'seed':seed,
         'decoded_bytes':r.at,'remaining_bytes':len(b)-r.at,'tail_start':b[r.at:r.at+100].hex()}
if __name__=='__main__':
 rows=[]
 for p in sorted((ROOT.parent/'reconstruction/assets/original/Data/Maps').rglob('*.rmp')):
  b=read_chunks(p)['MAP_HEADER'][0]
  try:row=decode(b)
  except Exception as e:print(str(p),repr(e));raise
  rows.append({'path':str(p),'sha256':hashlib.sha256(b).hexdigest(),**row})
 (HERE/'corpus-layout.json').write_text(json.dumps(rows,indent=2)+'\n')
 print(json.dumps({'maps':len(rows),'sizes':sorted(set(tuple(r['size']) for r in rows if r['size'])),'versions':sorted(set(r['version'] for r in rows))}))
