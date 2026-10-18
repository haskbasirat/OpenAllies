//!463B50/463440/461F70 ordered source algorithm. Generic references name
//!the caller's SAME Resource/FX owners; source errors retain native stores.
use aa_numeric::{Binary,Precision};
type Result<T>=std::result::Result<T,String>;
#[cfg(test)]
#[path="actor_animation_core_tests.rs"]
mod tests;
#[derive(Clone,Copy)]
pub(super) enum FxList{Standard(usize),Timed(usize)}
pub(super) enum Request<'a,R,F>{Resource(&'a R),Fx(&'a F,usize)}
pub(super) struct Cue{pub time:Option<u32>,pub list:Option<usize>}
pub(super) struct Animation<R,F>{
    pub ordinal:usize,pub ids:String,pub kind:usize,pub resource:Option<R>,
    pub length:u32,pub key_ratio:u32,pub walk_distance:u32,pub blend:u8,pub sister:i32,
    pub weight:u32,pub damage_threshold:u32,pub damage_filter:i32,pub move_threshold:u32,
    pub fx:Option<F>,pub lists:[Option<usize>;3],pub timed:Vec<Cue>,pub complete:bool,
}
pub(super) struct Set<R,F>{pub rows:Vec<Animation<R,F>>,pub by_type:[Vec<usize>;7],pub weights:[u32;7]}
///499FF0's actual constructor table and497020's prefix-before-alias rule.
///Later additional alias writers require their own source owner.
pub(super) fn registered_kind(value:&str)->i32{
    let value=value.to_ascii_lowercase();
    if let Some(index)=["idle","fidget","move","die","ability","summon","hit"].iter().position(|prefix|value.starts_with(prefix)){return index as i32}
    [("walk",2),("jog",2),("run",2),("attackidle",0),("castidle",0),("idlemelee",0),("idlerange",0),("melee1",4),("melee2",4),("range",4),("cast",4),("build",4)].iter().find(|(alias,_)|*alias==value).map_or(-1,|(_,kind)|*kind)
}
pub(super) trait Source<R,F>{
    fn string(&mut self,key:&str)->Result<String>;
    fn resource(&mut self,expanded:&str)->Result<Option<R>>;
    fn fixed(&mut self,key:&str)->Result<u32>;
    fn boolean(&mut self,key:&str)->Result<u8>;
    fn integer(&mut self,key:&str)->Result<i32>;
    fn resolve_kind(&mut self,text:&str)->Result<i32>;
    fn validate_damage_filter(&mut self,filter:i32)->Result<()>;
    fn begin_fx(&mut self)->Result<F>;
    fn append_fx(&mut self,context:&mut F,list:FxList)->Result<usize>;
    fn permanent(&mut self,context:&F,list:usize)->Result<bool>;
    fn timed_count(&mut self)->Result<usize>;
    fn timed_time(&mut self,index:usize)->Result<u32>;
    fn finish_fx(&mut self,context:&mut F)->Result<()>;
}
impl<R,F> Animation<R,F>{
    ///45EA60. Sequence and every FX list receive literal priority/mode0.
    ///Timed lists follow the retained native head order, including empties.
    pub fn request(&self,delta:u32,mut call:impl FnMut(Request<'_,R,F>,u32,u32)->Result<()>)->Result<()>{
        if !self.complete{return Err("Actor Animation source is incomplete".into())}
        if let Some(r)=&self.resource{call(Request::Resource(r),delta,0)?;}
        let context=self.fx.as_ref().ok_or("Actor Animation FX context absent")?;
        for key in &self.lists{call(Request::Fx(context,key.ok_or("Actor Animation list head absent")?),delta,0)?;}
        for cue in &self.timed{call(Request::Fx(context,cue.list.ok_or("Actor Animation timed list absent")?),delta,0)?;}
        Ok(())
    }
}
impl<R,F> Set<R,F>{
    pub fn constructed()->Self{Self{rows:Vec::new(),by_type:std::array::from_fn(|_|Vec::new()),weights:[0;7]}}
    pub fn load(&mut self,prefix:&str,inherit:bool,source:&mut impl Source<R,F>,pc:Precision)->Result<()>{
        let ids=source.string("ids")?;
        if inherit&&self.rows.iter().any(|a|a.ids.eq_ignore_ascii_case(&ids)){return Ok(())}
        let format=source.string("kf")?;
        let expanded=crate::object_animation_set::core::expand_kf(&format,prefix,&ids)?;
        let resource=source.resource(&expanded)?;
        let type_name=source.string("type")?;let kind=source.resolve_kind(&type_name)?;
        if !(0..7).contains(&kind){return Err("463440 requires actual valid Animation registry index before typed vector append".into())}
        let kind=kind as usize;let ordinal=self.rows.len();
        self.rows.push(Animation{ordinal,ids,kind,resource,length:0,key_ratio:0x3f000000,walk_distance:if kind==2{0x3f800000}else{0},blend:0,sister:-1,weight:0x3f800000,damage_threshold:0,damage_filter:-1,move_threshold:0,fx:None,lists:[None;3],timed:Vec::new(),complete:false});
        self.by_type[kind].push(ordinal);
        let a=self.rows.last_mut().expect("463440 stored allocation");
        a.length=source.fixed("length")?;
        a.key_ratio=source.fixed("key_ratio")?;
        a.walk_distance=source.fixed("walk_distance")?;
        a.blend=source.boolean("animation_blend")?;
        let sister=source.string("sister_anim_type")?;
        a.sister=if sister.is_empty(){-1}else{source.resolve_kind(&sister)?};
        a.weight=source.fixed("weight")?;
        a.damage_threshold=source.fixed("damage_threshold")?;
        if kind!=3&&f32::from_bits(a.damage_threshold)!=0.0{return Err("410C40 non-death Animation damage_threshold diagnostic".into())}
        a.damage_filter=source.integer("damage_filter")?;
        if kind!=3&&a.damage_filter != -1{return Err("410C40 non-death Animation damage_filter diagnostic".into())}
        if kind==3&&!matches!(a.damage_filter,-1|666){source.validate_damage_filter(a.damage_filter)?;}
        a.fx=Some(source.begin_fx()?);
        for i in 0..3{
            let context=a.fx.as_mut().unwrap();let key=source.append_fx(context,FxList::Standard(i))?;a.lists[i]=Some(key);
            if source.permanent(context,key)?{return Err("410C40 Animation creates permanent vfx diagnostic".into())}
        }
        let count=source.timed_count()?;
        for index in 0..count{
            //The actual16-byte cue is linked at the HEAD before time read.
            a.timed.insert(0,Cue{time:None,list:None});
            let time=source.timed_time(index)?;a.timed[0].time=Some(time);
            if f32::from_bits(time)==0.0{return Err("410C40 WhenDoAtTime zero time diagnostic".into())}
            let context=a.fx.as_mut().unwrap();let key=source.append_fx(context,FxList::Timed(index))?;a.timed[0].list=Some(key);
            if source.permanent(context,key)?{return Err("410C40 Animation creates permanent vfx diagnostic".into())}
        }
        let ratio=f32::from_bits(a.key_ratio);
        if !ratio.is_finite()||ratio<0.0||ratio>1.0{return Err("410C40 invalid Animation key_ratio diagnostic".into())}
        source.finish_fx(a.fx.as_mut().unwrap())?;
        //463E4B stores the current weight sum only AFTER461F70 returns.
        self.weights[kind]=Binary::from_f32_bits(a.weight).and_then(|v|v.add(&Binary::from_f32_bits(self.weights[kind])?,pc)).map_err(|e|e.to_string())?.store_f32().map_err(|e|e.to_string())?;
        a.complete=true;
        Ok(())
    }
    ///461190 runs for each NONNULL set after all aspect fallback loads.
    ///It preserves authored Move order, writes preceding thresholds in
    ///sequence, and diagnoses missing death threshold only after those stores.
    pub fn finalize(&mut self,pc:Precision)->Result<()>{
        for kind in [0,1,3]{if self.by_type[kind].is_empty(){return Err("410C40 Actor requires idle/fidget/death animation diagnostic".into())}}
        for pair in self.by_type[2].windows(2){
            let a=pair[0];let b=pair[1];
            self.rows[a].move_threshold=Binary::from_f32_bits(self.rows[a].walk_distance)
                .and_then(|v|v.add(&Binary::from_f32_bits(self.rows[b].walk_distance)?,pc))
                .and_then(|v|v.mul(&Binary::from_f32_bits(0x3f000000)?,pc))
                .map_err(|e|e.to_string())?.store_f32().map_err(|e|e.to_string())?;
        }
        if let Some(last)=self.by_type[2].last(){self.rows[*last].move_threshold=0x7f7fffff;}
        if !self.by_type[3].iter().any(|i|f32::from_bits(self.rows[*i].damage_threshold)<=0.0){
            return Err("410C40 Actor has no zero-threshold death animation diagnostic".into())
        }
        Ok(())
    }
}


