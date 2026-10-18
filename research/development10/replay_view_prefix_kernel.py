"""Verbatim current native VWorld field/prefix kernel replay. The public
World-identity wrapper is production checked separately, not transplanted.
"""
from pathlib import Path
import subprocess,os,json,hashlib
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'qa-output/development10/view-prefix-extracted'
def sha(v):return hashlib.sha256(v).hexdigest()
def main():
 OUT.mkdir(exist_ok=True)
 source=(ROOT/'crates/aa-runtime/src/view_init_prefix.rs').read_text();tests=(ROOT/'crates/aa-runtime/src/view_init_prefix_tests.rs').read_text()
 kernel=source[source.index('type Result<T>'):source.index('#[derive(Clone,Copy,Debug,PartialEq,Eq)]')]
 probe=tests[:tests.index('#[test]\nfn public_prefix_retains')]
 unit=kernel+'\n#[cfg(test)]mod tests{\n'+probe+'\n}\n';path=OUT/'kernel.rs';path.write_text(unit,encoding='utf-8')
 lib=max((ROOT/'target/native-runtime/release/deps').glob('libserde_json-*.rlib'),key=lambda p:p.stat().st_mtime)
 exe=OUT/'kernel.exe';env=dict(os.environ);env['CARGO_MANIFEST_DIR']=str(ROOT/'crates/aa-runtime')
 result=subprocess.run(['rustc','--edition=2024','--test','-C','opt-level=1','-L','dependency='+str(lib.parent),'--extern','serde_json='+str(lib),str(path),'-o',str(exe)],cwd=ROOT,env=env,capture_output=True,text=True)
 (OUT/'build.log').write_text(result.stdout+result.stderr);result.check_returncode()
 result=subprocess.run([str(exe),'--nocapture'],cwd=ROOT,env=env,capture_output=True,text=True)
 (OUT/'tests.log').write_text(result.stdout+result.stderr);result.check_returncode()
 report=dict(scope=__doc__,source_sha256=sha(source.encode()),test_sha256=sha(tests.encode()),kernel_sha256=sha(kernel.encode()),test_excerpt_sha256=sha(probe.encode()),exe_sha256=sha(exe.read_bytes()),fixture_sha256=sha((ROOT/'research/development10/native-vworld-prefix.json').read_bytes()),serde_json_sha256=sha(lib.read_bytes()))
 (OUT/'receipt.json').write_text(json.dumps(report,indent=2)+'\n');print(result.stdout);print(json.dumps(report))
if __name__=='__main__':main()
