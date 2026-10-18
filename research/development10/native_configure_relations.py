"""Whole native4E0F90 over original live relation tables and major vectors.
No successful child transport seams; actual+138 dispatch and mask stores execute.
Fixture table generation is reused from the earlier accepted relation proof.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'skirmish-20261016-load-kingdom-init'))
import native_relations_masks as old
nm=old.nm;ORIGINAL=old.ORIGINAL

def run(c,image,pc):
 nm.original=ORIGINAL if image=='original' else nm.modified
 m=nm.Machine(pc);world=m.alloc(0x200);m.w(0xb36534,world)
 ids={v['id']:m.alloc(0x400) for v in c['kingdoms']+c['teams']}
 for r in c['teams']:
  p=ids[r['id']];m.w(p+0x38,r['index']);m.w(p+0x4c,m.array(r['kingdoms']));m.w(p+0x58,m.array(r['teams']))
 for r in c['kingdoms']:
  p=ids[r['id']];m.w(p,0xa299d8)
  for key,off in [('index',0x188),('major',0x18c),('minor',0x190),('word228',0x228),('word22c',0x22c)]:m.w(p+off,r[key])
  m.w(p+0x1ec,ids[r['team']] if r['team'] is not None else 0)
  m.w(p+0x1d0,m.array(r['kingdoms']));m.w(p+0x1dc,m.array(r['teams']))
 m.w(world+0x154,m.array([ids[v] if v is not None else 0 for v in c['majors']]))
 m.w(world+0x158,len(c['majors']))
 assert m.r(0xa299d8+0x138)==0x4e5250
 rows=[]
 for r in c['kingdoms']:
  p=ids[r['id']];m.call(0x4e0f90,p)
  rows.append(dict(owner=r['id'],word228=m.r(p+0x228),word22c=m.r(p+0x22c)))
 return rows,m.visited

def main():
 rows=[];pins={}
 for c in old.inputs():
  for pc in nm.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   for pp in (pa,pb):
    for va,raw in pp.items():
     assert ORIGINAL.range(va,len(raw))==nm.modified.range(va,len(raw))==raw;pins[va]=raw
   rows.append(dict(input=c,precision=pc,output=a))
 doc=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_runs=len(rows)*2,cases=rows,
  instructions=[dict(va=hex(v),bytes=b.hex()) for v,b in sorted(pins.items())])
 path=HERE/'native-configure-relations.json'
 if '--write' in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(cases=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
