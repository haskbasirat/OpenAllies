"""Replay verbatim visual heightfield production field/arithmetic kernel.
The public actual World/Surface adapter is separately production compiled.
"""
from pathlib import Path
import subprocess,json,hashlib
ROOT=Path(__file__).resolve().parents[2];OUT=ROOT/'qa-output/development10/view-heightfield-kernel'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 OUT.mkdir(exist_ok=True);source=ROOT/'crates/aa-runtime/src/view_heightfield.rs';tests=ROOT/'crates/aa-runtime/src/view_heightfield_tests.rs'
 code=source.read_text();kernel=code[:code.index('///One retained visual allocation.')]
 probe=OUT/'kernel.rs';probe.write_text(kernel+'\n#[cfg(test)]#[path='+json.dumps(str(tests).replace('\\','/'))+']mod tests;\n')
 deps=ROOT/'target/native-runtime/release/deps';libs={name:max(deps.glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime)for name in ['aa_numeric','serde_json']}
 exe=OUT/'kernel.exe';cmd=['rustc','--edition=2024','--test','-C','opt-level=1','-L','dependency='+str(deps)]
 for name,p in libs.items():cmd+=['--extern',name+'='+str(p)]
 result=subprocess.run(cmd+[str(probe),'-o',str(exe)],cwd=ROOT,capture_output=True,text=True);(OUT/'build.log').write_text(result.stdout+result.stderr);result.check_returncode()
 result=subprocess.run([str(exe),'--nocapture'],cwd=ROOT,capture_output=True,text=True);(OUT/'tests.log').write_text(result.stdout+result.stderr);print(result.stdout+result.stderr);result.check_returncode()
 report=dict(scope=__doc__,source_sha256=sha(source),tests_sha256=sha(tests),kernel_sha256=sha(probe),exe_sha256=sha(exe),fixture_sha256=sha(ROOT/'research/development10/native-view-heightfield.json'),dependencies={name:sha(p)for name,p in libs.items()})
 (OUT/'receipt.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report))
if __name__=='__main__':main()
