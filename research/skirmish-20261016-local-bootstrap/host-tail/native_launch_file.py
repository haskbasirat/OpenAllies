"""Actual771180 file CString writer; successful copied-CString/UI seams.
NativeLaunchSettings constructor supplies the destination empty CString.
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
    for text in ('','Maps/(2v2)Berlin.rmp','A\x00B'):
     m.cs(m.S+0x5c,'old file');m.w(m.S+0x118,countdown&0xffffffff)
     supplied=m.alloc(4);m.cs(supplied,text)
     m.call(0x771180,[m.r(supplied)],this=m.S,callee_pop=4)
     got=m.rs(m.S+0x5c);assert got==[ord(c)for c in(text if countdown<1 else'old file')]
     rows.append({'image':image,'precision':p,'countdown':countdown,'input':[ord(c)for c in text],'after':got})
  visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'Actual771180 whole setter; successful named CString/UI seams. No filesystem/ScenarioFind or original UIdefault claim.',
 'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-launch-file.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'launch file rows')
if __name__=='__main__':main()
