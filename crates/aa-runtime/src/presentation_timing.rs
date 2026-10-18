//! Retained application timing-stats slice:41C1F0 constructor and41C180.
//! This is not the simulation clock or the Node's59AB60 timer. Disabled
//! smoothing reads no time/AVar input. Finite arithmetic uses unspilled x87
//! values at the caller's actual precision; unsupported inputs stay errors.
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub type Result<T> = std::result::Result<T, String>;
pub trait Services {
    ///Actual866060's unspilled ST0 value, not GetTickCount/World game time.
    fn time_seconds(&mut self) -> Result<Binary>;
    ///SAME current AVar cell at managerB39990+68, read after storing15C.
    fn smoothing_factor_bits(&mut self) -> Result<u32>;
}
#[derive(Debug)]
pub struct State {
    enabled_4: u8,
    smoothed_154: u32,
    frequency_158: u32,
    previous_15c: u32,
}
impl State {
    pub(crate) fn from_native_constructor() -> Self {
        Self { enabled_4:0,smoothed_154:0x3dcccccd,frequency_158:0x41200000,previous_15c:0 }
    }
    pub fn enabled_byte(&self)->u8 {self.enabled_4}
    pub fn smoothed_bits(&self)->u32 {self.smoothed_154}
    pub fn frequency_bits(&self)->u32 {self.frequency_158}
    pub fn previous_bits(&self)->u32 {self.previous_15c}
    pub fn update(&mut self,services:&mut impl Services,precision:Precision)->Result<()> {
        if self.enabled_4==0 {return Ok(())}
        let number=|bits|Binary::from_f32_bits(bits).map_err(|e|e.to_string());
        let time=services.time_seconds()?;
        let delta=time.sub(&number(self.previous_15c)?,precision).map_err(|e|e.to_string())?;
        self.previous_15c=time.store_f32().map_err(|e|e.to_string())?; //41C199
        let factor=number(services.smoothing_factor_bits()?)?;
        let one=Binary::from_i32(1);
        let inverse=one.sub(&factor,precision).map_err(|e|e.to_string())?;
        let new_part=inverse.mul(&delta,precision).map_err(|e|e.to_string())?;
        let old_part=factor.mul(&number(self.smoothed_154)?,precision).map_err(|e|e.to_string())?;
        let smoothed=new_part.add(&old_part,precision).map_err(|e|e.to_string())?;
        //41C1BF compares the unspilled result;41C1C5 stores without popping it.
        let small=smoothed.compare(&number(0x38d1b717)?)==Ordering::Less;
        self.smoothed_154=smoothed.store_f32().map_err(|e|e.to_string())?;
        self.frequency_158=if small {0} else {
            one.div(&smoothed,precision).and_then(|v|v.store_f32()).map_err(|e|e.to_string())?
        };
        Ok(())
    }
    ///No substitute clock or AVar is invented. The proved constructor's
    ///disabled path succeeds without calling either required service.
    pub fn update_with_unbound_services(&mut self,precision:Precision)->Result<()> {
        struct Required;
        impl Services for Required {
            fn time_seconds(&mut self)->Result<Binary>{Err("41C180 requires actual866060 timing owner".into())}
            fn smoothing_factor_bits(&mut self)->Result<u32>{Err("41C180 requires actual current AVar+68".into())}
        }
        self.update(&mut Required,precision)
    }
}

#[cfg(test)]
#[path="presentation_timing_tests.rs"]
mod tests;
