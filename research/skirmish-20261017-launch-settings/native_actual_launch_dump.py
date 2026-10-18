"""Native76D6C0 replay of actual Rust source-bound launch allocations.
Inputs are immutable test captures, not a production raw constructor. Each
map's same Header, selectedLoad, four fresh sibling objects/current pairs,
Data+C ordinal fields and App state are built as explicit native storage.
Real diagnostics execute; logger/CString transport seams as parent fixture.
"""
from pathlib import Path
import sys,json,hashlib,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from native_launch_dump import DumpMachine,h
sys.path.insert(0,str(ROOT/'research/phase8-net'));from native_lobby import base_data

def run(image,pc,capture):
 m=DumpMachine(image,pc);data=base_data();regs=copy.deepcopy(capture['registries'])
 for cfg in regs.values():cfg['kinds']={str(i):v for i,v in enumerate(cfg['kinds'])}
 data['registries'].update(regs);m.setup_data(data)
 c=copy.deepcopy(capture['native']);c['mode']=c.pop('map_type');c['field_0c']=c.pop('launch_type');c['state']=c.pop('app');c['map_default_flag']=True
 c['scenario']=c['map_scenario'].pop('source');c['random_map']=c['map_random'].pop('source');c['rmc_settings']=dict(field_0=0,list_4=[],list_10=[],field_1c=0)
 out=m.run(c);assert out['events']==capture['events'], next(((i,a,b)for i,(a,b)in enumerate(zip(out['events'],capture['events']))if a!=b),(len(out['events']),len(capture['events'])))
 return out,m.visited,m.intercepted

def main():
 capture=HERE/'actual-dump-inputs.json';inputs=json.loads(capture.read_text());rows=[];visited={};seams=set()
 for pc in h.PRECISION:
  for c in inputs:
   a,pa,sa=run('original',pc,c);b,pb,sb=run('v14',pc,c);assert a==b
   visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(map=c['map'],precision=pc,output=a))
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 data=(json.dumps(dict(schema=1,scope=__doc__,capture_sha256=hashlib.sha256(capture.read_bytes()).hexdigest(),producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,instructions=pins,seams=[hex(a)for a in sorted(seams)]),indent=1)+'\n').encode();path=HERE/'native-actual-launch-dump.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
