"""Exact private LaunchState kernel replay without a full netcode link.

Extract unchanged current wire field declaration, actual native constructor,
ordinary_launch function and its current cfg-test block. The test error alias
only represents Unsupported; no HostInput or authority factory is simulated.
The complete actual factory is covered by the current production typecheck.
"""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def item(text,needle):
 start=text.index(needle);brace=text.index('{',start);depth=1;i=brace+1
 while depth:
  if text[i]=='{':depth+=1
  elif text[i]=='}':depth-=1
  i+=1
 return text[start:i]
paths=[ROOT/'crates/aa-netcode/src'/n for n in ['lobby.rs','local_bootstrap.rs','local_selected_map_confirm.rs']]
raw=[p.read_text()for p in paths]
parts=[item(raw[0],'pub struct LaunchState {'),item(raw[1],'pub fn constructor_launch_state()'),item(raw[2],'fn ordinary_launch(')]
tests=raw[2][raw[2].index('#[cfg(test)]'):]
tests=tests.replace('../../../research/development10/application-state-menus/native-selected-confirm.json','native-selected-confirm.json')
out=HERE/'selected-kernel-tests.rs';out.write_text('type Result<T>=std::result::Result<T,Error>;\n#[derive(Debug)]enum Error{Unsupported(&\'static str)}\n'+'\n'.join(parts)+'\n'+tests)
(HERE/'selected-source-pins.json').write_text(json.dumps(dict(scope=__doc__,sources=[dict(path=str(p.relative_to(ROOT)),sha256=sha(p))for p in paths],kernel_parts_sha256=[hashlib.sha256(p.encode()).hexdigest()for p in parts],generated_sha256=sha(out)),indent=2)+'\n')
