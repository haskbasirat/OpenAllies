    fn post_create(&mut self,key:ObjectKey,_:&CreateRequest<String,()>,flag:u32)->world_creation::Result<()>{
        crate::fresh_post_create_world::post_create(self.staged,self.catalog,key,self.params,flag,self.pc).map_err(cr)
    }
