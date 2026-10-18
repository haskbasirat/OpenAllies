//!59E530 polymorphic save: base512A00 and CreatorLoad512C30.
//! The live entry point borrows the actual retained Creator allocation and
//! its shared sourceResource+50. It never changes a derived Creator into base.
use crate::{
    film_header::{self, Result, Sink},
    resource_creator::{Creator, Kind},
};
use aa_mapworld::header::WorldCreator;

///512C30 payload after the polymorphic class tag: derived version0, base
///version3 and fields, then current sourceResource IDS (empty for null).
pub fn load_payload(
    sink: &mut impl Sink,
    base: &WorldCreator,
    source_resource_ids: Option<&[u16]>,
) -> Result<()> {
    sink.write(&0u32.to_le_bytes())?;
    film_header::creator_base_payload(sink, base)?;
    film_header::string(sink, source_resource_ids.unwrap_or(&[]))
}

///The immutable loan covers this complete synchronous write. Thus the base
///serialization projection cannot become stale through these stream callbacks;
///the current mutable Creator/source owner is not duplicated or replaced.
pub fn write_current(sink: &mut impl Sink, creator: &Creator) -> Result<()> {
    match creator.kind() {
        Kind::Base => film_header::creator(sink, &creator.base_payload()),
        Kind::Load => {
            film_header::string(
                sink,
                &"KKC_WorldCreatorLoad".encode_utf16().collect::<Vec<_>>(),
            )?;
            load_payload(
                sink,
                &creator.base_payload(),
                creator.source_resource().map(|source| source.ids()),
            )
        }
        Kind::Blank | Kind::Random | Kind::Preset | Kind::Metagame => {
            Err("film writer requires this derived Creator's actual polymorphic serializer".into())
        }
    }
}
