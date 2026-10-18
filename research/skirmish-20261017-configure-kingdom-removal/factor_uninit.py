"""One-time factoring of our existing staged4D7580 storage services.
No native algorithm is regenerated; exact existing service blocks are retained.
"""
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
p=ROOT/'crates/aa-runtime/src/map_replacement_drain.rs'
s=p.read_text();start=s.index('fn actor_uninit(');end=s.index('///Actual4D55E0',start);old=s[start:end]
def part(a,b):return old[old.index(a):old.index(b,old.index(a))].strip()
components='let components = &canonical(staged, catalog, key)?\n        .actor\n        .as_ref()\n        .unwrap()\n        .components;\n'
blocks={
'timed_list':part('//Native4D37E0','    if actor_word(staged, catalog, key, 0x144)'),
'allocation_144':part('if actor_word(staged, catalog, key, 0x144)','    let active ='),
'observer_transition':part('let active =','    *staged'),
'set_category_140':part('*staged','    let companion ='),
'companion_158':part('let companion =','    for zone in canonical'),
'zones':part('for zone in canonical','    if canonical(staged, catalog, key)?\n        .actor'),
'linked_160':part('if actor_word(staged, catalog, key, 0x160)','    if staged\n        .actors\n        .key(ActorId::from_raw(actor_word(staged, catalog, key, 0x164)?))'),
'linked_164':part('if staged\n        .actors\n        .key(ActorId::from_raw(actor_word(staged, catalog, key, 0x164)?))','    let components ='),
'mind':components+part('if let Some(component) = components.get(ComponentSlot::Mind)','    if let Some(component) = components.get(ComponentSlot::Element)'),
'element':components+part('if let Some(component) = components.get(ComponentSlot::Element)','    if actor_word(staged, catalog, key, 0xe8)'),
'owner':part('if actor_word(staged, catalog, key, 0xe8)','    if staged\n        .actors\n        .key(ActorId::from_raw(actor_word(staged, catalog, key, 0x168)?))'),
'linked_168':part('if staged\n        .actors\n        .key(ActorId::from_raw(actor_word(staged, catalog, key, 0x168)?))','    let components ='),
'structure':components+part('if components.present(ComponentSlot::Structure)','    //4CF390'),
'component':part('if canonical(staged, catalog, key)?\n            .actor','    or_removal_flags(staged, key, 2)?;\n    base_object')[:-1].rstrip(),
}
def aliases(text):
 vals={'staged':'&mut *self.staged','catalog':'self.catalog','key':'self.key','pc':'self.pc','retain':'self.retain'}
 return ''.join(f'let {name}={value};\n'for name,value in vals.items()if name in text)
out='''fn actor_uninit(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,retain:bool,pc:Precision)->Result<()> {
    if canonical(staged,catalog,key)?.definition.native_class!=3 {
        return Err("4D7580 actual ordinary Actor vtable required".into());
    }
    crate::actor_uninit::run(&mut StagedUninit {staged,catalog,key,retain,pc})
}
struct StagedUninit<'a,'source> {staged:&'a mut Staged,catalog:&'a Catalog<'source>,key:ObjectKey,retain:bool,pc:Precision}
impl crate::actor_uninit::World for StagedUninit<'_,'_> {
    fn or_flags(&mut self,bits:u32)->Result<()> {or_removal_flags(self.staged,self.key,bits)}
    fn state_bit25(&mut self)->Result<bool> {
        Ok(canonical(self.staged,self.catalog,self.key)?.actor.as_ref().unwrap().components.state.low&(1<<25)!=0)
    }
'''
for name,body in blocks.items():
 arg=',slot:ComponentSlot'if name=='component'else''
 out+=f'fn {name}(&mut self{arg})->Result<()> {{\n{aliases(body)}{body}\nOk(())\n}}\n'
out+='''fn base_object(&mut self)->Result<()> {
    base_object_uninit_prefix(self.staged,self.catalog,self.key,self.retain,self.pc)
}
}
'''
p.write_text(s[:start]+out+s[end:])
print('Factored existing staged storage blocks:',','.join(blocks))
