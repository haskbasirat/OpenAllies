"""Execute original vtable+20 ground component requirement getters."""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));from actor_states import Machine,P,V,H

def main():
 data=json.loads((HERE/'component-dispatch.json').read_text());m=Machine();rows=[]
 for r in data['rows']:
  if r['slot'] not in [0,2,3,4,5,6,7,8,9,17,22]:continue
  entry=int(r['vtable_methods']['20'],16);m.invoke(entry,m.thing)
  from unicorn.x86_const import UC_X86_REG_EAX
  value=m.u.reg_read(UC_X86_REG_EAX)
  rows.append({'slot':r['slot'],'condition':r['condition'],'vtable':r['vtable'],'entry':hex(entry),'result':value})
 for a,b in m.observed.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'cases':rows,'source_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(m.observed.items())],'scope':'Native constant ground component requirement getters and three concrete Mind variants. Original constructor output proves each vtable selection; no runtime instance pointer dereferences in these getters.'}
 p=HERE/'native-component-requirements.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
