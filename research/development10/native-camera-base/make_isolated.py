from pathlib import Path
import sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
parent=(ROOT/'crates/aa-runtime/src/native_camera.rs').read_text();source=parent
for name in ['pose','ni','target','base']:
 source=source.replace(f'#[path="native_camera_{name}.rs"]',f'#[path="../../../crates/aa-runtime/src/native_camera_{name}.rs"]')
for name in ['backend','render_lists']:
 source=source.replace(f'#[path="native_camera_{name}.rs"]pub mod {name};\n','')
source=source.replace('#[path="native_camera_tests.rs"]','#[path="../../../crates/aa-runtime/src/native_camera_tests.rs"]')
source='#![allow(dead_code)]\n'+source+'\nmod view_grid_state{pub trait Globals{fn word(&mut self,address:u32)->Result<u32,String>;}}\n'
path=HERE/'isolated.rs'
if path.exists()and '--write'not in sys.argv:assert path.read_text()==source
else:path.write_text(source)
