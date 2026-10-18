from pathlib import Path
import hashlib,json,subprocess
P=Path(__file__).resolve().parent;R=P.parents[1];D=R/'target/s-graphics-review/release/deps'
src=R/'crates/aa-runtime/src/stored_ground_relocation.rs';live=R/'crates/aa-runtime/src/live_clocked.rs'
s=src.read_text();l=live.read_text()
fields=l[l.index('/// The same process Synchronizer A/'):l.index("impl<'a> From<SyncLog")]
body=s[s.index('pub fn log_position('):s.index('struct CatalogEffects')]
tests=s[s.index('#[cfg(test)]'):]
program='use aa_spatial::{Error,Result,position::Diagnostic};\nfn fail(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}\nmod live_clocked{use std::cell::RefCell;use aa_orders::net::sync::Synchronizer;\n'+fields+'}\n'+body+tests
host=P/'exact-position-log-host.rs';host.write_text(program)
cmd=['rustc','--edition=2024','--test',str(host),'-L','dependency='+str(D),'-o',str(P/'exact-position-log-tests.exe')];pins=[src,live,host]
for name in ['aa_spatial','aa_orders']:
 f=max(D.glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime);cmd+=['--extern',name+'='+str(f)];pins.append(f)
before={str(x):hashlib.sha256(x.read_bytes()).hexdigest()for x in pins}
with(P/'position-log-build01.log').open('w')as f:r=subprocess.run(cmd,stdout=f,stderr=subprocess.STDOUT)
assert r.returncode==0,r.returncode
with(P/'position-log-tests01.log').open('w')as f:r=subprocess.run([str(P/'exact-position-log-tests.exe'),'--nocapture'],stdout=f,stderr=subprocess.STDOUT)
assert r.returncode==0,r.returncode
assert before=={str(x):hashlib.sha256(x.read_bytes()).hexdigest()for x in pins}
(P/'position-log-pins.json').write_text(json.dumps(before,indent=2)+'\n')
print((P/'position-log-tests01.log').read_text())
