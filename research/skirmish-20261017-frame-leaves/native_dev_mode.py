"""Actual UVar62 flags2 ordinary4B3190 assignment denial through4B29B0.
No game-code seam; before/default and requested raw bytes are explicit inputs.
Separate complete213-registration proof establishes ctor0/0 and flags2.
Positive source/command-line bypass writes remain outside this slice.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-view-init'));import native_view_bool_assignment as base
def main():
 rows=[];base.h.VISITED.clear()
 for pc,before,requested,dev,session in itertools.product(base.h.PRECISION,(0,128),(0,1,128,255),(0,128),((0,0),(3,1),(1,1))):
  c={'before':before,'requested':requested,'flags':2,'dev':dev,'e4':session[0],'f4':session[1]}
  a=base.run('original',pc,c);b=base.run('v14',pc,c);assert a==b
  assert a=={'allowed':0,'current':before,'default':77}
  rows.append({'precision':pc,'input':c,'output':a})
 pins=[]
 for a,b in sorted(base.h.VISITED.items()):
  assert bytes.fromhex(b)==base.ORIGINAL.range(a,len(bytes.fromhex(b)))==base.h.V.range(a,len(bytes.fromhex(b)))
  pins.append({'va':hex(a),'bytes':b})
 doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
      'original_sha256':base.h.ORIGINAL_SHA,'v14_sha256':base.h.V14_SHA,'cases':rows,'executed_pins':pins}
 path=HERE/'native-dev-mode.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8',newline='\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':2*len(rows),'genuine_executed_pins':len(pins)}))
if __name__=='__main__':main()
