"""Null parent terrain after a prior nonnull F8; native diagnostic boundary."""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
import native_actor_terrain_refresh as base

def main():
 ap=argparse.ArgumentParser();ap.add_argument('--write',action='store_true');args=ap.parse_args()
 rows=[];pins={};stops={}
 for pc in ['Single24','Double53','Extended64']:
  for old in [0,1]:
   c=dict(flags=0,state_low=0,state_high=0,tile=1,old=old,organization=False,weapon=True,properties=[[],[]],element='active_parent',parent_terrain=None)
   a,pa,sa=base.run(base.nm.P,pc,c);b,pb,sb=base.run(base.nm.V,pc,c);assert a==b and pa==pb and sa==sb
   assert a['current_terrain'] is None and a['outcome']=='required_null_terrain_diagnostic'
   pins.update(pa);stops.update(sa);rows.append(dict(precision=pc,input=c,output=a))
 out=dict(schema=1,scope=__doc__,paired_cases=len(rows),native_executions=len(rows)*2,cases=rows,executed_instruction_pins=pins,required_boundary_entry_pins=stops)
 p=Path(__file__).with_name('native-actor-terrain-null.json')
 if args.write:assert not p.exists();p.write_text(json.dumps(out,indent=2)+'\n')
 else:assert json.loads(p.read_bytes())==out
 print(json.dumps(dict(paired_cases=len(rows),native_executions=len(rows)*2,sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
