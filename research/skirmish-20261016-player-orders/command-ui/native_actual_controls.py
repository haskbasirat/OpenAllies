"""Actual original/v14 code-predefined commands plus shipped inherited ActorCommand UI controls. Both PE images map independently; native enum/manager/registration and parser/getters/control fragment execute, no command descriptor or control values supplied. Earlier presentation loader resources remain outside scope."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/resume-20261016-actor-commands'))
import native_actor_commands as c
import trace_registry as base
P=c.P;V=c.V;H=lambda b:hashlib.sha256(b).hexdigest()
def main():
 rows=[];visited={}
 for image,label in [(P,'original'),(V,'v14')]:
  base.P=image
  for pc in c.CW:
   m=c.Commands();m.setup(pc);definitions=m.definitions();flags=m.load_flag_nodes();assert len(definitions)==39 and len(flags)==39
   rows.append({'image':label,'precision':pc,'definitions':definitions,'authored':flags});visited.update(m.visited);print(label,pc,len(flags),flush=True)
 base.P=P
 pins=[]
 for va,n in sorted(visited.items()):
  if P.base<=va<P.base+P.image_size:
   raw=P.range(va,n);assert raw==V.range(va,n),hex(va);pins.append({'va':hex(va),'bytes':raw.hex()})
 p=HERE/'native-actual-controls.json';p.open('x').write(json.dumps({'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'source_files':[{'path':f,'sha256':H((c.DATA/f).read_bytes())}for f in[c.TEMPLATE]+c.FILES],'runs':rows,'instructions':pins},indent=1)+'\n');print(json.dumps({'runs':len(rows),'controls':sum(len(r['authored'])for r in rows),'pins':len(pins),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
