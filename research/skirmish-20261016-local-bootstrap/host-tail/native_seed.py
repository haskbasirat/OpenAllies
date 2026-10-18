"""Whole7710E0 seed override writer, explicit old allocation poison and UI
rejection seam inherited. No host authorization/75DA40 completion claim.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent))
from native_bootstrap import BootstrapMachine,PE0,PE1,PC,proof_ranges
def main():
 rows=[];visited={}
 for image in ('original','v14'):
  m=BootstrapMachine(image);m.begin(1,list(range(16)),0)
  for p in PC:
   m.fpu(p)
   for countdown in (-1,0,1):
    for seed in (0,1,0x80000000,0xffffffff):
     m.w(m.S+0x118,countdown&0xffffffff);m.w(m.S+0x100,0xa5a5a5a5)
     m.call(0x7710e0,[seed],this=m.S,callee_pop=4)
     got=m.r(m.S+0x100);assert got==(seed if countdown<1 else 0xa5a5a5a5)
     rows.append({'image':image,'precision':p,'countdown':countdown,'supplied_seed':seed,'after':got})
  visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'Actual7710E0 complete setter, successful inherited CString/UI rejection seams;75DA40 identity/authority prefix separate',
 'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-seed.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'seed setter rows')
if __name__=='__main__':main()
