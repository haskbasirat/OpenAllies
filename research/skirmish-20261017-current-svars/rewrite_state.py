from pathlib import Path
p=Path('crates/aa-runtime/src/load_view_variables.rs');s=p.read_text();s=s.replace('struct Current {','#[cfg(test)]\nstruct Current {',1).replace('impl Current {','#[cfg(test)]\nimpl Current {',1)
a=s.index('    current: Current,',s.index('pub struct State'));b=s.index('    sources:',a);s=s[:a]+'    store: crate::canonical_svars::Store,\n'+s[b:]
a=s.index('        // Validate the public Settings projection');b=s.index('\n    pub fn validate_catalog',a)
s=s[:a]+'''        let store=crate::canonical_svars::from_fresh_source(catalog,pc)?;
        // Validate closed gameplay accessors once, so these infallible views
        // cannot encounter a missing or wrongly typed current entry later.
        for (name,kind) in [("FogOfWar",3),("CheatFog",1),("ZoneBuilderFastDetectionZones",1),
          ("ModifierAttributeClampMultiply",2),("ResourceStockpileDefault",2),("GameSpeed",2),
          ("EconomyStockpileResourceMax",2),("CityVictoryRatio",3),("StartingExperience",2),
          ("SupplyProvideUnderSiege",1),("SupplyStockpileRechargeRate",2)] {
            store.require_kind(name,kind)?;
        }
        for field in crate::load_combat_variables::FIELDS {store.require_kind(field.name(),2)?;}
        Ok(Self {store,sources:catalog.actors.sources.clone(),source_context:context,rules_source:None})
    }
''' +s[b:]
a=s.index('        self.validate_catalog(catalog)?;',s.index('pub fn publish_world_rules'));b=s.index('\n    ///512200 publication',a)
s=s[:a]+'''        let owner=crate::load_world_rules::Owner::load(catalog,inventory)?;
        self.publish_loaded_rules(catalog,&owner)
    }
''' +s[b:]
a=s.index('        let mut seen=[0usize;4];',s.index('pub fn publish_loaded_rules'));b=s.index('        self.rules_source=Some(',a)
s=s[:a]+'''        //512200 visits every retained descriptor in native registration order.
        //Each setter writes the SAME entry that gameplay and76D820 borrow.
        for row in owner.records() {
            self.store.publish_rule(row)?;
        }
''' +s[b:]
s=s.replace('Ok(self.game_speed114)','Ok(self.store.word("GameSpeed"))')
a=s.index('    pub fn fog_integer');s=s[:a]+'''    pub fn registrations(&self)->impl Iterator<Item=Registration<'_>> {self.store.registrations()}
    pub fn current(&self,index:usize)->Result<CurrentValue<'_>,String>{self.store.current(index)}
    pub fn fog_integer(&self)->i32 {self.store.word("FogOfWar") as i32}
    pub fn combat_values(&self)->crate::load_combat_variables::Values<'_> {crate::load_combat_variables::Values::borrow(&self.store)}
    pub fn assign_combat_bits(&mut self,f:crate::load_combat_variables::Field,v:u32){self.store.assign_word_raw(f.name(),v)}
    pub fn modifier_clamp_multiply_bits(&self)->u32{self.store.word("ModifierAttributeClampMultiply")}
    pub fn assign_modifier_clamp_multiply_bits(&mut self,v:u32){self.store.assign_word_raw("ModifierAttributeClampMultiply",v)}
    pub fn stockpile_default_bits(&self)->u32{self.store.word("ResourceStockpileDefault")}
    pub fn stockpile_maximum_bits(&self)->u32{self.store.word("EconomyStockpileResourceMax")}
    pub fn assign_stockpile_maximum_bits(&mut self,v:u32){self.store.assign_word_raw("EconomyStockpileResourceMax",v)}
    pub fn city_victory_ratio(&self)->i32{self.store.word("CityVictoryRatio")as i32}
    pub fn starting_experience_bits(&self)->u32{self.store.word("StartingExperience")}
    pub fn supply_provide_under_siege(&self)->bool{*self.store.bool_ref("SupplyProvideUnderSiege")!=0}
    pub fn assign_supply_provide_under_siege(&mut self,v:u8){self.store.assign_bool_raw("SupplyProvideUnderSiege",v)}
    pub fn supply_recharge_rate_bits(&self)->u32{self.store.word("SupplyStockpileRechargeRate")}
    pub fn assign_supply_recharge_rate_bits(&mut self,v:u32){self.store.assign_word_raw("SupplyStockpileRechargeRate",v)}
    pub fn assign_starting_experience_bits(&mut self,v:u32){self.store.assign_word_raw("StartingExperience",v)}
    pub fn assign_city_victory_ratio(&mut self,v:i32){self.store.assign_word_raw("CityVictoryRatio",v as u32)}
    pub fn assign_stockpile_bits(&mut self,v:u32){self.store.assign_word_raw("ResourceStockpileDefault",v)}
    pub fn fog_enabled(&self)->bool{self.fog_integer()!=0}
    pub fn cheat_fog_byte(&self)->u8{*self.cheat_fog_word()}
    pub fn cheat_fog_word(&self)->&u8{self.store.bool_ref("CheatFog")}
    pub fn cheat_fog_enabled(&self)->bool{self.cheat_fog_byte()!=0}
    pub fn fast_detection_zones_byte(&self)->u8{*self.store.bool_ref("ZoneBuilderFastDetectionZones")}
    pub fn fast_detection_zones(&self)->bool{self.fast_detection_zones_byte()!=0}
    pub fn assign_fog_integer(&mut self,v:i32){self.store.assign_word_raw("FogOfWar",v.clamp(0,2)as u32)}
    pub fn assign_cheat_fog(&mut self,v:u8){self.store.assign_bool_raw("CheatFog",v)}
    pub fn assign_fast_detection_zones(&mut self,v:u8){self.store.assign_bool_raw("ZoneBuilderFastDetectionZones",v)}
}
///Borrowed current cells, read only when a native consumer reaches its getter.
pub enum CurrentValue<'a>{Bool(&'a u8),Fixed(&'a u32),Int(&'a u32),CString(&'a [u16]),Point3(&'a [u32;3])}
pub struct Registration<'a>{pub(crate) ordinal:u32,pub(crate) name:&'a str,pub(crate) kind:u32}
impl Registration<'_>{pub fn ordinal(&self)->u32{self.ordinal}pub fn name(&self)->&str{self.name}pub fn kind(&self)->u32{self.kind}}
'''
p.write_text(s)
p=Path('crates/aa-runtime/src/load_combat_variables.rs');s=p.read_text();a=s.index('pub struct Values');b=s.index('    /// Immutable per-call',a);s=s[:a]+'''pub struct Values<'a> { store:&'a crate::canonical_svars::Store }
impl<'a> Values<'a> {
    pub(crate) fn borrow(store:&'a crate::canonical_svars::Store)->Self{Self{store}}
    pub fn bits(&self,field:Field)->u32{self.store.word(field.name())}
'''+s[b:];p.write_text(s)
p=Path('crates/aa-runtime/src/lib.rs');s=p.read_text();s+='\nmod canonical_svars;\n';p.write_text(s)
