"""Bind tested actual-map topology and region routes without claiming live world."""
from pathlib import Path
import hashlib,json,subprocess
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 commands=[]
 for command,name in [(['cargo','test','-p','aa-navigation','--test','topology_native','--','--include-ignored','--nocapture'],'rust-tests.log'),(['cargo','clippy','-p','aa-navigation','--all-targets','--no-deps','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run(command,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);log=OUT/name;log.write_bytes(run.stdout);assert run.returncode==0,run.stdout.decode(errors='replace')
  commands.append({'command':command,'exit_code':run.returncode,'log':name,'sha256':sha(log)})
 maps=json.loads((OUT/'native-topology.json').read_text());small=json.loads((OUT/'synthetic-topology.json').read_text());routes=json.loads((OUT/'native-map-routes.json').read_text())
 for doc,producer in [(maps,'topology_oracle.py'),(small,'synthetic_topology.py'),(routes,'routes_oracle.py')]:assert doc['passed']and doc['producer_sha256']==sha(OUT/producer)
 assert len(maps['maps'])==80 and len(small['fixtures'])==21 and sum(len(c['routes'])for c in routes['fixtures'])==36
 assert routes['topology_fixture_sha256']==sha(OUT/'native-topology.json')
 source=['crates/aa-navigation/src/topology.rs','crates/aa-navigation/tests/topology_native.rs','crates/aa-navigation/src/lib.rs','crates/aa-navigation/Cargo.toml']
 files=['README.md','topology_oracle.py','synthetic_topology.py','routes_oracle.py','inspect_native.py','record_evidence.py','native-topology.json','synthetic-topology.json','native-map-routes.json','native-provenance.json']
 deps=['crates/aa-navigation/src/region_search.rs','crates/aa-navigation/src/region.rs','crates/aa-numeric/src/lib.rs','crates/aa-mapworld/src/terrain.rs','research/phase6-spatial/native-real-map-terrain.json','research/phase6-mapheader/native_header.py','research/phase6-mapheader/inspect_header.py','research/phase4-rng/original_rng.py','research/phase6-terrain/terrain-render-metadata.json','research/engine-baseline/terrain-variation-metadata.json']
 report={'schema':1,'passed':True,'scope':'Original saved static topology→tile/region lookup→concrete flag0 route adapter','original_maps':80,'original_regions':sum(len(r['regions'])for r in maps['maps'].values()),'synthetic_native_cases':21,'source_native_route_searches':36,'rust_tests':3,'commands':commands,
 'source_sha256':{p:sha(ROOT/p)for p in source}|{str((OUT/p).relative_to(ROOT)).replace('\\','/'):sha(OUT/p)for p in files},'dependency_sha256':{p:sha(ROOT/p)for p in deps},
 'limits':['Fresh saved-topology binding only; native old-world version<17 static rebuild rejected.','prepare_graph returnsready=false until runtime supplies actual completed SAI startup state.','Native flag1 route tables, bottleneck analysis, dynamic topology changes and full SAI startup remain separate.','The actual-map region route oracle explicitly sets SAI ready after adjacency; no playable world or tile-collision Solve composition claimed.','Malformed/null native memory domains return errors; valid source extents and native finite arithmetic are the supported domain.'],
 'independent_review':'Separate peer report; producer does not self-approve.'}
 target=OUT/'topology-evidence.json';target.write_text(json.dumps(report,indent=2)+'\n');print({'passed':True,'sha256':sha(target),'maps':80,'regions':report['original_regions']})
if __name__=='__main__':main()
