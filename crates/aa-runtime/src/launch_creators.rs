//! Actual nonnull76E540 Creator siblings. Common513EB0 initialization is
//!shared with Load; each allocation keeps its own derived native class/tail.
use super::{Catalog,Creator,Kind};
type Result<T>=std::result::Result<T,String>;

pub struct Blank { creator:Box<Creator>,doodify:bool }
pub struct Random {
 creator:Box<Creator>,config:RandomConfig,total_terrain_weight:u32,
 terrain_weights:Vec<(Vec<u16>,u32)>,object_counts:Vec<(Vec<u16>,u32)>,num_heroes:u32,description_74:Vec<u16>,
}
///514050's actual NULL+50. A positive configuration must be supplied by its
///real writer, rather than a bool/filename claiming an owned resource.
enum RandomConfig { NullFromConstructor }
pub struct Preset { creator:Box<Creator>,source_50:PresetSource }
enum PresetSource { NullFromConstructor }
pub struct Metagame { creator:Box<Creator>,doodify:bool,words_54_58:[u32;2] }
///No Clone/Default or raw-field construction. Each sibling is a distinct
///allocation retained by the same LaunchSettings owner.
pub struct Others { blank:Blank,random:Random,preset:Preset,metagame:Metagame }
impl Others {
 ///Successful76E540 order around the SAME returned Load allocation. Native
 ///allocator failure/SEH is outside this successful value-construction API.
 pub fn construct_with_load(catalog:&Catalog<'_>,variables:&crate::creator_variables::State)->Result<(Self,Box<Creator>)> {
  let blank=Blank{creator:Creator::from_common_constructor(catalog,variables,Kind::Blank)?,doodify:true};
  let load=Creator::from_load_constructor(catalog,variables)?;
  let random=Random{creator:Creator::from_common_constructor(catalog,variables,Kind::Random)?,config:RandomConfig::NullFromConstructor,
   total_terrain_weight:0,terrain_weights:Vec::new(),object_counts:Vec::new(),num_heroes:0,description_74:Vec::new()};
  let preset=Preset{creator:Creator::from_common_constructor(catalog,variables,Kind::Preset)?,source_50:PresetSource::NullFromConstructor};
  let metagame=Metagame{creator:Creator::from_common_constructor(catalog,variables,Kind::Metagame)?,doodify:true,words_54_58:[0;2]};
  Ok((Self{blank,random,preset,metagame},load))
 }
 pub fn blank(&self)->&Blank {&self.blank}
 pub fn random(&self)->&Random {&self.random}
 pub fn preset(&self)->&Preset {&self.preset}
 pub fn metagame(&self)->&Metagame {&self.metagame}
}
impl Blank {pub fn creator(&self)->&Creator {&self.creator} pub fn doodify(&self)->bool {self.doodify}}
impl Random {
 pub fn creator(&self)->&Creator {&self.creator}
 pub fn config_is_null(&self)->bool {matches!(self.config,RandomConfig::NullFromConstructor)}
 pub fn total_terrain_weight_bits(&self)->u32 {self.total_terrain_weight}
 pub fn terrain_weights(&self)->&[(Vec<u16>,u32)] {&self.terrain_weights}
 pub fn object_counts(&self)->&[(Vec<u16>,u32)] {&self.object_counts}
 pub fn num_heroes(&self)->u32 {self.num_heroes}
 pub fn description_74(&self)->&[u16] {&self.description_74}
}
impl Preset {pub fn creator(&self)->&Creator {&self.creator}pub fn source_is_null(&self)->bool {matches!(self.source_50,PresetSource::NullFromConstructor)}}
impl Metagame {pub fn creator(&self)->&Creator {&self.creator}pub fn doodify(&self)->bool {self.doodify}pub fn words_54_58(&self)->[u32;2]{self.words_54_58}}
