//!Application+10C/+110 field owner and native40CE10 SetState coordinator.
//!This is separate from GameAppState+C and Session+E4/+F8. UI state nodes and
//!the two transported arguments remain the application's actual caller owners.
pub type Result<T> = std::result::Result<T, String>;

///Actual reached diagnostic and table-node virtual calls. There are no default
///success services. Args are transported by reference, with no pointer cast,
///inspection, Clone or Copy requirement. A callback may reenter set_state on
///this SAME owner; subsequent native field reads observe its retained writes.
pub trait Services {
    type Node;
    type Arg8: ?Sized;
    type ArgC: ?Sized;
    fn diagnostic(&mut self, application: &mut State, from: u32, to: u32) -> Result<()>;
    fn state_node(&mut self, index: u32) -> Result<Option<Self::Node>>;
    fn leave(&mut self, application: &mut State, node: Self::Node) -> Result<()>;
    fn enter(
        &mut self,
        application: &mut State,
        node: Self::Node,
        word8: &Self::Arg8,
        word_c: &Self::ArgC,
    ) -> Result<()>;
}

///Move-only pair projection of4177E0's proven4178F0/4178F6 stores. Construct
///once with the actual application; this does not construct UI nodes, threads,
///managers or certify completion of the full native application constructor.
#[derive(Debug)]
pub struct State {
    current_10c: u32,
    previous_110: u32,
}
impl State {
    pub fn from_constructor() -> Self {
        Self {
            current_10c: 0,
            previous_110: 0,
        }
    }
    pub fn current(&self) -> u32 {
        self.current_10c
    }
    pub fn previous(&self) -> u32 {
        self.previous_110
    }

    ///40CE10. Requested0 is a complete no-op. Equal positive state still calls
    ///OnEnter. Diagnostic/leave errors preserve all actual callback writes;
    ///the previous/current stores precede OnEnter and survive its failure.
    pub fn set_state<S: Services>(
        &mut self,
        requested: u32,
        word8: &S::Arg8,
        word_c: &S::ArgC,
        services: &mut S,
    ) -> Result<()> {
        if requested == 0 {
            return Ok(());
        }
        if requested != self.current_10c {
            let from = self.current_10c;
            services.diagnostic(self, from, requested)?;
            //Diagnostic may have reentered the application.40CE46 rereads10C.
            let current = self.current_10c;
            if current != 0 {
                let node = services
                    .state_node(current)?
                    .ok_or_else(|| format!("40CE5A native leave state node{current} is null"))?;
                services.leave(self, node)?;
            }
            //40CE61 rereads AFTER OnLeave; it does not retain the initial state.
            self.previous_110 = self.current_10c;
            self.current_10c = requested;
        }
        let current = self.current_10c;
        let node = services
            .state_node(current)?
            .ok_or_else(|| format!("40CE84 native enter state node{current} is null"))?;
        services.enter(self, node, word8, word_c)
    }
}

#[cfg(test)]
#[path = "application_state_tests.rs"]
mod tests;
