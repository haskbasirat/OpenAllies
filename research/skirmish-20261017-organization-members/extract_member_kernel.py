"""Compile-only extraction: unchanged5784C0 kernel and its original two tests.
Imports/path helper change; owner adapters and actual-source tests are excluded.
"""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[2]
HERE=Path(__file__).resolve().parent
source=ROOT/'crates/aa-runtime/src/saved_member_properties.rs'
tests=ROOT/'crates/aa-runtime/src/saved_member_properties_tests.rs'
s=source.read_text();t=tests.read_text()
kernel=s[s.index('/// Read services'):s.index('/// Native5784C0(member)')]
probe=t[t.index('struct Probe'):t.index('fn actual_input(')]
evidence=ROOT/'research/skirmish-20261016-member-properties'
out='use aa_actors::ActorId;\nuse aa_body::credit::Object;\ntype Result<T> = std::result::Result<T,String>;\n'+kernel
out+='\n#[cfg(test)] mod tests {\nuse super::*;\nuse std::{cell::RefCell,path::PathBuf};\nuse serde_json::{Value,json};\n'
out+='fn evidence(name:&str)->PathBuf { PathBuf::from('+json.dumps(str(evidence))+').join(name) }\n'
out+=probe+'\n}\n'
target=HERE/'exact-member-properties.rs';target.write_text(out)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
(HERE/'member-kernel-extraction.json').write_text(json.dumps({
 'scope':__doc__,'source':str(source.relative_to(ROOT)),'source_sha256':sha(source),
 'tests':str(tests.relative_to(ROOT)),'tests_sha256':sha(tests),
 'extracted_sha256':sha(target),'kernel_sha256':hashlib.sha256(kernel.encode()).hexdigest(),
 'probe_sha256':hashlib.sha256(probe.encode()).hexdigest()},indent=2)+'\n')
