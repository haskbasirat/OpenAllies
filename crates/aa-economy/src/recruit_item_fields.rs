//! Actual non-Single recruit allocations. Init4E5DF0 creates Base (A29650),
//! while Load4E6C20 type1 creates Counted (A29664). They are distinct owners.
use crate::EconomyError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseItem {
    definition: u32,
    enabled8: u8,
}
impl BaseItem {
    ///4E606F..6078: actual fresh Init allocation, no count+C exists.
    pub fn from_init_constructor(definition: u32) -> Self {
        Self {
            definition,
            enabled8: 1,
        }
    }
    pub fn definition(&self) -> u32 {
        self.definition
    }
    pub fn enabled8(&self) -> u8 {
        self.enabled8
    }
    ///The base portion of4E7920, after the actual definition resolver returns.
    pub fn load_base(&mut self, source: &mut impl ItemInput) -> Result<(), EconomyError> {
        self.definition = source.definition()?;
        self.enabled8 = source.enabled8()?;
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountedItem {
    base: BaseItem,
    count_c: u32,
}
impl CountedItem {
    ///4E6CB7..6CC4: actual type1 constructor before virtual10 Load.
    pub fn from_load_constructor() -> Self {
        Self {
            base: BaseItem {
                definition: 0,
                enabled8: 1,
            },
            count_c: 1,
        }
    }
    ///4E79F0 delegates4E7920 first, then reads SAME allocation+C.
    pub fn load_4e79f0(&mut self, source: &mut impl ItemInput) -> Result<(), EconomyError> {
        self.base.load_base(source)?;
        self.count_c = source.count_c()?;
        Ok(())
    }
    pub fn definition(&self) -> u32 {
        self.base.definition
    }
    pub fn enabled8(&self) -> u8 {
        self.base.enabled8
    }
    pub fn count_c(&self) -> u32 {
        self.count_c
    }
    fn consume(&mut self, wanted: i32) -> bool {
        if (self.count_c as i32) < wanted || self.base.enabled8 == 0 {
            return false;
        }
        self.count_c = self.count_c.wrapping_sub(wanted as u32);
        true
    }
    fn restore(&mut self, wanted: i32) {
        self.count_c = self.count_c.wrapping_add(wanted as u32)
    }
}
///Actual current input services; failure preserves the already-loaded fields.
///No saved-game item producer is inferred merely by implementing this trait.
pub trait ItemInput {
    fn definition(&mut self) -> Result<u32, EconomyError>;
    fn enabled8(&mut self) -> Result<u8, EconomyError>;
    fn count_c(&mut self) -> Result<u32, EconomyError>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    Base(BaseItem),
    Counted(CountedItem),
    ///Identity link to SAME actual singles vector allocation, not a field copy.
    Single {
        definition: u32,
        index: usize,
    },
}
impl Item {
    pub fn definition(&self) -> u32 {
        match self {
            Self::Base(v) => v.definition(),
            Self::Counted(v) => v.definition(),
            Self::Single { definition, .. } => *definition,
        }
    }
    pub fn is_single(&self) -> bool {
        matches!(self, Self::Single { .. })
    }
    pub(super) fn probe_non_single(&mut self, wanted: i32, restore: bool) -> Option<bool> {
        match self {
            Self::Base(v) => Some(v.enabled8 != 0),
            Self::Counted(v) => {
                let result = v.consume(wanted);
                if result && restore {
                    v.restore(wanted)
                }
                Some(result)
            }
            Self::Single { .. } => None,
        }
    }
}
