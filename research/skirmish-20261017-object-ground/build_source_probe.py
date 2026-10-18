"""Current BaseTerrain source factory against an existing coherent runtime.
No Cargo build and no native game process. Actual source/CV owner is used.
"""
from pathlib import Path
import subprocess,os,json,hashlib
P=Path(__file__).resolve().parent;R=P.parents[1];D=R/'target/native-runtime/release/deps'
source=R/'crates/aa-runtime/src/object_ground_source.rs'
program='''pub use aa_runtime::{catalog,definitions,live_object_init};
#[path="SOURCE"] mod ground;
use aa_rules::{sources::{V14Sources,SourceLimits},tgi::Limits};
fn main(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
 let mut catalog=catalog::Catalog::new(&actors);
 catalog.settings=Some(aa_runtime::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
 let receipt=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
 aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
 aa_runtime::combat_source_completion::compute(&catalog).unwrap();
 for id in ["ger_corps","usa_corps","gbr_corps"]{
  let binding=catalog.actor_bindings(id,aa_numeric::Precision::Double53).unwrap();
  let bank=catalog.definition_bank.borrow();
  let source=ground::Source::bind(&catalog,&bank,&binding.definition).unwrap();
  println!("{} {:?}",id,source);
  for i in 0..actors.native_data_registry("Biome").unwrap().entries.len(){assert_eq!(source.variation(i).unwrap(),None);}
  assert_eq!(source.padding(),1f32.to_bits());
 }
}
'''.replace('SOURCE',str(source).replace('\\','/'))
(P/'actual-source-probe.rs').write_text(program)
cmd=['rustc','--edition=2024',str(P/'actual-source-probe.rs'),'-L','dependency='+str(D),'-o',str(P/'actual-source-probe.exe')];pins=[source]
for name in ['aa_runtime','aa_rules','aa_mapworld','aa_numeric']:
 f=max(D.glob('lib'+name+'-*.rlib'),key=lambda p:p.stat().st_mtime);cmd+=['--extern',name+'='+str(f)];pins.append(f)
before={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in pins}
with(P/'actual-source-build02.log').open('w')as log:result=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
assert result.returncode==0,result.returncode
with(P/'actual-source02.log').open('w')as log:result=subprocess.run([str(P/'actual-source-probe.exe')],stdout=log,stderr=subprocess.STDOUT,env={**os.environ,'AA_GAME_ROOT':str(R.parent)})
assert result.returncode==0,result.returncode
assert before=={str(p):hashlib.sha256(p.read_bytes()).hexdigest()for p in pins}
(P/'actual-source-pins.json').write_text(json.dumps(before,indent=2))
print('actual BaseTerrain source probe PASS')
