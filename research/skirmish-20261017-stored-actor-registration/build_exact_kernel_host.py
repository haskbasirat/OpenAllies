"""Compile exact shared production state-loop, tile iteration and entry prefix.
No Catalog/ActorStore/World binding or positive state/zone child is fabricated.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
zone=ROOT/'crates/aa-runtime/src/load_zone_entry.rs';text=zone.read_text()
iterator=text[text.index('pub(crate) fn visit_indices('):text.index('///4CFEA0 over')]
entered=text[text.index('pub(crate) trait EnteredFields'):text.index('struct SavedEntered')]
oldtest=ROOT/'crates/aa-runtime/src/load_actor_registration_tests.rs';testtext=oldtest.read_text();start=testtext.index('#[test]');end=testtext.index('\n#[test]',start+1);looptest=testtext[start:end]
fixture=ROOT/'research/skirmish-20261016-load-actor-registration/native-registration.json'
looptest=looptest.replace('"../../../research/skirmish-20261016-load-actor-registration/native-registration.json"',f'r"{fixture}"')
body='pub use aa_actors::{actor_creation,components,ActorId};\nuse aa_actors::actor_creation::{Error,Result};\n'
body+='use aa_actors::components::StateBits;\nuse aa_numeric::{Binary,Precision};\nuse aa_spatial::{collision::{Grid,Shape},geometry::{Placement,TileRect}};\nuse aa_zones::set::Entry;\nfn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}\n'
body+=f'#[path=r"{ROOT / "crates/aa-actors/src/actor_registration.rs"}"]pub mod actor_registration;\n'
body+='pub mod load_zone_entry {use super::*;\n'+iterator+entered
body+=f'#[path=r"{ROOT / "crates/aa-runtime/src/load_zone_entry_tests.rs"}"]mod iterator_tests;\n'+'}\n'
body+='mod original_registration_tests{use super::*;\n'+looptest+'}\n'
body+=f'#[path=r"{ROOT / "crates/aa-runtime/src/stored_actor_registration_tests.rs"}"]mod new_entry_tests;\n'
out=HERE/'exact_kernel_host.rs'
with out.open('x') as f:f.write(body)
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
doc=dict(schema=1,scope=__doc__,zone_source_sha256=H(zone),iterator_fragment_sha256=hashlib.sha256(iterator.encode()).hexdigest(),entered_fragment_sha256=hashlib.sha256(entered.encode()).hexdigest(),
 state_loop_sha256=H(ROOT/'crates/aa-actors/src/actor_registration.rs'),old_test_source_sha256=H(oldtest),new_test_sha256=H(ROOT/'crates/aa-runtime/src/stored_actor_registration_tests.rs'),
 host_sha256=H(out),producer_sha256=H(Path(__file__)),transformations=['Exact production iterator/entered traits and kernel fragments; original pure state-loop test changes only include_str path to absolute native fixture. All test behavior and production algorithms remain unchanged.'])
with (HERE/'exact-kernel-source-pins.json').open('x') as f:f.write(json.dumps(doc,indent=1)+'\n')
print(json.dumps(doc))
