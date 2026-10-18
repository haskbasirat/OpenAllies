"""Generate a small exact-production-source test harness, no full runtime link.

Private World constructors stay private. Only their unchanged production
prefix before cfg(test) is included, rather than publishing test constructors.
The tiny completion kernel uses the same ordered algorithm with a String
test error alias; actual Node/emitter adapters are production-typechecked.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
pins=[]
for name in ['world_revision','view_init_prefix']:
 source=ROOT/f'crates/aa-runtime/src/{name}.rs';raw=source.read_text();part=raw.split('#[cfg(test)]')[0]
 output=HERE/f'generated-{name}.rs';output.write_text(part)
 pins.append(dict(path=str(source.relative_to(ROOT)),sha256=hashlib.sha256(source.read_bytes()).hexdigest(),fragment_sha256=hashlib.sha256(output.read_bytes()).hexdigest()))
source=ROOT/'crates/aa-netcode/src/local_world_view.rs';raw=source.read_text()
kernel=raw[raw.index('trait CompletionWorld {'):raw.index("struct Actual<'a>")]
test=raw[raw.index('#[cfg(test)]'):].replace('../../../research/skirmish-20261017-ui-world-completion/native-loading.json','native-loading.json')
output=HERE/'generated-native-completion.rs';output.write_text('type Result<T>=std::result::Result<T,String>;\nfn missing(v:impl Into<String>)->String{v.into()}\n'+kernel+test)
pins.append(dict(path=str(source.relative_to(ROOT)),sha256=hashlib.sha256(source.read_bytes()).hexdigest(),kernel_sha256=hashlib.sha256(kernel.encode()).hexdigest(),fragment_sha256=hashlib.sha256(output.read_bytes()).hexdigest()))
(HERE/'all_tests.rs').write_text('''#[path="generated-world_revision.rs"] mod world_revision;
#[path="generated-view_init_prefix.rs"] mod view_init_prefix;
#[path="../../crates/aa-runtime/src/application_loading.rs"] mod application_loading;
#[path="../../crates/aa-runtime/src/view_resource_task.rs"] mod view_resource_task;
#[path="../../crates/aa-runtime/src/ui_world_init.rs"] mod ui_world_init;
#[path="generated-native-completion.rs"] mod native_completion;
''')
(HERE/'standalone-source-pins.json').write_text(json.dumps(dict(scope=__doc__,sources=pins),indent=2)+'\n')
print(json.dumps(dict(generated_sources=len(pins))))
