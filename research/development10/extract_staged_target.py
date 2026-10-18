"""One-time root refactor of the existing checked target projection."""
from pathlib import Path
p=Path(__file__).resolve().parents[2]/"crates/aa-runtime/src/scheduler_owners.rs"
s=p.read_text()
start=s.index("            let raw = match entry {")
end=s.index("            let owner = match self.get(Target::Restoring(*entry)) {",start)
body=s[start:end]
assert "self." not in body
helper='''\n// Resolve the actual moved allocation, independently of saved-ID presence.\nfn staged_target(\n    entry:&aa_mapworld::restore_components::UpdatableTarget,\n    actors:&crate::actor_world::ActorStore,\n)->Result<Target> {\n    use crate::actor_world::{Source, SpecialSource};\n    use aa_mapworld::{map_objects::SpecialObject, restore_components::UpdatableTarget};\n'''
helper+=''.join(line[8:] if line.startswith('        ') else line for line in body.splitlines(True))
helper+='    Ok(target)\n}\n'
s=s[:start]+"            let target=staged_target(entry,actors)?;\n"+s[end:]
s=s.replace("        use crate::actor_world::{Source, SpecialSource};\n        use aa_mapworld::{map_objects::SpecialObject, restore_components::UpdatableTarget};",
    "        use aa_mapworld::restore_components::UpdatableTarget;\n        self.rebind_restoring(actors)?;",1)
method='''    /// Init may schedule an instance whose saved ID is-1. Rebind every\n    /// existing load label, including those absent from the saved-ID table.\n    pub fn rebind_restoring(&mut self,actors:&crate::actor_world::ActorStore)->Result<()> {\n        let labels:Vec<_>=self.by_target.keys().filter_map(|target| match target {\n            Target::Restoring(saved)=>Some(*saved),_=>None,\n        }).collect();\n        for saved in labels {\n            let target=staged_target(&saved,actors)?;\n            self.rebind(Target::Restoring(saved),target)?;\n        }\n        Ok(())\n    }\n'''
needle='    pub fn restoring('
assert s.count(needle)==1
s=s.replace(needle,method+needle,1)+helper
p.write_text(s)
