//!49DA40/49D900 (with49D0A0 CreateVFX and49CF80 PlaySFX children) and
//!49B7E0, shared by the canonical source adapter and native oracle replay.
//!Generic Resource/Audio/VFX references retain their real caller owner.
type Result<T>=std::result::Result<T,String>;
#[cfg(test)]
#[path="object_animation_core_tests.rs"]
mod tests;
///49D0A0's 8-byte element at Animation+20: +0 VFX ThingData, +4 Bone Data+C.
///79E130/877EA0 zero both words before the append.
pub(super) struct Vfx<V>{pub vfx:Option<V>,pub location:i32}
///49CF80's 8-byte element at Animation+2C: +0 AudioData, +4 looping byte.
///49B4A0/49B630 zero both before the append. 49B720 plays them later.
pub(super) struct Sfx<A>{pub audio:Option<A>,pub looping:u8}
pub(super) struct Animation<R,A,V>{
    pub ordinal:usize,pub ids:String,pub kind:i32,pub length:u32,
    pub start_min:u32,pub start_max:u32,pub exclusive:u8,pub resource:Option<R>,
    pub vfx:Vec<Vfx<V>>,pub sfx:Vec<Sfx<A>>,
}
pub(super) struct Core<R,A,V>{
    pub animations:Vec<Animation<R,A,V>>,pub by_type:[Vec<usize>;8],pub pending:Option<Animation<R,A,V>>,
}
pub(super) trait Source<R>{
    type Audio;type Vfx;type Child;
    fn string(&mut self,key:&str)->Result<String>;
    fn resource(&mut self,expanded:&str)->Result<Option<R>>;
    fn float(&mut self,key:&str)->Result<u32>;
    fn boolean(&mut self,key:&str)->Result<u8>;
    ///5B4220 on the Animation node.
    fn count(&mut self,name:&str)->Result<usize>;
    ///42BE10 over the native iterator {node,index,name,count}.
    fn child(&mut self,name:&str,index:usize)->Result<Self::Child>;
    ///5B54D0/5B5380 on that child.
    fn child_string(&mut self,child:&Self::Child,key:&str)->Result<String>;
    fn child_boolean(&mut self,child:&Self::Child,key:&str)->Result<u8>;
    ///4BDB70 on [B36530]+C; called for every IDS, including empty.
    fn vfx(&mut self,ids:&str)->Result<Option<Self::Vfx>>;
    ///42C070 on [B36530]+580, then Data+C. Empty IDS is42C070's own null.
    fn bone(&mut self,ids:&str)->Result<Option<i32>>;
    ///416370 on [B36530]+584; 49CF80 calls it only for a nonempty IDS.
    fn audio(&mut self,ids:&str)->Result<Option<Self::Audio>>;
    ///[B36530]+584 manager+20 name, read only by the unknown-IDS throw.
    fn audio_manager(&mut self)->Result<String>;
}
impl<R,A,V> Core<R,A,V>{
    pub fn constructed()->Self{Self{animations:Vec::new(),by_type:std::array::from_fn(|_|Vec::new()),pending:None}}
    pub fn load<S:Source<R,Audio=A,Vfx=V>>(&mut self,prefix:&str,source:&mut S)->Result<()>{
        let ids=source.string("ids")?;
        if self.animations.iter().any(|a|a.ids.eq_ignore_ascii_case(&ids)){
            return Err("410C40 ObjectAnimationSet duplicate IDS".into())
        }
        let format=source.string("kf")?;
        let path=expand_kf(&format,prefix,&ids)?;
        //Native NULL factory result skips allocation, all later reads and
        //both appends. Concrete required factory failures remain failures.
        let Some(resource)=source.resource(&path)? else{return Ok(())};
        self.pending=Some(Animation{ordinal:self.animations.len(),ids,kind:0,length:0,start_min:0,start_max:0,exclusive:0,resource:Some(resource),vfx:Vec::new(),sfx:Vec::new()});
        let kind=object_kind(&source.string("type")?);
        self.pending.as_mut().expect("49C020 allocation").kind=kind;
        if kind<0{return Err("49D98B requires actual native m_type>=0 assertion/global guard before later fields".into())}
        self.pending.as_mut().unwrap().length=source.float("length")?;
        self.pending.as_mut().unwrap().exclusive=source.boolean("exclusive")?;
        self.pending.as_mut().unwrap().start_min=source.float("start_min")?;
        self.pending.as_mut().unwrap().start_max=source.float("start_max")?;
        self.create_vfx(source)?;self.play_sfx(source)?;
        let animation=self.pending.take().expect("loaded49C020 allocation");
        let ordinal=animation.ordinal;let kind=animation.kind as usize;
        self.animations.push(animation);
        self.by_type[kind].push(ordinal);
        Ok(())
    }
    ///49D0A0. Each pair is appended (count+1) BEFORE42BE10 reads its child;
    ///a later throw retains that zeroed or partly stored pair.
    fn create_vfx<S:Source<R,Audio=A,Vfx=V>>(&mut self,source:&mut S)->Result<()>{
        let count=source.count("CreateVFX")?;
        for index in 0..count{
            let pending=self.pending.as_mut().expect("49C020 allocation");
            pending.vfx.push(Vfx{vfx:None,location:0});
            let child=source.child("CreateVFX",index)?;
            let ids=source.child_string(&child,"vfx_ids")?;
            let found=source.vfx(&ids)?;
            let missing=found.is_none();
            let pending=self.pending.as_mut().unwrap();
            pending.vfx.last_mut().unwrap().vfx=found;
            if missing{return Err(format!("410C40 Unknown vfx '{ids}' specified by object animation '{}'",pending.ids))}
            let location=source.child_string(&child,"location")?;
            let bone=source.bone(&location)?.unwrap_or(-1);
            let pending=self.pending.as_mut().unwrap();
            pending.vfx.last_mut().unwrap().location=bone;
            if bone<0{return Err(format!("410C40 Unknown location '{location}' specified by object animation '{}'",pending.ids))}
        }
        Ok(())
    }
    ///49CF80. The pair is appended before its child is read. An empty IDS
    ///skips416370 and stores null; a nonempty miss throws the manager
    ///diagnostic before the store. Looping is read only after a found Audio.
    fn play_sfx<S:Source<R,Audio=A,Vfx=V>>(&mut self,source:&mut S)->Result<()>{
        let count=source.count("PlaySFX")?;
        for index in 0..count{
            let pending=self.pending.as_mut().expect("49C020 allocation");
            pending.sfx.push(Sfx{audio:None,looping:0});
            let child=source.child("PlaySFX",index)?;
            let ids=source.child_string(&child,"sfx_ids")?;
            let found=if ids.is_empty(){None}else{
                let found=source.audio(&ids)?;
                if found.is_none(){
                    let manager=source.audio_manager()?;
                    return Err(format!("410C40 Unknown IDS '{ids}' for data manager '{manager}'"))
                }
                found
            };
            let missing=found.is_none();
            let pending=self.pending.as_mut().unwrap();
            pending.sfx.last_mut().unwrap().audio=found;
            if missing{return Err(format!("410C40 Unknown sfx '{ids}' specified by object animation '{}'",pending.ids))}
            let looping=source.child_boolean(&child,"looping")?;
            self.pending.as_mut().unwrap().sfx.last_mut().unwrap().looping=looping;
        }
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
