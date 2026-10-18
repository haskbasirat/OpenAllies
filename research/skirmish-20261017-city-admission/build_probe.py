"""Exact-source narrow replay; no whole-runtime unit link or adapter execution."""
from pathlib import Path
import hashlib,json
here=Path(__file__).resolve().parent;root=here.parents[1]
path=root/'crates/aa-runtime/src/load_city_candidate.rs'
source=path.read_text();start=source.index('pub fn compare_fields(');end=source.index('\npub(crate) fn value(',start)
code='''extern crate self as aa_actors;
#[path="../../crates/aa-actors/src/city_candidates.rs"]
pub mod city_candidates;
#[path="../../crates/aa-runtime/src/zone_city_admission.rs"]
pub mod zone_city_admission;
pub mod load_city_candidate {
use aa_numeric::{Binary,Precision};
use crate::zone_city_admission::ValueComparison;
'''+source[start:end]+'\n}\n'
(here/'standalone_city.rs').write_text(code)
paths=['crates/aa-runtime/src/zone_city_admission.rs','crates/aa-runtime/src/zone_city_admission_tests.rs','crates/aa-runtime/src/load_city_candidate.rs','crates/aa-actors/src/city_candidates.rs']
(here/'probe-source-pins.json').write_text(json.dumps({p:hashlib.sha256((root/p).read_bytes()).hexdigest()for p in paths},indent=2)+'\n')
print('Generated exact whole core/list and exact compare_fields body include, adapter not executed')
