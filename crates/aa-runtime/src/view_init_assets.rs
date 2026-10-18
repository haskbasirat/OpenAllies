//! Native43F1D4..43F948, the ordered asset segment of VWorld43F0A0.
//! The caller lends its retained field/resource owners. This segment alone
//! neither constructs the graphics scene nor writes nativeAD/Loading17.
type Result<T>=std::result::Result<T,String>;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum TextureSlot{Circle,Square,HollowCircle,HollowSquare}
impl TextureSlot{
    pub fn offset(self)->u32{match self{Self::Circle=>0x11c,Self::Square=>0x120,Self::HollowCircle=>0x124,Self::HollowSquare=>0x128}}
    pub fn key(self)->&'static str{match self{Self::Circle=>"TextureCircle",Self::Square=>"TextureSquare",Self::HollowCircle=>"TextureHollowCircle",Self::HollowSquare=>"TextureHollowSquare"}}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum EffectSlot{MoveCommand,SelectFacing,HealthBar,MoraleBar,SupplyStockpileBar,AttackEfficiencyBar,
    Ghosting,IllegalPlacement,CircleBrush,SquareBrush,ProjectedBoxTexture,ProjectedCircleTexture,
    LocationMarkerEditor,RecruitMarkerRallyPoint,Text,TextUnscaled,DamageNumbers,HealNumbers}
impl EffectSlot{
    pub fn offset(self)->u32{match self{
        Self::MoveCommand=>0x1c0,Self::SelectFacing=>0x1cc,Self::HealthBar=>0x1d0,Self::MoraleBar=>0x1d4,
        Self::SupplyStockpileBar=>0x1d8,Self::AttackEfficiencyBar=>0x1dc,Self::Ghosting=>0x1e0,
        Self::IllegalPlacement=>0x1e4,Self::CircleBrush=>0x1e8,Self::SquareBrush=>0x1ec,
        Self::ProjectedBoxTexture=>0x1f0,Self::ProjectedCircleTexture=>0x1f4,Self::LocationMarkerEditor=>0x1f8,
        Self::RecruitMarkerRallyPoint=>0x1fc,Self::Text=>0x200,Self::TextUnscaled=>0x204,
        Self::DamageNumbers=>0x1c4,Self::HealNumbers=>0x1c8}}
    pub fn key(self)->&'static str{match self{
        Self::MoveCommand=>"VFXMoveCommand",Self::SelectFacing=>"VFXSelectFacing",Self::HealthBar=>"VFXHealthBar",
        Self::MoraleBar=>"VFXMoraleBar",Self::SupplyStockpileBar=>"VFXSupplyStockpileBar",
        Self::AttackEfficiencyBar=>"VFXAttackEfficiencyBar",Self::Ghosting=>"VFXGhosting",Self::IllegalPlacement=>"VFXIllegalPlacement",
        Self::CircleBrush=>"VFXCircleBrush",Self::SquareBrush=>"VFXSquareBrush",Self::ProjectedBoxTexture=>"VFXProjectedBoxTexture",
        Self::ProjectedCircleTexture=>"VFXProjectedCircleTexture",Self::LocationMarkerEditor=>"VFXLocationMarkerEditor",
        Self::RecruitMarkerRallyPoint=>"VFXRecruitMarkerRallyPoint",Self::Text=>"VFXText",Self::TextUnscaled=>"VFXTextUnscaled",
        Self::DamageNumbers=>"VFXDamageNumbers",Self::HealNumbers=>"VFXHealNumbers"}}
}
const TEXTURES:[TextureSlot;4]=[TextureSlot::Circle,TextureSlot::Square,TextureSlot::HollowCircle,TextureSlot::HollowSquare];
const EFFECTS:[EffectSlot;16]=[EffectSlot::MoveCommand,EffectSlot::SelectFacing,EffectSlot::HealthBar,EffectSlot::MoraleBar,
    EffectSlot::SupplyStockpileBar,EffectSlot::AttackEfficiencyBar,EffectSlot::Ghosting,EffectSlot::IllegalPlacement,
    EffectSlot::CircleBrush,EffectSlot::SquareBrush,EffectSlot::ProjectedBoxTexture,EffectSlot::ProjectedCircleTexture,
    EffectSlot::LocationMarkerEditor,EffectSlot::RecruitMarkerRallyPoint,EffectSlot::Text,EffectSlot::TextUnscaled];
///The exact reached lookup set, in native Init order, for source binding.
///This is query order; it does not invent an AVar registration order.
pub fn alias_keys()->impl Iterator<Item=&'static str>{
    TEXTURES.into_iter().map(TextureSlot::key).chain(EFFECTS.into_iter().chain([EffectSlot::DamageNumbers,EffectSlot::HealNumbers]).map(EffectSlot::key))
}

pub trait Services{
    type Alias;
    type Texture;
    type Effect;
    fn environment_43ea50(&mut self)->Result<()>;
    ///410CB0 reads the SAME current AVar string allocation, not a default.
    fn alias_410cb0(&mut self,key:&'static str)->Result<Self::Alias>;
    fn texture_5a77e0(&mut self,alias:&Self::Alias,flags:u32)->Result<Option<Self::Texture>>;
    fn effect_4bdb70(&mut self,alias:&Self::Alias)->Result<Option<Self::Effect>>;
    ///Infallible native direct-field stores, including allocation-null.
    fn store_texture(&mut self,slot:TextureSlot,value:Option<Self::Texture>);
    fn store_effect(&mut self,slot:EffectSlot,value:Option<Self::Effect>);
    fn effect_present(&self,slot:EffectSlot)->bool;
    ///Read current stored effect18→Descriptor18 only after its null test.
    fn effect_kind(&self,slot:EffectSlot)->Result<u32>;
    fn warning_404ea0(&mut self,line:u32,text:&'static str)->Result<()>;
}
fn effect(s:&mut impl Services,slot:EffectSlot)->Result<()>{
    let alias=s.alias_410cb0(slot.key())?;
    let value=s.effect_4bdb70(&alias)?;
    s.store_effect(slot,value);
    drop(alias); //Native current CString retires before following warnings.
    Ok(())
}
pub fn initialize(s:&mut impl Services)->Result<()>{
    s.environment_43ea50()?;
    for slot in TEXTURES{
        let alias=s.alias_410cb0(slot.key())?;
        let value=s.texture_5a77e0(&alias,0)?;
        s.store_texture(slot,value);
        drop(alias);
    }
    for slot in EFFECTS{effect(s,slot)?;}
    if !s.effect_present(EffectSlot::MoveCommand){s.warning_404ea0(0x161,"VFXMoveCommand VFX not found")?;}
    effect(s,EffectSlot::DamageNumbers)?;
    if !s.effect_present(EffectSlot::DamageNumbers)||s.effect_kind(EffectSlot::DamageNumbers)?!=12{
        s.warning_404ea0(0x166,"VFXDamageNumbers must point to a VFXTEXT")?;
    }
    effect(s,EffectSlot::HealNumbers)?;
    if !s.effect_present(EffectSlot::HealNumbers)||s.effect_kind(EffectSlot::HealNumbers)?!=12{
        s.warning_404ea0(0x16b,"VFXHealNumbers must point to a VFXTEXT")?;
    }
    Ok(())
}
#[cfg(test)]
#[path="view_init_assets_tests.rs"]
mod tests;
