"""Exact current constructor/source-list selection slices; no runtime link."""
from pathlib import Path
import json,hashlib,subprocess,os
P=Path(__file__).resolve().parent;R=P.parents[1]
source=R/'crates/aa-runtime/src/fx_graph.rs';s=source.read_text();tests=source.with_name('fx_graph_tests.rs');t=tests.read_text()
def function(text,name):
 start=text.index('fn '+name+'(');end=text.index('{',start)+1;depth=1
 while depth:
  depth+=(text[end]=='{')-(text[end]=='}');end+=1
 return text[start:end]
def derives(text,name):return text.rfind('#[derive',0,text.index(name))
resource=source.with_name('resource_objects.rs');rs=resource.read_text();rs=rs[:rs.index("///5A60E0's field projection.")]
definition=source.with_name('definition_fx.rs');ds=definition.read_text();ds=ds[derives(ds,'pub enum ListKind'):ds.index('/// Source inputs only.')]
key=source.with_name('view_fx_command_list.rs');ks=key.read_text();ks=ks[:ks.index('pub(crate) trait Message')]
program='use aa_rules::resources::{Provenance,SourceIdentity};use aa_numeric::Precision;use std::collections::BTreeMap;pub type Result<T>=std::result::Result<T,String>;\n'
program+='pub mod resource_objects{'+rs.replace('//!','//')+'}\nmod view_fx_command_list{'+ks.replace('//!','//')+'}\n'+ds
program+=s[derives(s,'pub enum FxcKind'):s.index('#[path = "fx_graph_actor_request_source.rs"]')]
program+=s[derives(s,'pub struct NodeId'):s.index('/// Completed graph owner;')]
sound=source.with_name('fx_graph_sound.rs');ss=sound.read_text();ss=ss[ss.index('pub(super) fn construct_play_sfx'):ss.index('///416370:')]
actor=source.with_name('fx_graph_actor_state.rs');a=actor.read_text();astart=a.index('pub(super) fn construct_stop_vfx');astop=a.index('\nimpl Graph {',astart)
program+='mod sound{use super::*;'+ss+'}\nmod actor_state{use super::*;'+a[astart:astop]+'}\n'
program+='#[path="'+str(source.with_name('fx_graph_subtypes.rs')).replace('\\','/')+'"] mod subtypes;\n'
#Compile the exact derived child without its unit tests in this host.
subtype=source.with_name('fx_graph_subtypes.rs').read_text();subtype=subtype[:subtype.index('#[cfg(test)]')]
program=program[:program.index('#[path="')]+ 'mod subtypes{'+subtype.replace('//!','//')+'}\n'
program+=s[derives(s,'pub struct Graph'):s.index('///Borrowed capability')]
program+=s[s.index('/// Opaque owned428480'):s.index('\nimpl Graph {',s.index('/// Opaque owned428480'))]
program+='impl Graph{\n'
for name in ['nodes','roots','kind','require_runtime_fields','list_kind','view_list_roots']:
 program+=function(s[s.index('\nimpl Graph {',s.index('/// Opaque owned428480')):],name)+'\n'
program+='}\nmod regression{use super::*;use serde_json::{Value,json};\n'
program+=t[:t.index('#[test]')].replace('use super::*;','').replace('use serde_json::{Value, json};','')
program+=t[t.index('struct CreatedInput'):t.index('#[test]',t.index('struct CreatedInput'))]
program+=t[t.index('impl ExistingInputs for CreatedInput'):t.index('#[test]',t.index('impl ExistingInputs for CreatedInput'))]
for name in ['original_v14_created_event_constructor_embedded_null_and_thread_clock_selection_match','original_v14_existing_event_and_source_predicate_order_match','completed_view_arena_selects_prior_created_head_without_cloning_or_constructed_fallback']:
 program+='#[test]\n'+function(t,name)+'\n'
program+='}\n';program=program.replace('"../../../research/','"'+str(R/'research').replace('\\','/')+'/')
(P/'exact-events-host.rs').write_text(program,encoding='utf-8')
deps=R/'target/native-runtime/release/deps';cmd=['rustc','--edition=2024','--test',str(P/'exact-events-host.rs'),'-L','dependency='+str(deps),'-o',str(P/'exact-events-tests.exe')];pins=[]
for name in ['aa_rules','aa_numeric','aa_actors','serde','serde_json']:
 f=max(deps.glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime);cmd+=['--extern',name+'='+str(f)];pins.append(f)
with(P/'exact-events-build01.log').open('w')as log:result=subprocess.run(cmd,env={**os.environ,'CARGO_MANIFEST_DIR':str(R/'crates/aa-runtime')},stdout=log,stderr=subprocess.STDOUT)
assert result.returncode==0,result.returncode
result=subprocess.run([str(P/'exact-events-tests.exe'),'--nocapture'],capture_output=True,text=True)
(P/'exact-events-tests01.log').write_text(result.stdout+result.stderr);print(result.stdout,result.stderr);assert result.returncode==0
files=[source,tests,resource,definition,key,sound,actor,source.with_name('fx_graph_subtypes.rs'),P/'exact-events-host.rs',P/'exact-events-tests.exe',*pins]
(P/'exact-events-pins.json').write_text(json.dumps([dict(path=str(v),sha256=hashlib.sha256(v.read_bytes()).hexdigest())for v in files],indent=2))
