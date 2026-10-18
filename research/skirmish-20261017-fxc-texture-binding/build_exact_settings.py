"""Compile the unmodified settings kernel/tests in an exact tiny source host."""
from pathlib import Path
import subprocess,hashlib,json
R=Path(__file__).resolve().parents[2];P=Path(__file__).resolve().parent
source=R/'crates/aa-runtime/src/resource_texture_settings.rs';tests=source.with_name('resource_texture_settings_tests.rs')
s=source.read_text().replace('//!','//');s=s[:s.index('#[cfg(test)]')]
t=tests.read_text().replace('"../../../research/','"'+str(R/'research').replace('\\','/')+'/')
(P/'exact-settings-host.rs').write_text(s+'\nmod tests{\n'+t+'}\n')
deps=R/'target/native-runtime/release/deps';dep=max(deps.glob('libserde_json-*.rlib'),key=lambda x:x.stat().st_mtime)
cmd=['rustc','--edition=2024','--test',str(P/'exact-settings-host.rs'),'-L','dependency='+str(deps),'--extern','serde_json='+str(dep),'-o',str(P/'exact-settings-tests.exe')]
with(P/'exact-settings-build-final.log').open('w')as log:result=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
assert result.returncode==0,result.returncode
result=subprocess.run([str(P/'exact-settings-tests.exe'),'--nocapture'],capture_output=True,text=True)
(P/'exact-settings-tests-final.log').write_text(result.stdout+result.stderr);print(result.stdout,result.stderr);assert result.returncode==0
(P/'exact-settings-pins-final.json').write_text(json.dumps([dict(path=str(v),sha256=hashlib.sha256(v.read_bytes()).hexdigest())for v in [source,tests,dep,P/'exact-settings-host.rs',P/'exact-settings-tests.exe']],indent=2))
