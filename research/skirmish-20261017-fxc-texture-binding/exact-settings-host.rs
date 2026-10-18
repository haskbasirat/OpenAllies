//5A6D10/5A6C70,5A9330 and5A6340 over the current Texture allocation.
//Successful allocations and stable/read-only setting inputs are the admitted
//array domain. Loaded source children and diagnostics remain required services.
type Result<T> = std::result::Result<T, String>;

pub(crate) const SLOT_CONSTRUCTOR: [u32; 9] = [2, 0, 0, 0, 0, 0, 8, 0x22, 0];

///The Vec length represents initialized native allocation capacity, not count60.
///5A5070 initializes unused slots too, and copies the whole previous capacity.
pub(crate) fn push_5a6c70(slots: &mut Vec<[u32; 9]>, count: &mut usize, value: [u32; 9]) -> Result<()> {
    if *count > slots.len() || slots.len() > i32::MAX as usize {
        return Err("Texture render list has invalid native count/capacity".into());
    }
    let next = count.checked_add(1).filter(|v| *v <= i32::MAX as usize)
        .ok_or("Texture render list exceeds admitted signed capacity domain")?;
    if next > slots.len() || slots.is_empty() {
        let doubled = slots.len().checked_mul(2).filter(|v| *v <= i32::MAX as usize)
            .ok_or("Texture render list doubled capacity outside admitted domain")?;
        let mut capacity = doubled.max(next);
        if !slots.is_empty() { capacity = capacity.max(4); }
        slots.resize(capacity, SLOT_CONSTRUCTOR);
    }
    *count = next; //5A6D02 stores count before copying the nine source words.
    slots[next - 1] = value;
    Ok(())
}

pub(crate) trait EntryWorld {
    type Texture: Clone;
    fn flags_08(&mut self, texture: &Self::Texture) -> Result<u32>;
    fn requests_04(&mut self, texture: &Self::Texture) -> Result<u32>;
    fn store_flags_08(&mut self, texture: &Self::Texture, flags: u32) -> Result<()>;
    fn warning_404ed0(&mut self, texture: &Self::Texture, reading: bool) -> Result<()>;
    fn lock_857e10(&mut self, texture: &Self::Texture) -> Result<()>;
    fn loaded_reading_child(&mut self, texture: &Self::Texture, enabled: bool) -> Result<()>;
    fn unlock_857de0(&mut self, texture: &Self::Texture) -> Result<()>;
}
pub(crate) trait World:EntryWorld{
    fn last_location_writable(&mut self, texture: &Self::Texture) -> Result<bool>;
    fn writable_child_40(&mut self, texture: &Self::Texture) -> Result<Option<Self::Texture>>;
}

fn target<W: World>(world: &mut W, mut texture: W::Texture) -> Result<W::Texture> {
    //The native recursion/iteration rereads each current location and child.
    while world.last_location_writable(&texture)? {
        let Some(child) = world.writable_child_40(&texture)? else { break };
        texture = child;
    }
    Ok(texture)
}

pub(crate) fn set_reading_5a9330<W: World>(world: &mut W, texture: W::Texture, raw: u32) -> Result<()> {
    let texture = target(world, texture)?;
    set_reading_at(world,texture,raw)
}
pub(crate) fn set_reading_at<W:EntryWorld>(world:&mut W,texture:W::Texture,raw:u32)->Result<()>{
    let byte = raw as u8;
    let flags = world.flags_08(&texture)?;
    if byte == ((flags >> 20) & 1) as u8 { return Ok(()) }
    if byte == 0 && (world.requests_04(&texture)? as i32) > 0 {
        return world.warning_404ed0(&texture, true);
    }
    world.lock_857e10(&texture)?;
    //Native rereads flags after the lock service, before either store.
    let flags = world.flags_08(&texture)?;
    let flags = if byte == 0 { flags & !0x100000 } else { flags | 0x100000 };
    world.store_flags_08(&texture, flags)?;
    if flags & 1 != 0 { world.loaded_reading_child(&texture, byte != 0)?; }
    world.unlock_857de0(&texture)
}

pub(crate) fn set_rendering_5a6340<W: World>(world: &mut W, texture: W::Texture, raw: u32) -> Result<()> {
    let texture = target(world, texture)?;
    set_rendering_at(world,texture,raw)
}
pub(crate) fn set_rendering_at<W:EntryWorld>(world:&mut W,texture:W::Texture,raw:u32)->Result<()>{
    let byte = raw as u8;
    let flags = world.flags_08(&texture)?;
    if byte == ((flags >> 21) & 1) as u8 { return Ok(()) }
    if byte != 0 { return world.store_flags_08(&texture, flags | 0x200000) }
    if (world.requests_04(&texture)? as i32) > 0 {
        return world.warning_404ed0(&texture, false);
    }
    let flags = world.flags_08(&texture)?;
    world.store_flags_08(&texture, flags & !0x200000)
}


mod tests{
use super::*;
use serde_json::{Value,json};
#[test]
fn render_capacity_keeps_constructor_slots_and_overwrites_only_reached_element() {
    let mut slots=vec![SLOT_CONSTRUCTOR];let mut count=0;
    push_5a6c70(&mut slots,&mut count,[17;9]).unwrap();
    assert_eq!(count,1);assert_eq!(slots,vec![[17;9]]);
    push_5a6c70(&mut slots,&mut count,[23;9]).unwrap();
    assert_eq!(count,2);assert_eq!(slots.len(),4);
    assert_eq!(slots,vec![[17;9],[23;9],SLOT_CONSTRUCTOR,SLOT_CONSTRUCTOR]);
}

struct NativeInput<'a>{input:&'a Value,flags:[u32;2],events:Vec<Value>}
impl EntryWorld for NativeInput<'_>{
    type Texture=usize;
    fn flags_08(&mut self,t:&usize)->Result<u32>{Ok(self.flags[*t])}
    fn requests_04(&mut self,t:&usize)->Result<u32>{Ok(self.input["requests"][*t].as_i64().unwrap() as u32)}
    fn store_flags_08(&mut self,t:&usize,v:u32)->Result<()>{self.flags[*t]=v;Ok(())}
    fn warning_404ed0(&mut self,_:&usize,reading:bool)->Result<()>{
        self.events.push(json!(["warning",if reading{0x107}else{0x13f}]));
        if self.input["warning_error"]==true{Err("warning".into())}else{Ok(())}
    }
    fn lock_857e10(&mut self,t:&usize)->Result<()>{self.events.push(json!(["lock"]));self.flags[*t]^=self.input["lock_toggle"].as_u64().unwrap()as u32;Ok(())}
    fn loaded_reading_child(&mut self,_:&usize,enabled:bool)->Result<()>{self.events.push(json!(["loaded",enabled]));Err("loaded".into())}
    fn unlock_857de0(&mut self,_:&usize)->Result<()>{self.events.push(json!(["unlock"]));Ok(())}
}
impl World for NativeInput<'_>{
    fn last_location_writable(&mut self,t:&usize)->Result<bool>{
        if self.input["locations"][*t]==0{return Ok(false)}
        self.events.push(json!(["writable",*t]));Ok(self.input["writable"]==1)
    }
    fn writable_child_40(&mut self,t:&usize)->Result<Option<usize>>{Ok(if *t==0&&self.input["child"]==true{Some(1)}else{None})}
}
#[test]
fn original_v14_texture_settings_order_flags_capacity_and_error_prefix_all_pc(){
    let fixture:Value=serde_json::from_str(include_str!("E:/Games/Axis and Alies/rust-rebuild/research/skirmish-20261017-fxc-texture-binding/native-texture-settings.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(),570);
    for row in fixture["rows"].as_array().unwrap(){
        let c=&row["input"];let expected=&row["output"];
        let mut world=NativeInput{input:c,flags:[c["flags"][0].as_u64().unwrap()as u32,c["flags"][1].as_u64().unwrap()as u32],events:vec![]};
        let capacity=c["capacity"].as_u64().unwrap()as usize;
        let mut slots=(0..capacity).map(|i|if c["poison_slots"]==true{std::array::from_fn(|j|0x11000000+i as u32*0x100+j as u32)}else{SLOT_CONSTRUCTOR}).collect::<Vec<_>>();
        let mut count=c["count"].as_u64().unwrap()as usize;
        let format:[u32;9]=[1,0,255,65280,16711680,4278190080,32,0x20820881,0xc4100];
        assert_eq!(json!(format),expected["format"]);
        if c["push"]==true{
            let old_capacity=slots.len();
            push_5a6c70(&mut slots,&mut count,format).unwrap();
            if slots.len()!=old_capacity{world.events.push(json!(["alloc",slots.len()*36]));if old_capacity!=0{world.events.push(json!(["free"]));}}
        }
        let result=match c["operation"].as_str().unwrap(){
            "radar"=>set_reading_5a9330(&mut world,0,1).and_then(|()|set_rendering_5a6340(&mut world,0,0)),
            "reading"=>set_reading_5a9330(&mut world,0,c["raw"].as_u64().unwrap()as u32),
            "rendering"=>set_rendering_5a6340(&mut world,0,c["raw"].as_u64().unwrap()as u32),
            _=>unreachable!(),
        };
        assert_eq!(json!(result.err()),expected["stopped"],"{row}");
        assert_eq!(json!(world.flags),expected["flags"],"{row}");
        assert_eq!(json!(count),expected["count"],"{row}");
        assert_eq!(json!(slots.len()),expected["capacity"],"{row}");
        assert_eq!(json!(slots),expected["slots"],"{row}");
        assert_eq!(json!(world.events),expected["events"],"{row}");
    }
}
}
