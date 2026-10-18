"""Add the exact existing pure normalized_key function to the standalone host."""
from pathlib import Path
import hashlib,json
here=Path(__file__).resolve().parent;root=here.parents[1]
path=root/'crates/aa-runtime/src/resource_map_cache.rs';source=path.read_text()
function=source[source.index('pub(crate) fn normalized_key('):source.index('\nenum Metadata')]
objects=root/'crates/aa-runtime/src/resource_objects.rs'
objects_source=objects.read_text()
test_module='#[cfg(test)]\n#[path = "resource_objects_tests.rs"]\nmod tests;'
assert objects_source.count(test_module)==1
objects_production=objects_source.replace(test_module,'')
(here/'resource_objects-production.rs').write_text(objects_production)
program='''//!Exact production task/queue/resource owner sources; only the unrelated
//!resource-map-cache pure name normalizer is mechanically extracted, and
//!ResourceObjects' unrelated integration-test module declaration is excluded.
#[path="../../crates/aa-runtime/src/execution_context.rs"]pub mod execution_context;
#[path="resource_objects-production.rs"]pub mod resource_objects;
#[path="../../crates/aa-runtime/src/resource_task.rs"]pub mod resource_task;
pub mod resource_map_cache {type Result<T>=std::result::Result<T,String>;
'''+function+'\n}\n'
(here/'standalone_task.rs').write_text(program)
(here/'normalizer-source-pin.json').write_text(json.dumps(dict(source=str(path.relative_to(root)),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),extraction_sha256=hashlib.sha256(function.encode()).hexdigest()),indent=2)+'\n')
(here/'resource-objects-source-pin.json').write_text(json.dumps(dict(source=str(objects.relative_to(root)),sha256=hashlib.sha256(objects.read_bytes()).hexdigest(),production_extraction_sha256=hashlib.sha256(objects_production.encode()).hexdigest(),excluded=test_module),indent=2)+'\n')
print('Exact production source host generated')
