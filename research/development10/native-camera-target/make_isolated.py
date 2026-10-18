from pathlib import Path
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
p=(ROOT/'crates/aa-runtime/src/native_camera.rs').read_text()
state=p[p.index('///One move-only field'):p.index('///Every dependency')]
helpers=p[p.index('fn b('):p.index('///700E30.')]
source='#![allow(dead_code)]\nuse aa_numeric::{Binary,Precision};\nuse std::cmp::Ordering;\ntype Result<T>=std::result::Result<T,String>;\n'
source+='#[path="../../../crates/aa-runtime/src/native_camera_pose.rs"]pub mod pose;\n#[path="../../../crates/aa-runtime/src/native_camera_target.rs"]pub mod target;\n'+state+helpers
path=HERE/'isolated.rs'
if path.exists()and '--write'not in __import__('sys').argv:assert path.read_text()==source
else:path.write_text(source)
full=p
for name in ['pose','ni','target']:
 full=full.replace(f'#[path="native_camera_{name}.rs"]',f'#[path="../../../crates/aa-runtime/src/native_camera_{name}.rs"]')
for name in ['backend','render_lists']:
 full=full.replace(f'#[path="native_camera_{name}.rs"]pub mod {name};\n','')
full=full.replace('#[path="native_camera_tests.rs"]','#[path="../../../crates/aa-runtime/src/native_camera_tests.rs"]')
full='#![allow(dead_code)]\n'+full
path=HERE/'isolated_full.rs'
if path.exists()and '--write'not in __import__('sys').argv:assert path.read_text()==full
else:path.write_text(full)
