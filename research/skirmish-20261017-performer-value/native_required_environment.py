"""Native57E430 stops before first/second reached96D890 body. Original/v14
execute the actual cached prefix with no successful leaf seam; the separate
unbound-cache variant uses an explicit named-SVar lookup transport seam.
Actual captured input
and separate explicit unbound-cache variant prove reset/SVar binding prefixes;
this is a required-service stop, not executed CRT behavior or a crash recovery.
"""
from pathlib import Path
import argparse,hashlib,json,copy
import native_actual_performer as native
HERE=Path(__file__).resolve().parent

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args()
 capture=HERE/'actual-performer-values.json';doc=json.loads(capture.read_bytes());rows=[];pins={}
 for row in doc['environment_errors']:
  for variant in ['actual_cached','explicit_unbound_cached_pointer_guards']:
   case=copy.deepcopy(row['input'])
   if variant!='actual_cached':case['cached_svar_guards']=[False,False]
   for stop in [1,2]:
    a,pa=native.run('original',row['precision'],case,stop);b,pb=native.run('v14',row['precision'],case,stop)
    assert a==b and pa==pb and a['value']==row['output']==0
    assert a['cached_svar_guards']==[True,case['cached_svar_guards'][1]]
    rows.append(dict(precision=row['precision'],variant=variant,input=case,output=a));pins.update(pa)
 assert native.h.ORIGINAL.at(0x96D890,8)==native.h.ACCEPTED.at(0x96D890,8)
 result=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=2*len(rows),cases=rows,
  executed_instruction_pins={f'{a:08x}':b for a,b in sorted(pins.items())},
  stopped_boundary_pins={'0096d890':native.h.ORIGINAL.at(0x96D890,8).hex()},
  capture_sha256=hashlib.sha256(capture.read_bytes()).hexdigest(),
  limitations=['Explicit false guard variant may reach typed named-SVar lookup transport seam; actual captured cache guards are both true.', 'No successful CRT/body interception; stopped boundary instruction is not counted executed.'])
 #Actual cached rows have no successful seams before the required CRT entry.
 path=HERE/'native-required-environment.json'
 if args.write:
  with path.open('x')as f:json.dump(result,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_bytes())==result
 print(json.dumps(dict(paired_cases=len(rows),native_executions=2*len(rows),executed_pins=len(pins),stopped_boundaries=1,sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
