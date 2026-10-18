//! Same native World+275 byte and optional World+4 thirteen-word owner.
//! Constructor4F7D41/4F8042 and fresh Init tail4F2BE4,4F2C05..49.
//! This does not execute the rest of World Init. The caller must have executed
//! the intervening actual GameSpeed+278 producer before auxiliary allocation.

#[cfg(test)]
#[path="load_world_runtime_fields_tests.rs"]
mod tests;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
enum Phase {Constructed,Initializing,Initialized,Failed}
/// Move-only ownership. The thirteen words are semantic fields, not a pool or
/// capacity cap. No raw/default/clone constructor can create an initialized one.
pub struct State {
    fog_cheat_275:u8,
    auxiliary:Option<Box<[u32;13]>>,
    phase:Phase,
}
impl State {
    pub fn from_world_constructor()->Self {
        Self {fog_cheat_275:0,auxiliary:None,phase:Phase::Constructed}
    }
    /// Fresh positive-allocation domain of4F27E0. Reinitializing an old nonnull
    /// owner needs actual53A780 destruction and is deliberately unsupported.
    /// Rust allocation failure retains the earlier275 reset and stops binding;
    /// it is not converted to a guessed successful null native allocation.
    pub fn initialize_fresh_auxiliary_4f27e0(&mut self)->Result<(),String> {
        self.initialize_with(|| {
            let mut words=Vec::new();
            words.try_reserve_exact(13).map_err(|e|e.to_string())?;
            words.resize(13,0u32);
            let words:Box<[u32;13]>=words.into_boxed_slice().try_into()
                .map_err(|_|"auxiliary539F30 field count differs".to_owned())?;
            Ok(Some(words))
        })
    }
    fn initialize_with(&mut self,allocate:impl FnOnce()->Result<Option<Box<[u32;13]>>,String>)->Result<(),String> {
        if self.phase!=Phase::Constructed || self.auxiliary.is_some() {
            return Err("World+4 fresh Init requires its actual constructor-null owner".into());
        }
        self.phase=Phase::Initializing;
        self.fog_cheat_275=0; //4F2BE4 precedes allocation and replacement.
        match allocate() {
            Ok(owner)=> {self.auxiliary=owner;self.phase=Phase::Initialized;Ok(())},
            Err(error)=> {self.phase=Phase::Failed;Err(error)},
        }
    }
    pub fn require_initialized(&self)->Result<(),String> {
        if self.phase==Phase::Initialized {Ok(())} else {Err("actual World+4 Init has not completed".into())}
    }
    pub fn fog_cheat_word(&self)->&u8 {&self.fog_cheat_275}
    pub fn auxiliary_words(&self)->Result<Option<&[u32;13]>,String> {
        self.require_initialized()?;Ok(self.auxiliary.as_deref())
    }
    pub fn counter_loan(&mut self)->Result<crate::live_clocked::Counter<'_>,String> {
        self.require_initialized()?;
        Ok(match self.auxiliary.as_deref_mut() {
            Some(words)=>crate::live_clocked::Counter::Present(&mut words[0]),
            None=>crate::live_clocked::Counter::Null,
        })
    }
    pub fn frame_fields<'a>(&'a mut self,svar_fog_cheat:&'a dyn crate::live_clocked::BoolSource)
        ->Result<crate::live_clocked::Fields<'a>,String> {
        self.require_initialized()?;
        let auxiliary=match self.auxiliary.as_deref_mut() {
            Some(words)=>crate::live_clocked::Counter::Present(&mut words[0]),
            None=>crate::live_clocked::Counter::Null,
        };
        Ok(crate::live_clocked::Fields {svar_fog_cheat,world_fog_cheat:&self.fog_cheat_275,auxiliary})
    }
}
