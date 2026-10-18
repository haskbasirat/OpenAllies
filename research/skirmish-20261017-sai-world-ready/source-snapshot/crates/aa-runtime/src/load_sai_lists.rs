//! Native region+28/+34 and manager+90 list containers. Entries are GTIDs;
//! their owning pointer vectors remain separate, retaining duplicate prepends.
use std::collections::VecDeque;
pub(super) type Result<T>=std::result::Result<T,String>;
#[derive(Debug)]
pub(super) struct RegionLists {pub land:Vec<VecDeque<u32>>,pub water:Vec<VecDeque<u32>>}
impl RegionLists {
    ///7BA4F0 constructs both native vectors with count0, capacity5.
    pub fn constructed()->Result<Self> {
        let mut land=Vec::new();let mut water=Vec::new();
        land.try_reserve_exact(5).map_err(|e|e.to_string())?;
        water.try_reserve_exact(5).map_err(|e|e.to_string())?;Ok(Self {land,water})
    }
    pub fn prepend(&mut self,water:bool,index:usize,actor:u32)->Result<()> {
        let rows=if water {&mut self.water}else {&mut self.land};
        while rows.len()<=index {rows.try_reserve(1).map_err(|e|e.to_string())?;rows.push(VecDeque::new());}
        rows[index].try_reserve(1).map_err(|e|e.to_string())?;rows[index].push_front(actor);Ok(())
    }
}
pub(super) fn water(flags:u32,low:u32,high:u32)->bool {
    flags&(1<<29)!=0 || low&((1<<25)|(1<<26))!=0 || high&(4|16)!=0
}
