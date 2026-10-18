"""Actual778F20(false)/778730 growable Session+4/+8/+C queue, original/v14.
CString successful allocator seams are inherited and named; no callbacks or
native allocation/exception/reentry parity claim. No original files written.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges

def main():
 rows=[];visited={}
 for image in ('original','v14'):
  for p in PC:
   m=BootstrapMachine(image);m.fpu(p);m.begin(1,list(range(16)),0)
   queue=m.alloc(4*12)
   for i in range(12):m.cs(queue+i*4,'')
   m.w(m.S+4,queue);m.w(m.S+8,0);m.w(m.S+12,4)
   triples=[['Kingdom0','Profile0','Profile0'],['','\u03a9',''],['A\x00B','Second','Third']]
   triples += [[f'Kingdom{i}',f'Name{i}',f'Profile{i}']for i in range(3,9)]
   for index,triple in enumerate(triples):
    slots=m.alloc(12)
    for j,text in enumerate(triple):m.cs(slots+4*j,text)
    m.call(0x778f20,[m.r(slots),m.r(slots+4),m.r(slots+8),0],this=m.S,callee_pop=16)
    assert m.r(m.S+8)==index+1
    array=m.r(m.S+4)
    actual=[[m.rs(array+i*12+j*4)for j in range(3)]for i in range(index+1)]
    assert actual==[[[ord(c)for c in text]for text in row]for row in triples[:index+1]],(image,p,index,actual)
    rows.append({'image':image,'precision':p,'appended':[[ord(c)for c in text]for text in triple],
     'count':index+1,'native_capacity':m.r(m.S+12),'triples':actual})
   visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'Actual778F20 false branch plus778730 and real growth7756E0 on Session4/8/C; successful named CString/allocator seams only',
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),
  'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-setup-ai.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'SetupAI append rows')
if __name__=='__main__':main()
