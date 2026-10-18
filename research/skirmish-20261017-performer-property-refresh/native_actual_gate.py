"""Native57F1E0 gate prefixes for explicit actual-source-constructed inputs.
These compare committed enabled bytes to the real Rust composition. Native
4D1350 is a required stop here; its body and subsequent57E430 are NOT claimed
executed by this gate-prefix oracle. Rust executes both real concrete services.
"""
from pathlib import Path
import argparse,json,hashlib
import native_performer_property as native
HERE=Path(__file__).resolve().parent

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 source=HERE/'actual-property-refresh.json';actual=json.loads(source.read_bytes());rows=[];pins={}
 for row in actual['rows']:
  a,pa,stop=native.run('original',row['precision'],row['input']);b,pb,s=native.run('v14',row['precision'],row['input'])
  assert a==b and pa==pb and stop==s and a['enabled']==[int(v)for v in row['enabled_after']]
  assert a['required']=='4D1350 actions'
  rows.append(dict(owner=row['owner'],phase=row['phase'],precision=row['precision'],input=row['input'],output=a,rust_full_result=row['result']));pins.update(pa)
 result=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=2*len(rows),cases=rows,successful_seams=[],executed_instruction_pins={f'{a:08x}':b for a,b in sorted(pins.items())},stopped_boundary_pins={'004d1350':native.h.ORIGINAL.at(0x4D1350,8).hex()},capture_sha256=hashlib.sha256(source.read_bytes()).hexdigest())
 path=HERE/'native-actual-gate.json'
 if args.write:
  with path.open('x')as f:json.dump(result,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_bytes())==result
 print(json.dumps(dict(paired_cases=len(rows),native_executions=2*len(rows),executed_pins=len(pins),required_boundaries=1,sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
