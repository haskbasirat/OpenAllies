from pathlib import Path
import hashlib,json
p=Path('research/skirmish-20261017-preworld-view');root=p.resolve().parents[1]
objects=root/'crates/aa-runtime/src/resource_objects.rs';s=objects.read_text();fragment=s[:s.index("///5A60E0's field projection.")]
world=root/'crates/aa-runtime/src/world_revision.rs';w=world.read_text();w=w[:w.index('#[cfg(test)]')]
normal=root/'crates/aa-runtime/src/resource_map_cache.rs';ns=normal.read_text();ns=ns[ns.index('pub(crate) fn normalized_key('):ns.index('\nenum Metadata')]
program='//!Exact-source task/Header/queue/callback field test host.\n'
program+='pub mod resource_objects {\n'+fragment.replace('//!','//')+'}\n'
program+='pub mod world_revision {\n'+w.replace('//!','//')+'}\n'
program+='pub mod resource_map_cache {type Result<T>=std::result::Result<T,String>;\n'+ns+'}\n'
for module in ['execution_context','simulation_execution','view_init_prefix','application_loading','view_resource_task','resource_task','resource_task_guards','ui_world_init']:
 program+=f'#[path="../../crates/aa-runtime/src/{module}.rs"] pub mod {module};\n'
(p/'exact_queue_host.rs').write_text(program)
pins=dict(scope='Resource identity and World revision production fragments omit unrelated Entry/decoder graph and World integration tests only. All Header/queue/AC/Loading implementations plus existing task/prefix/loading tests compile from current exact production paths. No behavioral substitute modules.',fragments=[])
for f,body in [(objects,fragment),(world,w),(normal,ns)]:pins['fragments'].append(dict(path=str(f.relative_to(root)),source_sha256=hashlib.sha256(f.read_bytes()).hexdigest(),fragment_sha256=hashlib.sha256(body.encode()).hexdigest()))
for module in ['execution_context','simulation_execution','view_init_prefix','application_loading','view_resource_task','resource_task','resource_task_dispatch_tests','resource_task_tests','view_init_prefix_tests','view_init_completion_tests','application_loading_tests','resource_task_guards','ui_world_init','ui_world_init_tests']:
 f=root/f'crates/aa-runtime/src/{module}.rs';pins.setdefault('sources',[]).append(dict(path=str(f.relative_to(root)),sha256=hashlib.sha256(f.read_bytes()).hexdigest()))
(p/'exact-host-source-pins.json').write_text(json.dumps(pins,indent=2)+'\n')
