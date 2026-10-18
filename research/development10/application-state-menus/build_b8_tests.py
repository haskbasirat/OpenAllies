"""Small exact production prefix tests; no runtime link or public test mint.

Only world_revision's unchanged pre-cfg(test) portion is generated to avoid
unrelated Sim tests. The entire current prefix file and its actual tests are
included directly. Its private World counter remains private to this harness.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
p=ROOT/'crates/aa-runtime/src/world_revision.rs'
out=HERE/'generated-world-revision.rs';out.write_text(p.read_text().split('#[cfg(test)]')[0])
(HERE/'b8-tests.rs').write_text('''#[path="generated-world-revision.rs"]mod world_revision;
#[path="../../../crates/aa-runtime/src/view_init_prefix.rs"]mod view_init_prefix;
''')
names=['world_revision.rs','view_init_prefix.rs','view_init_prefix_tests.rs','view_init_completion_tests.rs']
(HERE/'b8-source-pins.json').write_text(json.dumps(dict(scope=__doc__,sources=[dict(path='crates/aa-runtime/src/'+n,sha256=sha(ROOT/'crates/aa-runtime/src'/n))for n in names],generated_sha256=sha(out)),indent=2)+'\n')
