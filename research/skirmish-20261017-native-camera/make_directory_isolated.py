from pathlib import Path
root=Path.cwd();out=root/'research/skirmish-20261017-native-camera/directory-isolated';out.mkdir(exist_ok=True)
for name in ('native_camera','view_scene_init','world_revision'):
 s=(root/f'crates/aa-runtime/src/{name}.rs').read_text()
 if name!='native_camera':s=s.split('#[cfg(test)]')[0]
 else:
  import re
  s=re.sub(r'#\[path="([^"]+)"\]',lambda m:'#'+'[path="../../../crates/aa-runtime/src/'+m[1]+'"]',s)
 # All production bodies copied exactly; only module relocation/test exclusion.
 (out/f'{name}.rs').write_text(s)
(root/'research/skirmish-20261017-native-camera/directory-isolated.rs').write_text('''#![allow(dead_code,unused_imports)]
#[path="directory-isolated/native_camera.rs"]mod native_camera;
#[path="directory-isolated/view_scene_init.rs"]mod view_scene_init;
#[path="directory-isolated/world_revision.rs"]mod world_revision;
#[path="../../crates/aa-runtime/src/native_render_properties.rs"]mod native_render_properties;
#[path="../../crates/aa-runtime/src/native_render_defaults.rs"]mod native_render_defaults;
mod view_grid_state{pub trait Globals{fn word(&mut self,address:u32)->Result<u32,String>;}}
''')
