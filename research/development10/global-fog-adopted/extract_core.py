"""Compile the exact unchanged production Fog traversal with its native rows.
This is coordinator parity, not execution of the new placement adapter.
"""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parents[3]
here=Path(__file__).resolve().parent
source=(root/'crates/aa-runtime/src/load_store_fog.rs').read_text()
old=(here/'source-before/load_store_fog.rs').read_text()
core=source[source.index('trait World {'):source.index('///A synchronous loan')]
old_core=old[old.index('trait World {'):old.index('struct Owned<')]
assert core.rstrip()==old_core.rstrip(), 'native coordinator changed'
tests=(root/'crates/aa-runtime/src/load_store_fog_tests.rs').read_text()
tests=tests[:tests.index('#[test]\n#[ignore')]
error=source[source.index('fn error('):source.index('#[cfg(test)]')]
generated='use aa_actors::{ActorId,actor_creation::{Error,Result}};\n'+error+core+'\n#[cfg(test)] mod tests {\n'+tests+'\n}\n'
output=here/'fog-core-exact.rs'
with output.open('x') as f:f.write(generated)
print(json.dumps({'core_sha256':hashlib.sha256(core.encode()).hexdigest(),'generated_sha256':hashlib.sha256(output.read_bytes()).hexdigest(),'fixture_rows':1728,'coordinator_unchanged':True}))
