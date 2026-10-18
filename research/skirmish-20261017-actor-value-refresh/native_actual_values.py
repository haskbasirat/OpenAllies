"""Replay captured actual tutorial02 Denizen and Organization refresh inputs.
The original/modified instructions execute with no successful function seams.
Captured source/runtime scalar relationships are rebuilt as isolated native
fixture memory; this is boundary parity, not an original game initialization.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));import native_denizen_value as denizen
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-saved-org-combat'));import native_saved_org_combat as org
from unicorn import UC_HOOK_CODE

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 source=HERE/'actual-denizen-value.json';captured=json.loads(source.read_bytes());rows=[];pins={}
 for row in captured['rows']:
  a,pa=denizen.run(denizen.nm.P,row['precision'],row['input']);b,pb=denizen.run(denizen.nm.V,row['precision'],row['input'])
  assert a==b==row['output'] and pa==pb
  rows.append(dict(kind='Denizen',precision=row['precision'],input=row['input'],output=a));pins.update(pa)
 for row in captured['organization']['rows']:
  results=[];local=[]
  for image in ['original','v14']:
   m=org.Saved(dict(org.MODES)[row['precision']],row['input'],image);assert not m.unbound and not m.seams
   p={}
   def observe(_,address,size,__):
    if org.ORIGINAL.base<=address<org.ORIGINAL.base+org.ORIGINAL.size:
     raw=org.ORIGINAL.at(address,size);assert raw==org.ACCEPTED.at(address,size)
     assert address not in m.seams
     p[f'{address:08x}']=raw.hex()
   m.uc.hook_add(UC_HOOK_CODE,observe);result=m.run();assert result==row['output'];results.append(result);local.append(p)
  assert results[0]==results[1] and local[0]==local[1]
  rows.append(dict(kind='Organization',precision=row['precision'],input=row['input'],output=results[0]));pins.update(local[0])
 result=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=len(rows)*2,successful_seams=[],cases=rows,executed_instruction_pins=pins,capture_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),source_images=[dict(name=n,sha256=hashlib.sha256(p.b).hexdigest())for n,p in [('original',denizen.nm.P),('v14',denizen.nm.V)]])
 path=HERE/'native-actual-values.json'
 if args.write:
  with path.open('x')as f:json.dump(result,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_bytes())==result
 print(json.dumps(dict(paired_cases=len(rows),native_executions=len(rows)*2,executed_pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
