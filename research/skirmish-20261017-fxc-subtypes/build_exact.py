"""Build exact production FX compiler slices without the unrelated World/link."""
from pathlib import Path
import json,hashlib,subprocess,os
P=Path(__file__).resolve().parent;R=P.parents[1]
source=R/'crates/aa-runtime/src/fx_graph.rs';s=source.read_text()
enum=s[s.rfind('#[derive',0,s.index('pub enum FxcKind')):s.index('/// Exact native list scope.')]
core=s[s.rfind('#[derive',0,s.index('pub struct NodeId')):s.index('/// Completed graph owner;')]
sound=R/'crates/aa-runtime/src/fx_graph_sound.rs';ss=sound.read_text();ss=ss[ss.index('pub(super) fn construct_play_sfx'):ss.index('///416370:')]
actor=R/'crates/aa-runtime/src/fx_graph_actor_state.rs';a=actor.read_text();astart=a.index('pub(super) fn construct_stop_vfx');astop=a.index('\nimpl Graph {',astart)
active=a[a.index('pub(super) fn active_fields'):a.index('#[cfg(test)]',a.index('pub(super) fn active_fields'))]
resources=R/'crates/aa-runtime/src/resource_objects.rs';rs=resources.read_text();rs=rs[:rs.index("///5A60E0's field projection.")]
program='use aa_rules::resources::Provenance;use aa_numeric::Precision;use std::collections::BTreeMap;\npub type Result<T>=std::result::Result<T,String>;\n'
program+='pub mod resource_objects{\n'+rs.replace('//!','//')+'}\n'+enum+core
program+='mod sound{use super::*;\n'+ss+'}\n'
program+='mod actor_state{use super::*;\n'+a[astart:astop]+active+'}\n'
binding=s[s.index("struct Bound<'a>"):s.index('#[path = "fx_graph_sound.rs"]')]
program+='use aa_rules::{actors::{ActorCatalog,ResolvedNode},native::{ScalarType,TypedValue},targets::TargetDomain};\n'+binding+'\n'
#The Audio lookup is the exact existing source owner; unrelated Catalog/Graph
#wrappers remain outside this focused source-compiler host.
full_sound=sound.read_text();find=full_sound[full_sound.index('pub(super) fn find_audio'):full_sound.index('#[cfg(test)]')]
program=program.replace('mod sound{use super::*;\n'+ss+'}\n','mod sound{use super::*;\n'+ss+find+'}\n')
program+='#[path="'+str(R/'crates/aa-runtime/src/fx_graph_subtypes.rs').replace('\\','/')+'"] mod subtypes;\n'
def function(text,name):
 start=text.index('fn '+name+'(');end=text.index('{',start);depth=1;end+=1
 while depth:
  depth+=(text[end]=='{')-(text[end]=='}');end+=1
 return '#[test]\n'+text[start:end]+'\n'
regression=R/'crates/aa-runtime/src/fx_graph_tests.rs';reg=regression.read_text()
program+='mod regression{\n'+reg[:reg.index('#[test]')]
for name in ['original_v14_create_vfx_graph_full_state_order_and_partial_stores_match','native_separate_lists_share_one_node_and_reference_context','request_only_native_factory_common_prefix_and_unbound_fields','full_play_object_animation_loader_matches_native_order_and_failure_prefix']:
 program+=function(reg,name)
program+='}\n'
sound_tests=R/'crates/aa-runtime/src/fx_graph_sound_tests.rs';sr=sound_tests.read_text()
sound_regression='mod regression{\n'+sr[:sr.index('#[test]')]+function(sr,'original_v14_play_sfx_full_state_order_and_partial_stores_match')+'}\n'
program=program.replace('mod sound{use super::*;\n'+ss+find+'}\n','mod sound{use super::*;\n'+ss+find+sound_regression+'}\n')
program=program.replace('"../../../research/','"'+str(R/'research').replace('\\','/')+'/')
(P/'exact_subtypes_host.rs').write_text(program,encoding='utf-8')
deps=R/'target/native-runtime/release/deps'
cmd=['rustc','--edition=2024','--test',str(P/'exact_subtypes_host.rs'),'-L','dependency='+str(deps),'-o',str(P/'exact-subtypes-tests.exe')];pins=[]
for name in ['aa_rules','aa_numeric','aa_actors','serde','serde_json']:
 f=max(deps.glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime)
 cmd+=['--extern',name+'='+str(f)];pins.append(dict(path=str(f),sha256=hashlib.sha256(f.read_bytes()).hexdigest()))
with(P/'exact-build01.log').open('w')as log:r=subprocess.run(cmd,env={**os.environ,'CARGO_MANIFEST_DIR':str(R/'crates/aa-runtime')},stdout=log,stderr=subprocess.STDOUT)
assert r.returncode==0,r.returncode
files=[source,sound,actor,resources,regression,sound_tests,R/'crates/aa-runtime/src/fx_graph_subtypes.rs',R/'crates/aa-runtime/src/fx_graph_subtypes_tests.rs']
(P/'exact-source-pins.json').write_text(json.dumps(dict(scope='Exact current compiler/core/typed resource identity slices; unrelated Catalog/World binding and effect execution omitted. Native synthetic allocation transports are explicit in fixture tests; no substitute compiler.',sources=[dict(path=str(f.relative_to(R)),sha256=hashlib.sha256(f.read_bytes()).hexdigest())for f in files],externs=pins),indent=2)+'\n')
print('exact source build passed')
