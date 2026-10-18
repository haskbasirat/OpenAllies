//!752FC0/74DA50/74DAA0 over retained local Node fields. Remote connection
//!owners remain required. The completion callback is private and cannot mint
//!an ActivatedSession; its concrete775BF0/773AE0 caller is a separate boundary.
use super::super::*;

///The raw DWORD ring at AdminNode+238 is distinct from packet histories.
///75CFD9..75D001 allocates256 cells without initializing out-of-window words.
///Only the reached constructor/clear/retire domain is exposed here.
pub(crate) struct TaskHistory {
    begin: i32,
    end: i32,
    cells: Vec<Option<u32>>,
}
impl TaskHistory {
    #[cfg(test)]
    pub(crate) fn from_explicit_duration_fixture(begin:i32,values:&[u32],capacity:usize)->Self{
        //Typed native oracle input only; not a task-completion constructor.
        let mut out=Self{begin,end:begin+values.len() as i32,cells:vec![None;capacity.max(values.len()).max(1)]};
        for (i,bits) in values.iter().enumerate(){let slot=out.duration_slot(begin+i as i32);out.cells[slot]=Some(*bits);}
        out
    }
    pub(crate) fn begin_word(&self)->i32{self.begin}
    pub(crate) fn end_word(&self)->i32{self.end}
    fn duration_slot(&self,i:i32)->usize{((i as u32).wrapping_add(0x80000000) as usize)%self.cells.len()}
    pub(crate) fn duration_word(&self,i:i32)->Result<u32>{
        if self.cells.is_empty()||i<self.begin||i>=self.end{return Err(missing("752AE0 invalid current raw duration index"));}
        self.cells[self.duration_slot(i)].ok_or_else(||missing("752AE0 raw history cell has no native writer"))
    }
    pub(crate) fn append_duration_74ecd0(&mut self,bits:u32)->Result<()>{
        //The real315 completion clears/retires this SAME ring to0 before
        //normal frames. Constructor-only negative windows are not a receipt
        //for that transition, and native signed-overflow domains stay refused.
        if self.begin<0||self.end<self.begin||self.end==i32::MAX||self.cells.is_empty(){
            return Err(missing("752AE0 requires actual retired nonnegative DWORD history238"));
        }
        if (self.end-self.begin) as usize==self.cells.len(){
            let new_len=self.cells.len().checked_mul(2).ok_or_else(||missing("raw history allocation size overflow"))?;
            let mut new=Vec::new();new.try_reserve_exact(new_len).map_err(|_|missing("raw history allocation failed"))?;new.resize(new_len,None);
            for i in self.begin..self.end{new[((i as u32).wrapping_add(0x80000000) as usize)%new_len]=self.cells[self.duration_slot(i)];}
            self.cells=new;
        }
        let slot=self.duration_slot(self.end);self.cells[slot]=Some(bits);self.end+=1;Ok(())
    }
    pub(crate) fn retire_duration_750990(&mut self,target:i32)->Result<()>{
        if target<self.begin||target>self.end{return Err(missing("752AE0 invalid typed retire target"));}
        for i in self.begin..target{let slot=self.duration_slot(i);self.cells[slot]=Some(0);}
        self.begin=target;Ok(())
    }
    pub(crate) fn from_native_constructor() -> Self {
        Self {
            begin: i32::MIN,
            end: i32::MIN,
            cells: vec![None; 0x100],
        }
    }
    fn clear_750940(&mut self) -> Result<()> {
        if self.end < self.begin || self.cells.is_empty() {
            return Err(missing("750940 invalid typed current DWORD-history range"));
        }
        for index in self.begin..self.end {
            let slot = ((index as u32).wrapping_add(0x8000_0000) as usize) % self.cells.len();
            self.cells[slot] = Some(0);
        }
        self.begin = i32::MIN;
        self.end = i32::MIN;
        Ok(())
    }
    fn retire_zero_750990(&mut self) {
        //The actual caller just cleared the window; no intermediate scalar
        //values are exposed and this branch performs no allocation.
        self.begin = 0;
        self.end = 0;
    }
}

trait TaskWorld {
    fn set_ordering_peer_status(&mut self, status: u32) -> Result<()>;
    fn peer_count(&mut self) -> Result<usize>;
    fn peer_status(&mut self, index: usize) -> Result<u32>;
    fn clear_history(&mut self) -> Result<()>;
    fn retire_history_zero(&mut self) -> Result<()>;
    fn completion(&mut self, success: bool) -> Result<()>;
    fn clear_diagnostic(&mut self) -> Result<()>;
    fn clear_peer(&mut self, index: usize) -> Result<()>;
    fn clear_view_required(&mut self) -> Result<()>;
}
fn process<W: TaskWorld>(world: &mut W, status: u32) -> Result<()> {
    world.set_ordering_peer_status(status)?;
    let mut success = true;
    //Read count/current entries lazily, matching the native linked traversal.
    let mut index = 0;
    while index < world.peer_count()? {
        let value = world.peer_status(index)?;
        success &= value != 4;
        if matches!(value, 0..=2) {
            return Ok(());
        }
        index += 1;
    }
    world.clear_history()?;
    world.retire_history_zero()?;
    world.completion(success)?;
    world.clear_diagnostic()?;
    let mut index = 0;
    while index < world.peer_count()? {
        world.clear_peer(index)?;
        index += 1;
    }
    world.clear_view_required()
}

struct Current<'a, 'owner, F> {
    players: &'a mut PlayersBorrow<'owner>,
    local: &'a mut LocalClientBorrow<'owner>,
    slot: &'a u32,
    complete: &'a mut F,
}
impl<F: FnMut(&mut PlayersBorrow<'_>, bool) -> Result<()>> TaskWorld for Current<'_, '_, F> {
    fn set_ordering_peer_status(&mut self, status: u32) -> Result<()> {
        //752FC3 reads the SAME Admin284.8569A0 returns that player's+4.
        if *self.slot == 0 {
            return Err(missing("752FC0 null actual ordering player284"));
        }
        let player = self
            .players
            .find_order_player(*self.slot, false)
            .ok_or_else(|| missing("752FC0 actual ordering player allocation is absent"))?;
        match self.players.player_link(&player)? {
            PeerLink::Node => {
                *self.local.peer_task_34 = status;
                Ok(())
            }
            PeerLink::Conn(_) => Err(missing("752FC0 actual remote peer34 owner is required")),
            PeerLink::Null => Err(missing("752FC0 ordering player has NULL client4")),
        }
    }
    fn peer_count(&mut self) -> Result<usize> {
        Ok(self.local.peers_2ac.len())
    }
    fn peer_status(&mut self, index: usize) -> Result<u32> {
        match self.local.peers_2ac.get(index) {
            Some(PeerLink::Node) => Ok(*self.local.peer_task_34),
            Some(PeerLink::Conn(_)) => {
                Err(missing("74DA50 actual remote peer34 owner is required"))
            }
            _ => Err(missing("74DA50 invalid actual peer list entry")),
        }
    }
    fn clear_history(&mut self) -> Result<()> {
        self.local.task_history_238.clear_750940()
    }
    fn retire_history_zero(&mut self) -> Result<()> {
        self.local.task_history_238.retire_zero_750990();
        Ok(())
    }
    fn completion(&mut self, success: bool) -> Result<()> {
        (self.complete)(self.players, success)
    }
    fn clear_diagnostic(&mut self) -> Result<()> {
        Ok(())
    }
    fn clear_peer(&mut self, index: usize) -> Result<()> {
        match self.local.peers_2ac.get(index) {
            Some(PeerLink::Node) => {
                *self.local.peer_task_34 = 0;
                Ok(())
            }
            Some(PeerLink::Conn(_)) => {
                Err(missing("74DAA0 actual remote peer34 owner is required"))
            }
            _ => Err(missing("74DAA0 invalid actual peer list entry")),
        }
    }
    fn clear_view_required(&mut self) -> Result<()> {
        *self.local.vworld_init_required_8c = Some(0);
        Ok(())
    }
}
///Only the concrete order-effects caller may bind the required776560 child.
///No fallback callback, public readiness constructor or independent Node is
///provided by this helper. An error retains all prior stores and histories.
pub(crate) fn process_current_315<'owner, F: FnMut(&mut PlayersBorrow<'_>, bool) -> Result<()>>(
    players: &mut PlayersBorrow<'owner>,
    local: &mut LocalClientBorrow<'owner>,
    slot: &u32,
    status: u32,
    complete: &mut F,
) -> Result<()> {
    if !Arc::ptr_eq(players.identity, local.owner_identity) {
        return Err(missing(
            "315 player roster and Node belong to different native owners",
        ));
    }
    process(
        &mut Current {
            players,
            local,
            slot,
            complete,
        },
        status,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    struct Replay {
        input: Value,
        peers: Vec<u32>,
        target: u32,
        history: TaskHistory,
        required: u8,
        events: Vec<Value>,
    }
    impl Replay {
        fn event(&mut self, kind: &str, fields: Value) -> Result<()> {
            if self.input["stop"].as_str() == Some(kind) {
                return Err(missing(kind));
            }
            let mut value = json!({"kind":kind});
            for (k, v) in fields.as_object().unwrap() {
                value[k] = v.clone();
            }
            self.events.push(value);
            Ok(())
        }
    }
    impl TaskWorld for Replay {
        fn set_ordering_peer_status(&mut self, status: u32) -> Result<()> {
            self.target = status;
            if !self.peers.is_empty() {
                self.peers[self.input["target"].as_u64().unwrap() as usize] = status;
            }
            Ok(())
        }
        fn peer_count(&mut self) -> Result<usize> {
            Ok(self.peers.len())
        }
        fn peer_status(&mut self, index: usize) -> Result<u32> {
            Ok(self.peers[index])
        }
        fn clear_history(&mut self) -> Result<()> {
            self.event("clear_history", json!({}))?;
            self.history.clear_750940()
        }
        fn retire_history_zero(&mut self) -> Result<()> {
            self.event("retire_history", json!({"value":0}))?;
            self.history.retire_zero_750990();
            Ok(())
        }
        fn completion(&mut self, success: bool) -> Result<()> {
            self.event("complete", json!({"success":success}))
        }
        fn clear_diagnostic(&mut self) -> Result<()> {
            self.event("clear_tasks", json!({}))
        }
        fn clear_peer(&mut self, index: usize) -> Result<()> {
            self.peers[index] = 0;
            if index == self.input["target"].as_u64().unwrap() as usize {
                self.target = 0;
            }
            Ok(())
        }
        fn clear_view_required(&mut self) -> Result<()> {
            self.required = 0;
            Ok(())
        }
    }
    #[test]
    fn actual_native_task_history_and_completion_prefixes() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/development10/world-ready-7757f0/native-task-protocol.json"
        ))
        .unwrap();
        assert_eq!(fixture["passed"], true);
        assert_eq!(fixture["cases"].as_array().unwrap().len(), 600);
        for row in fixture["cases"].as_array().unwrap() {
            let input = row["input"].clone();
            let mut peers: Vec<u32> = input["peers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect();
            if !peers.is_empty() {
                peers[input["target"].as_u64().unwrap() as usize] = 0xbad0;
            }
            let range = &input["range"];
            let mut replay = Replay {
                input: input.clone(),
                peers,
                target: 0xbad0,
                required: 0xa5,
                events: vec![],
                history: TaskHistory {
                    begin: range[0].as_i64().unwrap() as i32,
                    end: range[1].as_i64().unwrap() as i32,
                    cells: (0..7).map(|i| Some(0x12340000 + i)).collect(),
                },
            };
            assert_eq!(
                process(&mut replay, input["status"].as_u64().unwrap() as u32).is_err(),
                !row["stop"].is_null(),
                "{row}"
            );
            assert_eq!(json!(replay.events), row["events"], "{row}");
            assert_eq!(json!(replay.peers), row["peers"], "{row}");
            assert_eq!(
                replay.target,
                row["target"].as_u64().unwrap() as u32,
                "{row}"
            );
            assert_eq!(
                replay.required,
                row["required"].as_u64().unwrap() as u8,
                "{row}"
            );
            assert_eq!(
                json!([replay.history.begin as u32, replay.history.end as u32]),
                row["history"],
                "{row}"
            );
            assert_eq!(
                json!(
                    replay
                        .history
                        .cells
                        .iter()
                        .map(|v| v.expect("explicit native lab cell"))
                        .collect::<Vec<_>>()
                ),
                row["cells"],
                "{row}"
            );
        }
    }
    #[test]
    fn constructor_does_not_fabricate_initialized_raw_history_cells() {
        let mut history = TaskHistory::from_native_constructor();
        assert_eq!((history.begin, history.end), (i32::MIN, i32::MIN));
        assert_eq!(history.cells.len(), 256);
        assert!(history.cells.iter().all(Option::is_none));
        history.clear_750940().unwrap();
        history.retire_zero_750990();
        assert_eq!((history.begin, history.end), (0, 0));
        assert!(history.cells.iter().all(Option::is_none));
    }
    #[test]
    fn cross_host_roster_node_refuses_before_status_or_history_stores() {
        let mut first = LocalBootstrap::from_native_constructors();
        let mut second = LocalBootstrap::from_native_constructors();
        //A real partial Node constructor is sufficient for this refusal test;
        //no Reset/Host/StartedMission success is manufactured from it.
        second.host_node = Some(crate::local_host::Node::before_session_reset().unwrap());
        let mut players = PlayersBorrow {
            session: &mut first.session,
            local_player: &first.local_player,
            identity: &first.identity,
        };
        let mut second_parts = second.runtime_borrow().unwrap();
        let mut called = false;
        let result = process_current_315(
            &mut players,
            &mut second_parts.local_client,
            &1,
            3,
            &mut |_, _| {
                called = true;
                Ok(())
            },
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("different native owners")
        );
        assert!(!called);
        assert_eq!(*second_parts.local_client.peer_task_34, 0);
        assert_eq!(second_parts.local_client.task_history_238.begin, i32::MIN);
        assert_eq!(*second_parts.local_client.vworld_init_required_8c, None);
    }
}
