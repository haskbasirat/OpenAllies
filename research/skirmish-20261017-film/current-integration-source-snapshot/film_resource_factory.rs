//! Actual5A27A0/5A31D0 field construction over the retained managed World.
//! Resource/depot lookup, removal/publication and5A00E0 stream open remain
//! ordered caller operations; this constructor does not manufacture them.
use crate::{
    film_header_owner, film_process_metadata, film_resource::Resource, film_world_capture,
};

pub struct Failure {
    error: String,
    creator: Option<Box<crate::resource_creator::Creator>>,
    header: Option<film_header_owner::State>,
    header_failure: Option<film_header_owner::Failure>,
}
impl Failure {
    pub fn error(&self) -> &str {
        &self.error
    }
    pub fn creator(&self) -> Option<&crate::resource_creator::Creator> {
        self.creator.as_deref()
    }
    pub fn header(&self) -> Option<&film_header_owner::State> {
        self.header.as_ref()
    }
    pub fn header_failure(&self) -> Option<&film_header_owner::Failure> {
        self.header_failure.as_ref()
    }
}
impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilmResourceFactoryFailure")
            .field("error", &self.error)
            .field("creator", &self.creator.is_some())
            .field("header", &self.header.is_some())
            .field("header_constructor_prefix", &self.header_failure.is_some())
            .finish()
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error)
    }
}
impl std::error::Error for Failure {}

///Fresh Film resource fields; `ids` is the caller's actual filename CString.
///Clone current Resource+88 first, construct an independent Header second,
///capture current World third, then set its Rules mode3. On failure the
///returned object retains all Rust field/process/localization prefixes; C++
///allocator failure and destructor/SEH behavior are outside this owned slice.
pub fn construct_current<P, C>(
    ids: &[u16],
    process: &mut film_process_metadata::State,
    mut context: film_world_capture::Context<'_, '_, P, C>,
) -> std::result::Result<Resource, Failure>
where
    P: crate::load_kingdom_glyph::Players + crate::film_header_entries::Players,
    C: FnMut() -> Result<[u32; 2], String>,
{
    construct_current_in(ids, process, &mut context)
}
pub fn construct_current_in<P, C>(
    ids: &[u16],
    process: &mut film_process_metadata::State,
    context: &mut film_world_capture::Context<'_, '_, P, C>,
) -> std::result::Result<Resource, Failure>
where
    P: crate::load_kingdom_glyph::Players + crate::film_header_entries::Players,
    C: FnMut() -> Result<[u32; 2], String>,
{
    let mut source = Actual {
        process,
        context,
        header_failure: None,
    };
    match sequence(&mut source) {
        Ok((creator, header)) => Ok(Resource::from_constructed_fields(
            ids.to_vec(),
            header,
            creator,
        )),
        Err(failed) => Err(Failure {
            error: failed.error,
            creator: failed.creator,
            header: failed.header,
            header_failure: source.header_failure,
        }),
    }
}

trait Fields {
    type Creator;
    type Header;
    fn clone_creator(&mut self) -> Result<Self::Creator, String>;
    fn construct_header(&mut self) -> Result<Self::Header, String>;
    fn capture_world(&mut self, header: &mut Self::Header) -> Result<(), String>;
    fn mark_mode_three(&mut self, header: &mut Self::Header);
}
struct Prefix<C, H> {
    error: String,
    creator: Option<C>,
    header: Option<H>,
}
fn sequence<F: Fields>(
    source: &mut F,
) -> Result<(F::Creator, F::Header), Prefix<F::Creator, F::Header>> {
    let creator = source.clone_creator().map_err(|error| Prefix {
        error,
        creator: None,
        header: None,
    })?;
    let mut header = match source.construct_header() {
        Ok(header) => header,
        Err(error) => {
            return Err(Prefix {
                error,
                creator: Some(creator),
                header: None,
            });
        }
    };
    if let Err(error) = source.capture_world(&mut header) {
        return Err(Prefix {
            error,
            creator: Some(creator),
            header: Some(header),
        });
    }
    source.mark_mode_three(&mut header);
    Ok((creator, header))
}
struct Actual<'p, 'context, 'a, 'assets, P, C> {
    process: &'p mut film_process_metadata::State,
    context: &'context mut film_world_capture::Context<'a, 'assets, P, C>,
    header_failure: Option<film_header_owner::Failure>,
}
impl<P, C> Fields for Actual<'_, '_, '_, '_, P, C>
where
    P: crate::load_kingdom_glyph::Players + crate::film_header_entries::Players,
    C: FnMut() -> Result<[u32; 2], String>,
{
    type Creator = Box<crate::resource_creator::Creator>;
    type Header = film_header_owner::State;
    fn clone_creator(&mut self) -> Result<Self::Creator, String> {
        self.context
            .world
            .resource_load
            .as_ref()
            .ok_or("film constructor requires actual current MapResource owner")?
            .creator()?
            .clone_for_resource(self.context.catalog, self.context.bank)
    }
    fn construct_header(&mut self) -> Result<Self::Header, String> {
        match film_header_owner::State::from_film_constructor(
            self.context.catalog,
            self.context.bank,
            self.process,
            || (self.context.current_local_filetime)(),
        ) {
            Ok(header) => Ok(header),
            Err(failed) => {
                let error = failed.error().into();
                self.header_failure = Some(failed);
                Err(error)
            }
        }
    }
    fn capture_world(&mut self, header: &mut Self::Header) -> Result<(), String> {
        crate::film_header_fields::capture(header, self.context)
    }
    fn mark_mode_three(&mut self, header: &mut Self::Header) {
        header.mark_film_mode_5a31d0();
    }
}

///One mutable concrete factory context avoids competing World borrows between
///construction and5A3250's later post-publication name read.
pub struct Current<'p, 'a, 'assets, P, C> {
    ids: Vec<u16>,
    process: &'p mut film_process_metadata::State,
    context: film_world_capture::Context<'a, 'assets, P, C>,
}
impl<'p, 'a, 'assets, P, C> Current<'p, 'a, 'assets, P, C> {
    pub fn new(
        ids: &[u16],
        process: &'p mut film_process_metadata::State,
        context: film_world_capture::Context<'a, 'assets, P, C>,
    ) -> Self {
        Self {
            ids: ids.to_vec(),
            process,
            context,
        }
    }
}
impl<P, C> crate::film_create::Constructor for Current<'_, '_, '_, P, C>
where
    P: crate::load_kingdom_glyph::Players + crate::film_header_entries::Players,
    C: FnMut() -> Result<[u32; 2], String>,
{
    fn construct(&mut self) -> std::result::Result<Resource, Failure> {
        construct_current_in(&self.ids, self.process, &mut self.context)
    }
    fn recording_name(&mut self) -> Result<Vec<u16>, String> {
        Ok(self
            .context
            .world
            .resource_load
            .as_ref()
            .ok_or("5A3250 current Resource owner is absent")?
            .target_header()?
            .recording_name()?
            .to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Current {
        input: serde_json::Value,
        events: Vec<String>,
    }
    impl Current {
        fn event(&mut self, name: &str) -> Result<(), String> {
            if self.input["stop"].as_u64() == Some(self.events.len() as u64) {
                return Err(name.into());
            }
            self.events.push(name.into());
            Ok(())
        }
    }
    //Explicit move-only transport tokens, never native pointer conversions.
    struct Creator;
    struct Header(u32);
    impl Fields for Current {
        type Creator = Creator;
        type Header = Header;
        fn clone_creator(&mut self) -> Result<Creator, String> {
            self.event("clone")?;
            Ok(Creator)
        }
        fn construct_header(&mut self) -> Result<Header, String> {
            self.event("header")?;
            Ok(Header(self.input["before"].as_u64().unwrap() as u32))
        }
        fn capture_world(&mut self, header: &mut Header) -> Result<(), String> {
            self.event("capture")?;
            header.0 = self.input["captured"].as_u64().unwrap() as u32;
            Ok(())
        }
        fn mark_mode_three(&mut self, header: &mut Header) {
            header.0 = 3;
        }
    }
    #[test]
    fn original_film_constructor_order_mode_store_and_required_child_prefixes() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-film/native-film-factory.json"
        ))
        .unwrap();
        let rows = fixture["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 36);
        for row in rows {
            let mut source = Current {
                input: row["input"].clone(),
                events: Vec::new(),
            };
            let (creator, header, stopped) = match sequence(&mut source) {
                Ok((c, h)) => (Some(c), Some(h), None),
                Err(f) => (f.creator, f.header, Some(f.error)),
            };
            assert_eq!(
                serde_json::to_value(&source.events).unwrap(),
                row["output"]["events"]
            );
            assert_eq!(
                serde_json::to_value(stopped).unwrap(),
                row["output"]["stopped"]
            );
            assert_eq!(creator.is_some(), row["output"]["creator_installed"]);
            assert_eq!(header.is_some(), row["output"]["header_installed"]);
            let mode = header.map_or(source.input["before"].as_u64().unwrap() as u32, |h| h.0);
            assert_eq!(u64::from(mode), row["output"]["mode"]);
        }
    }
}
