//!Bounded child+54 of the same process renderer-manager70EF50 allocation.
//!The modern graphics adapter retains this actual704350-written field beside
//!its existing SceneRenderer. This is not a claim of all Ni/material children
//!having run, nor is7040E0 a Loading/AD/resource-completion notification.
pub struct State { active_24:u8 }
impl State {
    ///704387 zeroes EBX;7043C3 writes BL to this exact byte. Other native
    ///fields/material children remain owned by their separately bound stages.
    pub fn from_constructor_fields()->Self{Self{active_24:0}}
    ///Actual7040E0: one unconditional byte store, no callbacks/World reads.
    pub fn begin_7040e0(&mut self){self.active_24=1;}
    pub fn is_active(&self)->bool{self.active_24!=0}
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn genuine_ctor_prefix_and_begin_leaf_match(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-view-screen.json")).unwrap();
        for row in fixture["cases"].as_array().unwrap(){
            let mut state=State::from_constructor_fields();
            if row["operation"]=="begin"{state.active_24=row["before"].as_u64().unwrap()as u8;state.begin_7040e0();}
            assert_eq!(state.active_24 as u64,row["output"]["active"].as_u64().unwrap());
        }
        assert_eq!(fixture["cases"].as_array().unwrap().len(),30);
    }
}
