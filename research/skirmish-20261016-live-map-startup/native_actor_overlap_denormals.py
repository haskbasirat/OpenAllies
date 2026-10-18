"""Additional exact raw binary32 expiry comparisons, including denormals.
Uses the same bounded native-list runner; no spatial query behavior is claimed.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
import native_actor_overlap_list as base

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');args=ap.parse_args()
 entries=[[i+0x10001,b] for i,b in enumerate([0,0x80000000,1,0x80000001,0x7fffff,0x807fffff,0x800000,0x80800000,0x7f800000,0xff800000,0x7f800001,0x7fc00001])]
 cases=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for time in [1,0x80000001,0x7fffff,0x807fffff,0x800000,0x80800000,0x7f800001,0x7fc00001]:
   for stop in [False,True]:
    row=dict(flags=1<<10,state_low=0,state_high=0,time_bits=time,entries=entries,discovered=[0x20002],stop_before_query=stop)
    a,pa=base.run(base.nm.P,pc,row);b,pb=base.run(base.nm.V,pc,row);assert a==b and pa==pb
    pins.update(pa);cases.append(dict(precision=pc,input=row,output=a))
 out=dict(schema=1,scope=__doc__,paired_case_count=len(cases),native_execution_count=len(cases)*2,cases=cases,instruction_pins=pins,source_images=[dict(name=name,sha256=hashlib.sha256(image.b).hexdigest()) for name,image in [('original',base.nm.P),('v14',base.nm.V)]])
 target=Path(__file__).with_name('native-actor-overlap-denormals.json')
 if args.write:
  assert not target.exists();target.write_text(json.dumps(out,indent=2)+'\n')
 else:assert json.loads(target.read_text())==out
 print(json.dumps(dict(paired_cases=len(cases),native_executions=len(cases)*2,pins=len(pins),sha256=hashlib.sha256(target.read_bytes()).hexdigest())))
if __name__=='__main__':main()
