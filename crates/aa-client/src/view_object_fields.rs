//! Native scalar ownership of VThing45C1E0, VObject455050 and common Init45B530.
//! Backend/model/derived Init and VGrid effects remain reached services. This
//! module cannot publish Thing+C or certify VWorld/74FF50 completion.
use anyhow::{Context,Result,ensure};

pub(crate) trait ConstructorSources {
    ///One reached global read, including the repeated XYZ seed reads.
    fn global_word(&mut self,address:u32)->Result<u32>;
    ///Actual AVar25 RendererDefaultOrientation+4/+8/+C, not chosen facing.
    fn renderer_default_orientation(&mut self)->Result<[u32;3]>;
    fn world_elapsed_e0(&mut self)->Result<u32>;
}
pub(crate) struct ConstructorFailure {pub fields:Fields,pub error:anyhow::Error}
impl std::fmt::Debug for ConstructorFailure {
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("ViewConstructorFailure").field("error",&self.error).finish_non_exhaustive()}
}
///Only scalar DWORDs live here. Source/Thing identity and typed view-list links
///belong to ViewDirectory, never raw or truncated pointer values in this array.
pub(crate) struct Fields {words:[Option<u32>;47],object:bool}
impl Fields {
    pub(super) fn construct(object:bool,sources:&mut impl ConstructorSources)->std::result::Result<Self,ConstructorFailure>{
        let mut fields=Self{words:[None;47],object};
        match fields.construct_inner(sources){Ok(())=>Ok(fields),Err(error)=>Err(ConstructorFailure{fields,error})}
    }
    fn construct_inner(&mut self,sources:&mut impl ConstructorSources)->Result<()> {
        for o in [0x18,0x1c]{self.store(o,0)?;}
        for base in [0x20,0x2c,0x38]{
            for (axis,address) in [0xb43cf8,0xb43cfc,0xb43d00].into_iter().enumerate(){
                let value=sources.global_word(address)?;self.store(base+4*axis,value)?;
            }
        }
        for o in [0x54,0x58,0x5c,0x60,0x68,0x6c,0x70,0x74,4,8,12]{self.store(o,0)?;}
        let facing=sources.renderer_default_orientation()?;
        for (axis,value) in facing.into_iter().enumerate(){self.store(0x44+4*axis,value)?;}
        self.store(0x64,0)?;
        let time=sources.world_elapsed_e0()?;self.store(0x50,time)?;
        if self.object {
            for o in [0x78,0x7c,0x80,0x84,0x9c,0xa0,0xa4,0xa8,0xac,0xb0,0xb8]{self.store(o,0)?;}
            self.store(0x98,0xbf800000)?;self.store(0xb4,u32::MAX)?;
            self.store(0x54,self.required(0x54)?|0x01000000)?;
            for o in [0x88,0x8c,0x90,0x94]{self.store(o,0)?;}
        }
        Ok(())
    }
    pub(super) fn word(&self,offset:usize)->Result<Option<u32>>{
        ensure!(offset%4==0,"unaligned native view scalar read");
        self.words.get(offset/4).copied().context("native view scalar outside allocation")
    }
    pub(super) fn required(&self,offset:usize)->Result<u32>{self.word(offset)?.with_context(||format!("native view scalar+{offset:X} is unwritten"))}
    fn store(&mut self,offset:usize,value:u32)->Result<()> {
        ensure!(offset%4==0,"unaligned native view scalar store");
        *self.words.get_mut(offset/4).context("native view scalar outside allocation")?=Some(value);Ok(())
    }
    pub(super) fn flags(&self)->Result<u32>{self.required(0x54)}
    pub(super) fn aspect_index(&self)->Result<u32>{self.required(0xb4)}
    pub(super) fn set_aspect_index(&mut self,index:u32)->Result<()>{self.store(0xb4,index)}
    ///The ctor's NULL scalar is retired when the SAME Directory binds its
    ///typed Model handle. Positive resource identity is never an ABI DWORD.
    pub(super) fn bind_model_pointer(&mut self)->Result<()>{
        ensure!(self.object,"VThing has no VObject Model7C field");
        self.words[0x7c/4]=None;Ok(())
    }
    pub(super) fn grid_store(&mut self,offset:u32,value:u32)->Result<()> {
        ensure!(matches!(offset,0x54|0x68),"VGrid cannot mutate another native view field");self.store(offset as usize,value)
    }
    ///+54 bit28: the 45B530 Data pre-request is still owed a release.
    pub(super) fn request_pending(&self)->Result<bool>{Ok(self.flags()?&0x10000000!=0)}
    ///45B720 / 45BCA0: `and [+54], 0EFFFFFFFh` after the Data vf18(1).
    pub(super) fn clear_request_pending(&mut self)->Result<()>{self.store(0x54,self.flags()?&!0x10000000)}
    ///+54 written by the UIWorldState selection paths (set_flags_54).
    pub(super) fn set_flags_54(&mut self,flags:u32)->Result<()>{self.flags()?;self.store(0x54,flags)}
    ///45BBA0, shared actual virtual8C for VThing/Object/Actor/Organization/Camera.
    ///The caller lends the current native Thing+8 owner at the reached read.
    pub(super) fn refresh_thing_flag14(&mut self,thing_flags:u32)->Result<()> {
        self.store(0x54,(self.flags()?&!0x4000)|(thing_flags&0x4000))
    }
    ///45B670. The five concrete VThing/Object/Actor/Organization/Camera
    ///virtual30 slots execute943800 ret4, leaving the input radius unchanged.
    ///Native equality treats +/-zero as equal; an unordered comparison takes
    ///the change branch. A required grid child leaves the old radius stored.
    pub(super) fn set_radius(&mut self,radius:u32,grid:&mut impl RadiusUpdate)->Result<()> {
        let old=self.required(0x74)?;
        if f32::from_bits(old)!=f32::from_bits(radius){
            if self.flags()?&0x20!=0{
                grid.change_occupied_radius([self.required(0x38)?,self.required(0x3c)?],old,radius)?;
            }
            self.store(0x74,radius)?;
        }
        Ok(())
    }
    ///45B530. Mutates this same allocation before each fallible child; a child
    ///error leaves its prefix visible. This is a common base Init, not VObject
    ///457A40 or VActor451F90's derived resource/animation/color initialization.
    pub(super) fn common_init(&mut self,services:&mut impl InitServices)->Result<()> {
        let flags=self.flags()?|1;self.store(0x54,flags)?;
        if services.source_flags_1c()?&(1<<21)!=0{self.store(0x54,flags|(1<<25))?;}
        self.store(0x54,self.flags()?|0x100)?;
        if let Some(thing)=services.thing_pose()?{
            for base in [0x38,0x2c,0x20]{for (axis,value) in thing.position.into_iter().enumerate(){self.store(base+4*axis,value)?;}}
            for (axis,value) in thing.facing.into_iter().enumerate(){self.store(0x44+4*axis,value)?;}
            if thing.flags&(1<<12)!=0{self.store(0x54,self.flags()?&!1)?;self.store(0x64,0)?;}
            services.virtual_8c(self)?;
        }
        if self.flags()?&(1<<4)==0{services.occupy(self)?;}
        //The real byte is read before AC; native short-circuit order matters.
        let pre_request=services.pre_request_uvar_108()?;
        if (pre_request&&!services.global_request_mode_b39930()?)||services.vworld_ac()?{
            services.source_pre_request()?;
            self.store(0x54,self.flags()?|0x10000000)?;
            services.enqueue_request(self)?;
        }
        Ok(())
    }
}
pub(super) trait RadiusUpdate{
    fn change_occupied_radius(&mut self,_position:[u32;2],_old:u32,_new:u32)->Result<()>{
        anyhow::bail!("4306C0")
    }
}
pub(super) struct ThingPose {pub position:[u32;3],pub facing:[u32;3],pub flags:u32}
pub(super) trait InitServices {
    fn source_flags_1c(&mut self)->Result<u32>;
    fn thing_pose(&mut self)->Result<Option<ThingPose>>;
    fn virtual_8c(&mut self,fields:&mut Fields)->Result<()>;
    fn occupy(&mut self,fields:&mut Fields)->Result<()>;
    fn pre_request_uvar_108(&mut self)->Result<bool>;
    fn global_request_mode_b39930(&mut self)->Result<bool>;
    fn vworld_ac(&mut self)->Result<bool>;
    fn source_pre_request(&mut self)->Result<()>;
    fn enqueue_request(&mut self,fields:&mut Fields)->Result<()>;
}

#[cfg(test)]mod tests {
    use super::*;
    struct Replay<'a>{c:&'a serde_json::Value,events:Vec<String>}
    impl Replay<'_>{fn scalar(&self,k:&str)->u32{self.c[k].as_u64().unwrap()as u32}fn vector(&self,k:&str)->[u32;3]{std::array::from_fn(|i|self.c[k][i].as_u64().unwrap()as u32)}
        fn child(&mut self,name:&str)->Result<()>{self.events.push(name.into());ensure!(self.c["cutoff"].as_str()!=Some(name),"{name}");Ok(())}}
    impl ConstructorSources for Replay<'_>{
        fn global_word(&mut self,a:u32)->Result<u32>{Ok(self.vector("seed")[match a{0xb43cf8=>0,0xb43cfc=>1,0xb43d00=>2,_=>unreachable!()}])}
        fn renderer_default_orientation(&mut self)->Result<[u32;3]>{Ok(self.vector("default_facing"))}
        fn world_elapsed_e0(&mut self)->Result<u32>{Ok(self.scalar("time"))}
    }
    impl InitServices for Replay<'_>{
        fn source_flags_1c(&mut self)->Result<u32>{Ok(self.scalar("source_flags"))}
        fn thing_pose(&mut self)->Result<Option<ThingPose>>{Ok(self.c["thing"].as_bool().unwrap().then(||ThingPose{position:self.vector("position"),facing:self.vector("facing"),flags:self.scalar("thing_flags")}))}
        fn virtual_8c(&mut self,_:&mut Fields)->Result<()>{self.child("virtual8C")}
        fn occupy(&mut self,_:&mut Fields)->Result<()>{self.child("occupy")}
        fn pre_request_uvar_108(&mut self)->Result<bool>{self.events.push("read-uvar108".into());Ok(self.scalar("uvar")!=0)}
        fn global_request_mode_b39930(&mut self)->Result<bool>{self.events.push("read-global-request-mode".into());Ok(self.scalar("mode")!=0)}
        fn vworld_ac(&mut self)->Result<bool>{self.events.push("read-vworld-ac".into());Ok(self.scalar("ac")!=0)}
        fn source_pre_request(&mut self)->Result<()>{self.child("source-pre-request")}
        fn enqueue_request(&mut self,_:&mut Fields)->Result<()>{self.child("request-queue")}
    }
    fn compare(fields:&Fields,expected:&serde_json::Value){for (key,value) in expected.as_object().unwrap(){let offset=usize::from_str_radix(key.trim_start_matches("0x"),16).unwrap();assert_eq!(fields.word(offset).unwrap(),Some(value.as_u64().unwrap()as u32),"{key}");}}
    #[test]fn actual_base_constructor_and_common_init_all_pc_prefixes(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-view-fields.json")).unwrap();
        for row in fixture["rows"].as_array().unwrap(){
            let c=&row["input"];let mut replay=Replay{c,events:Vec::new()};
            let mut fields=Fields::construct(c["object"].as_bool().unwrap(),&mut replay).unwrap();compare(&fields,&row["output"]["constructor"]);
            fields.store(0x54,replay.scalar("initial_flags")).unwrap();
            let error=fields.common_init(&mut replay).err().map(|e|e.to_string());
            assert_eq!(error.as_deref(),row["output"]["error"].as_str());compare(&fields,&row["output"]["after"]);
            assert_eq!(serde_json::json!(replay.events),row["output"]["events"]);
        }
    }
    #[test]fn constructor_failure_retains_only_reached_fields(){
        struct Missing;impl ConstructorSources for Missing{fn global_word(&mut self,_:u32)->Result<u32>{anyhow::bail!("actual seed unbound")}fn renderer_default_orientation(&mut self)->Result<[u32;3]>{unreachable!()}fn world_elapsed_e0(&mut self)->Result<u32>{unreachable!()}}
        let failure=Fields::construct(true,&mut Missing).err().unwrap();assert_eq!(failure.fields.word(0x18).unwrap(),Some(0));assert_eq!(failure.fields.word(0x20).unwrap(),None);assert_eq!(failure.fields.word(0x54).unwrap(),None);
    }
    #[test]fn actual_shared_virtual_flag14_all_pc(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-view-flag14.json")).unwrap();
        for row in fixture["rows"].as_array().unwrap(){let mut fields=Fields{words:[None;47],object:true};fields.store(0x54,row["flags"].as_u64().unwrap()as u32).unwrap();fields.refresh_thing_flag14(row["thing_flags"].as_u64().unwrap()as u32).unwrap();assert_eq!(fields.flags().unwrap(),row["result"].as_u64().unwrap()as u32);}
    }
    #[test]fn actual_native_radius_setter_retains_old_word_at_required_grid_child(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-view-radius.json")).unwrap();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),768);
        struct Required{child:Option<[u32;4]>}
        impl RadiusUpdate for Required{
            fn change_occupied_radius(&mut self,p:[u32;2],old:u32,new:u32)->Result<()>{self.child=Some([p[0],p[1],old,new]);anyhow::bail!("4306C0")}
        }
        for row in rows{
            let occupied=row["occupied"].as_bool().unwrap();let mut fields=Fields{words:[None;47],object:true};
            for (offset,word)in [(0x74,row["old"].as_u64().unwrap()as u32),(0x54,if occupied{0x1000020}else{0x1000000}),(0x38,0x41280000),(0x3c,0xc0780000)]{fields.store(offset,word).unwrap();}
            let mut grid=Required{child:None};let error=fields.set_radius(row["new"].as_u64().unwrap()as u32,&mut grid).err().map(|e|e.to_string());
            assert_eq!(error.as_deref(),row["output"]["error"].as_str());
            assert_eq!(fields.required(0x74).unwrap(),row["output"]["radius"].as_u64().unwrap()as u32);
            assert_eq!(fields.flags().unwrap(),row["output"]["flags"].as_u64().unwrap()as u32);
            assert_eq!(serde_json::json!(grid.child),row["output"]["child"]);
        }
    }
}
