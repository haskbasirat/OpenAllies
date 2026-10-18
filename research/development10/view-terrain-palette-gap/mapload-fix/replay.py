"""Compile/replay the exact terrain material-grid helpers and focused tests.
No full client build, GPU renderer or simulated World is substituted.
"""
from pathlib import Path
import datetime, hashlib, json, os, subprocess
ROOT=Path(__file__).resolve().parents[4]
OUT=Path(__file__).resolve().parent

def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    source=ROOT/'crates/aa-client/src/view_terrain_backend.rs'
    text=source.read_text()
    helpers=text[text.index('fn use_effective_material_grid('):text.index('fn coordinate(')]
    tests=text[text.index('    #[test]\n    fn effective_material_grid_'):text.index('    #[test]\n    fn terrain_tessellation_')]
    tests=tests.replace('"../../../research/',json.dumps(str(ROOT/'research').replace('\\','/')+'/')[:-1])
    code='use anyhow::{Result,Context,ensure};\nuse std::collections::BTreeMap;\nuse aa_assets::AssetStore;\n'+helpers+'\n#[cfg(test)]mod tests{use super::*;\n'+tests+'\n}\n'
    probe=OUT/'extracted.rs';probe.write_text(code)
    deps=ROOT/'target/x86_64-pc-windows-msvc/release/deps'
    names=['aa_assets','aa_runtime','aa_rules','aa_mapworld','aa_numeric','anyhow','serde_json']
    libs={name:max(deps.glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime)for name in names}
    cmd=['rustc','--edition=2024','--test','--target','x86_64-pc-windows-msvc','-C','opt-level=1','-C','lto=thin','-C','embed-bitcode=yes','-C','codegen-units=16','-L','dependency='+str(deps),'-L','dependency='+str(ROOT/'target/release/deps')]
    for name,path in libs.items():cmd+=['--extern',name+'='+str(path)]
    exe=OUT/'extracted-tests.exe'
    build=subprocess.run(cmd+[str(probe),'-o',str(exe)],cwd=ROOT,capture_output=True,text=True)
    (OUT/'build-current.log').write_text(build.stdout+build.stderr);build.check_returncode()
    env=dict(os.environ,AA_GAME_ROOT=str(ROOT.parent))
    run=subprocess.run([str(exe),'--include-ignored','--nocapture'],cwd=ROOT,env=env,capture_output=True,text=True)
    (OUT/'tests-current.log').write_text(run.stdout+run.stderr);print(run.stdout+run.stderr)
    report={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':__doc__,'source_sha256':sha(source),'extracted_sha256':sha(probe),'exe_sha256':sha(exe),'dependencies':{name:{'path':str(path),'sha256':sha(path)}for name,path in libs.items()},'exit_code':run.returncode}
    (OUT/'test-receipt-current.json').write_text(json.dumps(report,indent=2)+'\n');run.check_returncode()
if __name__=='__main__':main()
