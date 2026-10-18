//!49DA40/49D900 and49B7E0, shared by the canonical source adapter and native
//!oracle replay. Generic Resource references retain their real caller owner.
type Result<T>=std::result::Result<T,String>;
#[cfg(test)]
#[path="object_animation_core_tests.rs"]
mod tests;
pub(super) struct Animation<R>{
    pub ordinal:usize,pub ids:String,pub kind:i32,pub length:u32,
    pub start_min:u32,pub start_max:u32,pub exclusive:u8,pub resource:Option<R>,
}
pub(super) struct Core<R>{
    pub animations:Vec<Animation<R>>,pub by_type:[Vec<usize>;8],pub pending:Option<Animation<R>>,
}
pub(super) trait Source<R>{
    fn string(&mut self,key:&str)->Result<String>;
    fn resource(&mut self,expanded:&str)->Result<Option<R>>;
    fn float(&mut self,key:&str)->Result<u32>;
    fn boolean(&mut self,key:&str)->Result<u8>;
    fn create_vfx(&mut self)->Result<()>;
    fn play_sfx(&mut self)->Result<()>;
}
impl<R> Core<R>{
    pub fn constructed()->Self{Self{animations:Vec::new(),by_type:std::array::from_fn(|_|Vec::new()),pending:None}}
    pub fn load(&mut self,prefix:&str,source:&mut impl Source<R>)->Result<()>{
        let ids=source.string("ids")?;
        if self.animations.iter().any(|a|a.ids.eq_ignore_ascii_case(&ids)){
            return Err("410C40 ObjectAnimationSet duplicate IDS".into())
        }
        let format=source.string("kf")?;
        let path=expand_kf(&format,prefix,&ids)?;
        //Native NULL factory result skips allocation, all later reads and
        //both appends. Concrete required factory failures remain failures.
        let Some(resource)=source.resource(&path)? else{return Ok(())};
        self.pending=Some(Animation{ordinal:self.animations.len(),ids,kind:0,length:0,start_min:0,start_max:0,exclusive:0,resource:Some(resource)});
        let kind=object_kind(&source.string("type")?);
        self.pending.as_mut().expect("49C020 allocation").kind=kind;
        if kind<0{return Err("49D98B requires actual native m_type>=0 assertion/global guard before later fields".into())}
        self.pending.as_mut().unwrap().length=source.float("length")?;
        self.pending.as_mut().unwrap().exclusive=source.boolean("exclusive")?;
        self.pending.as_mut().unwrap().start_min=source.float("start_min")?;
        self.pending.as_mut().unwrap().start_max=source.float("start_max")?;
        source.create_vfx()?;source.play_sfx()?;
        let animation=self.pending.take().expect("loaded49C020 allocation");
        let ordinal=animation.ordinal;let kind=animation.kind as usize;
        self.animations.push(animation);
        self.by_type[kind].push(ordinal);
        Ok(())
    }
    pub fn request(&self,delta:u32,mut call:impl FnMut(&R,u32)->Result<()>)->Result<()>{
        let mut index=0;
        while index<self.animations.len(){
            if let Some(resource)=&self.animations[index].resource{call(resource,delta)?;}
            index+=1;
        }
        Ok(())
    }
}
///Actual497F10 registration;497220 performs case-insensitive lookup.
pub(super) fn object_kind(name:&str)->i32{["CONST","RANDOM","IDLE","FIDGET","OPEN_GATE","CLOSE_GATE","CUSTOMPLAY","SELECTED"].iter().position(|v|v.eq_ignore_ascii_case(name)).map_or(-1,|v|v as i32)}
///405D60's bounded two-string variadic domain, with real format-body oracle.
pub(crate) fn expand_kf(format:&str,prefix:&str,ids:&str)->Result<String>{
    let mut out=String::new();let mut chars=format.chars();let args=[prefix,ids];let mut used=0;
    while let Some(c)=chars.next(){
        if c!='%'{out.push(c);continue}
        match chars.next(){Some('%')=>out.push('%'),Some('s')if used<2=>{out.push_str(args[used]);used+=1;},_=>return Err("ObjectAnimation KF requires unported native printf expression".into())}
    }
    Ok(out)
}

