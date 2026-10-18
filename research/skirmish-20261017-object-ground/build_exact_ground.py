"""Exact production position coordinator plus legacy regression, no Cargo link."""
from pathlib import Path
import subprocess,os,json,hashlib
P=Path(__file__).resolve().parent;R=P.parents[1];D=R/'target/native-runtime/release/deps'
source=R/'crates/aa-spatial/src/position.rs';legacy=R/'crates/aa-spatial/tests/position_native.rs'
core=source.read_text();core=core[:core.index('///4EB820 after position')].replace('//!','//')
test=source.with_name('position_ground_tests.rs').read_text()
program='pub use aa_spatial::*;\nmod position{\n'+core+'\nmod replay{'+test+'}}\n'
program=program.replace('"../../../research/','"'+str(R/'research').replace('\\','/')+'/')
(P/'exact-ground-host.rs').write_text(program)
cmd=['rustc','--edition=2024','--test',str(P/'exact-ground-host.rs'),'-L','dependency='+str(D),'-o',str(P/'exact-ground-tests.exe')];pins=[source,legacy,source.with_name('position_ground_tests.rs')]
for name in ['aa_spatial','aa_numeric','serde','serde_json']:
 f=max(D.glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime);cmd+=['--extern',name+'='+str(f)];pins.append(f)
before={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in pins}
with(P/'exact-ground-build01.log').open('w')as log:result=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
assert result.returncode==0,result.returncode
with(P/'exact-ground-tests01.log').open('w')as log:result=subprocess.run([str(P/'exact-ground-tests.exe'),'--nocapture'],stdout=log,stderr=subprocess.STDOUT)
assert result.returncode==0,result.returncode
assert before=={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in pins}
(P/'exact-ground-pins.json').write_text(json.dumps(before,indent=2))
print((P/'exact-ground-tests01.log').read_text())
