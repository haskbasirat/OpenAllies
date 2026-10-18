"""Compile verbatim current private sync traversal/tests against existing
compiled owners. This avoids relinking the full runtime test suite for a pure
kernel replay. Actual field binding still requires production/integration QA.
"""
from pathlib import Path
import subprocess,os,json,hashlib
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'qa-output/development10/initial-sync-extracted'
def sha(v):return hashlib.sha256(v).hexdigest()
def latest(crate):
 files=list((ROOT/'target/native-runtime/release/deps').glob('lib'+crate+'-*.rlib'))
 assert files,crate
 return max(files,key=lambda p:p.stat().st_mtime)
def main():
 OUT.mkdir(exist_ok=True)
 source=(ROOT/'crates/aa-runtime/src/initial_world_sync.rs').read_text()
 tests=(ROOT/'crates/aa-runtime/src/initial_world_sync_tests.rs').read_text()
 sync=(ROOT/'crates/aa-runtime/src/live_clocked.rs').read_text()
 sink=(ROOT/'crates/aa-runtime/src/launch_settings_dump.rs').read_text()
 pieces=[source[source.index('pub(crate) struct Counters'):source.index('fn definition_index')],
         source[source.index('fn thing('):source.index('pub(crate) struct Fields')],
         source[source.index('fn traverse('):source.index('#[cfg(test)]')]]
 sync_piece=sync[sync.index("pub struct SyncLog<'a>"):sync.index("impl<'a> From<SyncLog")]
 sink_piece=sink[sink.index('pub trait Sink {'):sink.index('fn count(')]
 unit='use live_clocked::SyncBinding;\nuse aa_actors::ActorId;\nuse aa_numeric::Precision;\nuse aa_simulation::NativeRng;\ntype Result<T>=std::result::Result<T,String>;\nmod live_clocked {use aa_orders::net::sync::Synchronizer;use std::cell::RefCell;\n'+sync_piece+'\n}\n'+sink_piece+'\n'+'\n'.join(pieces)+'\n#[cfg(test)] mod tests {\n'+tests+'\n}\n'
 path=OUT/'kernel.rs';path.write_text(unit,encoding='utf-8')
 crates=['aa_orders','aa_actors','aa_numeric','aa_simulation','aa_mapworld','serde_json']
 deps={name:latest(name)for name in crates}
 args=['rustc','--edition=2024','--test','-C','opt-level=1','-L','dependency='+str(ROOT/'target/native-runtime/release/deps')]
 for name,lib in deps.items():args+=['--extern',name+'='+str(lib)]
 exe=OUT/'kernel.exe';args +=[str(path),'-o',str(exe)]
 env=dict(os.environ);env['CARGO_MANIFEST_DIR']=str(ROOT/'crates/aa-runtime')
 build=subprocess.run(args,cwd=ROOT,env=env,capture_output=True,text=True)
 (OUT/'build.log').write_text(build.stdout+build.stderr,encoding='utf-8');build.check_returncode()
 result=subprocess.run([str(exe),'--nocapture'],cwd=ROOT,env=env,capture_output=True,text=True)
 (OUT/'tests.log').write_text(result.stdout+result.stderr,encoding='utf-8');result.check_returncode()
 report=dict(scope=__doc__,source_sha256=sha(source.encode()),test_source_sha256=sha(tests.encode()),verbatim_piece_sha256=[sha(p.encode())for p in pieces],sync_piece_sha256=sha(sync_piece.encode()),sink_piece_sha256=sha(sink_piece.encode()),extracted_sha256=sha(unit.encode()),exe_sha256=sha(exe.read_bytes()),rlibs={k:dict(path=str(v),sha256=sha(v.read_bytes()))for k,v in deps.items()},fixture_sha256=sha((ROOT/'research/development10/native-initial-world-sync.json').read_bytes()),counter_fixture_sha256=sha((ROOT/'research/development10/native-world-counters.json').read_bytes()))
 (OUT/'receipt.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
 print(result.stdout,flush=True);print(json.dumps(dict(exe_sha256=report['exe_sha256'],scope='current verbatim kernel and3tests; no fullactualWorld activation claim')))
if __name__=='__main__':main()
