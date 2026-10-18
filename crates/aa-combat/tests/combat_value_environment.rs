//! Reached-service/error-prefix checks for the additive lazy CRT interface.
use aa_body::F32;
use aa_combat::{combat_value::{self as cv,World},Error,Result};
use aa_numeric::{Binary,Precision,crt_pow::CrtEnvironment};
fn f(x:f32)->F32{F32(x.to_bits())}
#[derive(Default)]
struct Prefix{events:Vec<&'static str>}
impl World for Prefix{
 fn ranged_minimum(&mut self)->Result<F32>{self.events.push("ranged");Ok(f(1.0))}
 fn bind_defense_weight_ranged(&mut self)->Result<()>{self.events.push("bind_weight");Ok(())}
 fn defense_weight_ranged(&mut self)->Result<F32>{self.events.push("read_weight");Ok(f(0.5))}
 fn modify_projectile_range(&mut self,_:u32,_:F32)->Result<Binary>{panic!("no actual modifier")}
 fn modify_bombard_range(&mut self,_:u32,_:F32)->Result<Binary>{panic!("no actual modifier")}
 fn modify_power(&mut self,_:u32,_:F32,_:bool)->Result<Binary>{panic!("no actual modifier")}
 fn modify_recharge(&mut self,_:u32,_:F32)->Result<Binary>{panic!("no actual modifier")}
 fn log_enabled(&mut self)->Result<bool>{self.events.push("log");Ok(false)}
}
fn inputs(power:f32,flags:u32)->(cv::AbilityLocal,cv::UnitInputs,cv::MeanValues,cv::Settings){
 (cv::AbilityLocal{definition:cv::AbilityDefinition{flags_54:flags,range_24:f(2.0),area_38:f(0.0),projectile_3c:false,power_64:f(power)},value_04:f(0.0),recharge_18:f(1.0),first_entry_10:None},
  cv::UnitInputs{defense:f(0.0),health:f(0.0),speed:f(1.0),additive_a:f(0.0),additive_b:f(0.0)},
  cv::MeanValues{maximum_74:f(1.0),attack:f(10.0),attack_rate:f(1.0),defense:f(1.0),movement:f(1.0),health:f(1.0),lifetime:f(1.0),offense:f(1.0)},
  cv::Settings{damage_minimum:f(1.0),range_power:f(0.15),area_power:f(1.0),offense_weight:f(0.5),defense_weight:f(0.5),movement_weight:f(0.2),movement_default:f(1.0)})
}
#[test]
fn unused_environment_is_never_requested(){
 for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
  for (power,flags)in [(0.0,0),(-1.0,0),(5.0,0x80)]{
   let (a,u,m,s)=inputs(power,flags);let mut world=Prefix::default();
   let out=cv::ability_value_with_environment(&a,&u,None,&m,&s,||panic!("native never reaches CRT"),&mut world,pc).unwrap();
   if flags&0x80!=0{assert_eq!(out.value,F32::ZERO);assert!(world.events.is_empty())}
   else{assert_eq!(world.events,["ranged","bind_weight","read_weight","log"])}
  }
 }
}
#[test]
fn each_reached_pow_samples_and_error_retains_preceding_world_prefix(){
 for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
  for fail_at in [1,2]{
   let (a,u,m,s)=inputs(5.0,0);let mut world=Prefix::default();let mut samples=0;
   let error=cv::ability_value_with_environment(&a,&u,None,&m,&s,||{
    samples+=1;if samples==fail_at{Err(Error::World(format!("required current CRT sample{fail_at}")))}else{Ok(CrtEnvironment::SSE2_DOUBLE53)}
   },&mut world,pc).unwrap_err();
   assert_eq!(samples,fail_at);assert!(error.to_string().contains("required current CRT"));assert_eq!(world.events,["ranged"]);
  }
 }
}
