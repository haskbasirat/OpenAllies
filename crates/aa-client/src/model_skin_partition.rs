//!NiSkinPartition::MakePartitions 87F760 (the 5A5EC0 hardware-skinning
//!repartition of 5A65B0). Not ported yet: every entry point is an explicit
//!error until the native port and its corpus oracle land.
#![allow(dead_code)]
type Result<T>=std::result::Result<T,String>;
pub(crate) fn not_ported()->Result<()>{Err("87F760 NiSkinPartition::MakePartitions is not ported".into())}
