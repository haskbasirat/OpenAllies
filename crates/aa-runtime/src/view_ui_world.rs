//!640640/438E10's game-interface-owned UIWorldState. This is separate from
//!the player's4ED6D0 simulation selection. Constructor children stay installed
//!on a failed later source read; no pointer, input or battle-ready flag is made.
type Result<T> = std::result::Result<T, String>;

pub trait Constructor {
    type Timer;
    ///The actual438970 child initializes59AA70 then captures59AB60's anchor.
    fn timer_59aa70_59ab60(&mut self) -> Result<Self::Timer>;
    ///Actual4B9F40 lookup, after the brush child. The CString copied into the
    ///group is the current display, not an untranslated key fallback.
    fn wildcard_display_4b9f40(&mut self, key: &str) -> Result<Vec<u16>>;
}

pub struct Brush<T> {
    timer_10: T,
    enabled_8: u8,
    words_18_1c: [u32; 2],
    attached_24: Option<()>,
}
impl<T> Brush<T> {
    pub fn timer(&self) -> &T { &self.timer_10 }
}
pub struct StaticGroup<V> {
    name_0: Vec<u16>,
    display_4: Vec<u16>,
    text_8: Vec<u16>,
    members_c: Vec<V>,
    byte_18: u8,
    word_1c: u32,
    byte_20: u8,
}
impl<V> StaticGroup<V> {
    pub fn name(&self) -> &[u16] { &self.name_0 }
    pub fn display(&self) -> &[u16] { &self.display_4 }
    pub fn members(&self) -> &[V] { &self.members_c }
}

///Only actual constructor-written fields are represented. In particular+4,
///byte1E/1F, brush+C and the upper bytes of group18/20 remain unprojected.
pub struct State<V, T> {
    selected_8: Vec<V>,
    hovered_10: Option<V>,
    fields_14_18: [Option<V>; 2],
    bytes_1c_1d: [u8; 2],
    fields_20_2c: [Option<V>; 4],
    brush_30: Option<Brush<T>>,
    groups_34: Vec<StaticGroup<V>>,
    field_40: Option<V>,
    wildcard_44: Option<StaticGroup<V>>,
    fields_48_4c: [Option<V>; 2],
    derived_50_54: Option<[u32; 2]>,
    attempted: bool,
}
impl<V: Copy, T> State<V, T> {
    ///438E10 direct stores before the first positive child. Native container
    ///capacities are allocation details, represented by growable owned lists.
    pub fn before_children_438e10() -> Self {
        Self { selected_8: Vec::new(), hovered_10: None,
            fields_14_18: [None; 2], bytes_1c_1d: [0; 2], fields_20_2c: [None; 4],
            brush_30: None, groups_34: Vec::new(), field_40: None, wildcard_44: None,
            fields_48_4c: [None; 2], derived_50_54: None, attempted: false }
    }
    ///Allocation-success constructor domain. A source/OS refusal retains this
    ///same prefix and forbids reconstructing over it. Allocator SEH is separate.
    pub fn construct_640640(&mut self, sources: &mut impl Constructor<Timer=T>) -> Result<()> {
        if self.attempted { return Err("same UIWorldState constructor was already attempted".into()) }
        self.attempted = true;
        let timer = sources.timer_59aa70_59ab60()?;
        self.brush_30 = Some(Brush { timer_10: timer, enabled_8: 0,
            words_18_1c: [0; 2], attached_24: None });
        let display = sources.wildcard_display_4b9f40("static_group_wildcard_name")?;
        //438F23 passes the separately constructed empty CString as the group
        //name; the wildcard IDS is used only by the preceding display lookup.
        self.wildcard_44 = Some(StaticGroup { name_0: Vec::new(),
            display_4: display, text_8: Vec::new(), members_c: Vec::new(),
            byte_18: 0, word_1c: 0, byte_20: 0 });
        self.derived_50_54 = Some([0; 2]);
        Ok(())
    }
    pub fn require_constructed(&self) -> Result<()> {
        self.derived_50_54.as_ref().ok_or("actual640640 constructor is incomplete")?; Ok(())
    }
    pub fn selected(&self) -> &[V] { &self.selected_8 }
    pub fn hovered(&self) -> Option<V> { self.hovered_10 }
    pub fn brush(&self) -> Option<&Brush<T>> { self.brush_30.as_ref() }
    pub fn wildcard(&self) -> Option<&StaticGroup<V>> { self.wildcard_44.as_ref() }
    ///4363C0 clears+10 only AFTER the concrete virtual68 unhighlight child.
    ///Errors retain the exact current target; no raw hover setter is exposed.
    pub fn clear_hover_4363c0(&mut self, mut child: impl FnMut(V, bool)->Result<()>) -> Result<()> {
        if let Some(view) = self.hovered_10 { child(view, false)?; self.hovered_10 = None; }
        Ok(())
    }
    ///43B470→4363B0 forwards the actual current+10 to concrete virtual54.
    pub fn current_4363b0<R>(&self, child: impl FnOnce(Option<V>)->Result<R>) -> Result<R> {
        child(self.hovered_10)
    }
}

#[cfg(test)]
#[path="view_ui_world_tests.rs"]
mod tests;
