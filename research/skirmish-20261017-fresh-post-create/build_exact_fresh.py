"""Tiny exact production caller kernel; no runtime library link or stand-in behavior."""
from pathlib import Path
import hashlib,json,subprocess
P=Path(__file__).resolve().parent;ROOT=P.parents[1]
source=ROOT/'crates/aa-actors/src/world_publication.rs'
test=ROOT/'crates/aa-runtime/src/fresh_post_create_world_tests.rs'
body=test.read_text().replace('use aa_actors::{','use crate::{',1).replace('//!','//')
body=body.replace('../../../research/skirmish-20261017-fresh-post-create/native-fresh-post-create.json',str(P/'native-fresh-post-create.json').replace('\\','/'))
program='pub use aa_actors::{ActorId,components};\n#[path="'+str(source).replace('\\','/')+'"] pub mod world_publication;\nmod tests{\n'+body+'\n}\n'
(P/'exact_fresh_host.rs').write_text(program)
deps=ROOT/'target/native-runtime/release/deps'
command=['rustc','--edition=2024','--test',str(P/'exact_fresh_host.rs'),'-L','dependency='+str(deps),'-o',str(P/'exact-fresh-tests-final.exe')]
pins=[]
for name in ['aa_actors','aa_body','serde_json','thiserror']:
 f=max(deps.glob('lib'+name+'-*.rlib'),key=lambda x:x.stat().st_mtime)
 command+=['--extern',name+'='+str(f)]
 pins.append({'path':str(f),'sha256':hashlib.sha256(f.read_bytes()).hexdigest()})
with(P/'exact-build-final.log').open('w')as log:
 result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT)
assert result.returncode==0,result.returncode
for row in pins:assert hashlib.sha256(Path(row['path']).read_bytes()).hexdigest()==row['sha256']
(P/'exact-host-pins-final.json').write_text(json.dumps({'production_source_sha256':hashlib.sha256(source.read_bytes()).hexdigest(),'test_source_sha256':hashlib.sha256(test.read_bytes()).hexdigest(),'externs':pins,'qualification':'Exact current world_publication source compiled; current test only crate import and absolute fixture path adapted. This checks native caller kernel, not actual Staged ownership binding.'},indent=2)+'\n')
print('exact build passed')
