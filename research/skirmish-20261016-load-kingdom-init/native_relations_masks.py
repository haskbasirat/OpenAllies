"""Native Kingdom+138 relations and both masks with real virtual dispatch.

All tables/major slots are explicit fixture inputs. No relation, iterator or
mask callback is replaced. Original/v14 code executes separately at all PCs.
"""
from pathlib import Path
import sys,json,hashlib,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
ORIGINAL=nm.original

def inputs():
 out=[];rng=random.Random(4615250)
 for i in range(16):
  teams=[dict(id=101+j,index=j,kingdoms=[rng.randrange(4) for _ in range(3)],teams=[rng.randrange(4) for _ in range(2)]) for j in range(2)]
  kingdoms=[dict(id=201+j,index=j,major=rng.choice([j,j,16+j,31,32,0xffffffff]),minor=rng.choice([0,0,1]),
      team=[None,101,102][(i+j)%3],kingdoms=[rng.randrange(4) for _ in range(3)],teams=[rng.randrange(4) for _ in range(2)],
      word228=rng.getrandbits(32),word22c=rng.getrandbits(32)) for j in range(3)]
  majors=[201,None,202,203] if i%2 else [None,201,202,None,203,None]
  out.append(dict(name='tables_'+str(i),teams=teams,kingdoms=kingdoms,majors=majors))
 return out

def run(c,image,pc):
 nm.original=ORIGINAL if image=='original' else nm.modified;m=nm.Machine(pc)
 world=m.alloc(0x200);m.w(0xb36534,world)
 ids={v['id']:m.alloc(0x400) for v in c['kingdoms']+c['teams']}
 for row in c['teams']:
  p=ids[row['id']];m.w(p+0x38,row['index']);m.w(p+0x4c,m.array(row['kingdoms']));m.w(p+0x58,m.array(row['teams']))
 for row in c['kingdoms']:
  p=ids[row['id']];m.w(p,0xa299d8);m.w(p+0x188,row['index']);m.w(p+0x18c,row['major']);m.w(p+0x190,row['minor'])
  m.w(p+0x1ec,ids[row['team']] if row['team'] is not None else 0)
  m.w(p+0x1d0,m.array(row['kingdoms']));m.w(p+0x1dc,m.array(row['teams']))
  m.w(p+0x228,row['word228']);m.w(p+0x22c,row['word22c'])
 m.w(world+0x154,m.array([ids[v] if v is not None else 0 for v in c['majors']]));m.w(world+0x158,len(c['majors']))
 assert m.r(0xa299d8+0x138)==0x4e5250 and m.r(0xa299d8+0x144)==0x4e52d0
 rows=[]
 for current in c['kingdoms']:
  p=ids[current['id']]
  relations=[dict(other=other['id'],relation=m.call(0x4e52d0,p,[ids[other['id']]]),
      related=bool(m.call(0x4e5250,p,[ids[other['id']]]))) for other in c['kingdoms']]
  m.call(0x54eba0,p+0x228,[p]);after228=m.r(p+0x228)
  m.call(0x559600,p+0x22c,[p]);after22c=m.r(p+0x22c)
  rows.append(dict(owner=current['id'],queries=relations,word228=after228,word22c=after22c))
 return rows,m.visited

def main():
 rows=[];pins={}
 for c in inputs():
  for pc in nm.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   for pp in (pa,pb):
    for va,raw in pp.items():
     assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw;pins[va]=raw
   rows.append(dict(input=c,precision=pc,output=a))
 doc=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_runs=len(rows)*2,cases=rows,
  instructions=[dict(va=hex(va),bytes=raw.hex()) for va,raw in sorted(pins.items())])
 path=HERE/'native-relations-masks.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(cases=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
