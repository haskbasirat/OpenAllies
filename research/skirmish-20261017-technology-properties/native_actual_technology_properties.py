"""Actual source authored order -> native4C4CF0 ->45ECA0 ->runtime Property tokens.
Canonical Property lookup remains an explicitly captured source identity seam.
The actual saved2697 source and every positive installed list are compared,
without authorizing4E3AD0 property effects or claiming full ActorData loading.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_technology_properties as core
def main():
 authored_path=HERE/'actual-authored-source.json';runtime_path=HERE/'actual-source-input.json'
 authored=json.loads(authored_path.read_text());runtime=json.loads(runtime_path.read_text());assert authored['source_identities']==runtime['source_identities']
 rows=[];pins={}
 for current in runtime['rows']:
  raw=next((v for v in authored['rows']if v['id']==current['id']),None)
  if raw is None:
   # The standalone pure-source capture exhausts all ActorData Structure
   #TechnologyProperty lists and retains every nonempty one.
   assert current['head_first_ids']==[];names=[];lookups={}
  else:
   assert current['source']==raw['source'];names=raw['authored_requests'];lookups=raw['canonical_lookups']
  for pc in core.PC:
   a,pa=core.run(names,'original',pc,lookups);b,pb=core.run(names,'v14',pc,lookups);assert a==b and a['throw']is None
   canonical=[lookups[name]['id']for name in a['values']];assert canonical==current['head_first_ids']
   assert len(current['tokens'])==len(canonical)
   pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,id=current['id'],source=current['source'],authored_requests=names,canonical_lookups=lookups,native=a,head_first_ids=canonical,runtime_tokens=current['tokens']))
 for va,raw in pins.items():assert core.ORIGINAL.range(va,len(raw))==core.V14.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,authored_sha256=hashlib.sha256(authored_path.read_bytes()).hexdigest(),runtime_sha256=hashlib.sha256(runtime_path.read_bytes()).hexdigest(),rows=rows,native_executions=len(rows)*2,instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())])
 path=HERE/'native-actual-technology-properties.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
