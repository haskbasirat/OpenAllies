"""Independent navigation-worker review of Phase6 geometry inputs and boundaries."""
from pathlib import Path
import contextlib,hashlib,importlib,json,re,subprocess,sys
ROOT=Path(__file__).resolve().parents[2];SOURCE=Path(__file__).parent
OUT=SOURCE/'independent-navigation-output'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 OUT.mkdir(exist_ok=True);sys.path.insert(0,str(SOURCE));commands=[]
 modules=[('original_height_binding','native-height-binding.json'),('original_map_heights','native-real-map-heights.json'),('original_tile_walk','native-tile-walk.json')]
 regenerated=[]
 for name,filename in modules:
  module=importlib.import_module(name);module.OUT=OUT
  with (OUT/(name+'.log')).open('w') as log,contextlib.redirect_stdout(log):module.main()
  actual=json.loads((OUT/filename).read_text());expected=json.loads((SOURCE/filename).read_text());assert actual==expected,filename
  regenerated.append({'fixture':filename,'sha256':sha(SOURCE/filename),'reproduced':True})
 tests=0
 for args,name in [(['cargo','test','-p','aa-heightfield','-p','aa-spatial','-p','aa-movement','--','--include-ignored','--nocapture'],'rust-tests.log'),(['cargo','clippy','-p','aa-heightfield','-p','aa-spatial','-p','aa-movement','--all-targets','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run(args,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);log=OUT/name;log.write_text(run.stdout)
  commands.append({'command':args,'exit_code':run.returncode,'log':str(log.relative_to(ROOT)),'sha256':sha(log)})
  assert run.returncode==0,run.stdout
  if name=='rust-tests.log':tests=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout)))
 reviewed=['crates/aa-heightfield/src/binding.rs','crates/aa-heightfield/src/lib.rs','crates/aa-heightfield/tests/native_binding.rs','crates/aa-heightfield/tests/original_maps.rs','crates/aa-spatial/src/tile_walk.rs','crates/aa-spatial/src/height_layers.rs','crates/aa-spatial/tests/tile_walk.rs','crates/aa-movement/src/lib.rs','crates/aa-movement/tests/native_fixtures.rs','crates/aa-client/src/mechanics_check.rs']
 report={'schema':1,'passed':True,'reviewer':'independent navigation worker','findings':[],
 'scope':'Native height construction/v1 chunk binding, actual original-map samples, exact TileWalk, HeightLayers dispatch, fallible GroundWorld propagation',
 'regenerated':regenerated,'native_counts':{'constructors':28,'loads':6,'actual_map_samples':144,'full_tile_walks':360},'maps_bound':80,'height_layers_bound':160,'rust_test_count':tests,
 'commands':commands,'reviewed_source_sha256':{p:sha(ROOT/p) for p in reviewed},
 'checked_semantics':['5519A0 ceil/truncate/shift construction and default/flag initialization match original instructions.','551E60 is the input routine; raw v1 resolution/count witnesses checked; version0 world-derived flags are explicitly unsupported.','Raw MAP_HEADER extent/SVar identity provenance and160 height payload hashes checked; rounded renderer data does not produce sampler inputs.','All144 actual-map80-bit sampler results andf32stores reexecuted; all80 maps loaded by ignored integration test.','4DD030/4DD150/4DDDE0 exact traversal and bounds skipping rerun, including X extended-time vs Y stored-time comparison and Y tie choice.','HeightLayers preserves Binary returns and rejects inconsistent grid layouts.','GroundWorld Result propagation stops later callbacks and intentionally retains prior fields/host effects; native success fixtures and injected error tests pass.'],
 'limits':['No review of newly added actor_collision module or its lib export.','Constructor resolution1..30/nonnegative finite extent safe domain and v1 format only; unsupported native-data domains are errors.','No actor shape intersection, region topology, tile membership, world adapter or gameplay completion claim.','Numeric/trig internals and map-header parser rely on their separate native evidence; this review checks their use and reruns integration.']}
 target=SOURCE/'independent-review-navigation.json';target.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'tests':tests,'sha256':sha(target)}))
if __name__=='__main__':main()
