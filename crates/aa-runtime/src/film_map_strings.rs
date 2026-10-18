//!Actual59FDA0 ordering. DataText preparation is unconditional. The current
//!EditorMapStringsSeparate flag gates only the final external save; full
//!embedded text and the prior DeleteFileW call still execute when it is0.
//!Services must borrow actual resource/depot/text owners; no missing service
//!is represented by an empty text stream or an absent-resource default.
pub type Result<T> = std::result::Result<T, String>;
pub trait Services {
    type Text;
    fn prepare(&mut self) -> Result<Self::Text>;
    fn build_current(&mut self, text: &mut Self::Text) -> Result<()>;
    fn ensure_existing_work(&mut self, text: &mut Self::Text) -> Result<()>;
    fn compression(&mut self) -> Result<u8>;
    fn open_map_strings(&mut self, compressed: u8) -> Result<()>;
    fn serialize(&mut self, text: &mut Self::Text) -> Result<()>;
    ///Native859B30's DeleteFileW result is ignored by59FDA0. An unavailable
    ///path/owner/service is distinct from the actual OS returning FALSE.
    fn remove_separate_file(&mut self, text: &mut Self::Text) -> Result<()>;
    fn separate_enabled(&mut self) -> Result<u8>;
    fn save_separate(&mut self, text: &mut Self::Text) -> Result<()>;
    fn release(&mut self, text: &mut Self::Text) -> Result<()>;
}
pub struct Failure<T> {
    error: String,
    text: Option<T>,
}
impl<T> Failure<T> {
    pub fn error(&self) -> &str {
        &self.error
    }
    pub fn text(&self) -> Option<&T> {
        self.text.as_ref()
    }
}
impl<T> std::fmt::Debug for Failure<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilmMapStringsFailure")
            .field("error", &self.error)
            .field("prepared_text", &self.text.is_some())
            .finish()
    }
}
pub fn save<S: Services>(services: &mut S) -> std::result::Result<(), Failure<S::Text>> {
    let mut text = services
        .prepare()
        .map_err(|error| Failure { error, text: None })?;
    let result = (|| {
        services.build_current(&mut text)?;
        services.ensure_existing_work(&mut text)?;
        let compressed = services.compression()?;
        services.open_map_strings(compressed)?;
        services.serialize(&mut text)?;
        services.remove_separate_file(&mut text)?;
        if services.separate_enabled()? != 0 {
            services.save_separate(&mut text)?;
        }
        services.release(&mut text)
    })();
    result.map_err(|error| Failure {
        error,
        text: Some(text),
    })
}

///Closed binding of59FDA0 to the same actual application, film stream,
///current variable cells and live map-text entry table. Text is read at the
///4738C0 build boundary, after native wrapper/path preparation.
pub struct Current<'a> {
    pub application: &'a mut crate::resource_depot::Application,
    pub resource: &'a mut crate::film_resource::Resource,
    pub variables: &'a crate::film_variables::State,
    pub order: &'a std::result::Result<crate::film_text::MapTextOrder, String>,
    pub text: &'a crate::localization::Localization,
}
impl Services for Current<'_> {
    type Text = crate::film_data_text::PreparedDataText;
    fn prepare(&mut self) -> Result<Self::Text> {
        Self::Text::prepare(self.application, self.resource)
    }
    fn build_current(&mut self, text: &mut Self::Text) -> Result<()> {
        let order = self.order.as_ref().map_err(Clone::clone)?;
        text.build_current(&order.serialize_current_logical(self.text)?)
    }
    fn ensure_existing_work(&mut self, text: &mut Self::Text) -> Result<()> {
        text.ensure_existing_work(self.application)
    }
    fn compression(&mut self) -> Result<u8> {
        Ok(self.variables.compressed_byte())
    }
    fn open_map_strings(&mut self, compressed: u8) -> Result<()> {
        self.resource.open_chunk("MAPSTRINGS", compressed)
    }
    fn serialize(&mut self, text: &mut Self::Text) -> Result<()> {
        text.serialize_into(self.resource)
    }
    fn remove_separate_file(&mut self, text: &mut Self::Text) -> Result<()> {
        text.remove_separate_file()
    }
    fn separate_enabled(&mut self) -> Result<u8> {
        Ok(self.variables.separate_map_strings_byte())
    }
    fn save_separate(&mut self, text: &mut Self::Text) -> Result<()> {
        text.save_separate()
    }
    fn release(&mut self, text: &mut Self::Text) -> Result<()> {
        text.release()
    }
}
pub enum PreludeFailure {
    Header(String),
    Text(Failure<crate::film_data_text::PreparedDataText>),
}
impl PreludeFailure {
    pub fn error(&self) -> &str {
        match self {
            Self::Header(error) => error,
            Self::Text(failure) => failure.error(),
        }
    }
}
impl std::fmt::Debug for PreludeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Header(e) => f.debug_tuple("FilmHeaderPreludeFailure").field(e).finish(),
            Self::Text(e) => f.debug_tuple("FilmTextPreludeFailure").field(e).finish(),
        }
    }
}
///Actual5A01E0: Header+Creator first, then MAPSTRINGS. This dispatch never
///serializes a wholeWorld snapshot. A failed text step retains its real owner.
pub fn save_current_map(
    catalog: &crate::catalog::Catalog<'_>,
    bank: &crate::definitions::Bank,
    mut current: Current<'_>,
) -> std::result::Result<(), PreludeFailure> {
    current
        .resource
        .save_header_current(catalog, bank, current.text, current.variables)
        .map_err(PreludeFailure::Header)?;
    save(&mut current).map_err(PreludeFailure::Text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    struct Mock<'a> {
        case: &'a Value,
        events: Vec<String>,
        seen: Vec<u8>,
        separate: u8,
        compressed: u8,
    }
    impl Mock<'_> {
        fn event(&mut self, name: &str) -> Result<()> {
            if self.case["stop"].as_u64() == Some(self.events.len() as u64) {
                return Err(name.into());
            }
            self.events.push(name.into());
            Ok(())
        }
    }
    impl Services for Mock<'_> {
        type Text = ();
        fn prepare(&mut self) -> Result<()> {
            Ok(())
        }
        fn build_current(&mut self, _: &mut ()) -> Result<()> {
            self.event("build")?;
            if let Some(v) = self.case["build_compressed"].as_u64() {
                self.compressed = v as u8;
            }
            Ok(())
        }
        fn ensure_existing_work(&mut self, _: &mut ()) -> Result<()> {
            if self.case["existing"].as_bool().unwrap() {
                self.event("existing_work")?;
                if !self.case["has_work"].as_bool().unwrap() {
                    self.event("add_work")?;
                }
            }
            Ok(())
        }
        fn compression(&mut self) -> Result<u8> {
            self.event("compression")?;
            Ok(self.compressed)
        }
        fn open_map_strings(&mut self, c: u8) -> Result<()> {
            self.event("chunk")?;
            self.seen.push(c);
            Ok(())
        }
        fn serialize(&mut self, _: &mut ()) -> Result<()> {
            self.event("serialize")?;
            if let Some(v) = self.case["serialize_separate"].as_u64() {
                self.separate = v as u8;
            }
            Ok(())
        }
        fn remove_separate_file(&mut self, _: &mut ()) -> Result<()> {
            self.event("delete")?;
            if let Some(v) = self.case["delete_separate"].as_u64() {
                self.separate = v as u8;
            }
            Ok(())
        }
        fn separate_enabled(&mut self) -> Result<u8> {
            self.event("separate")?;
            Ok(self.separate)
        }
        fn save_separate(&mut self, _: &mut ()) -> Result<()> {
            self.event("save")
        }
        fn release(&mut self, _: &mut ()) -> Result<()> {
            self.event("release")
        }
    }
    #[test]
    fn original_v14_late_flag_reads_false_branch_and_all_service_prefixes() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-film/native-film-map-strings.json"
        ))
        .unwrap();
        let mut count = 0;
        for row in fixture["cases"].as_array().unwrap() {
            let case = &row["input"];
            if case["kind"] != "tail" {
                continue;
            }
            let mut mock = Mock {
                case,
                events: Vec::new(),
                seen: Vec::new(),
                separate: case["separate"].as_u64().unwrap() as u8,
                compressed: case["compressed"].as_u64().unwrap() as u8,
            };
            let result = save(&mut mock);
            let output = &row["output"];
            assert_eq!(serde_json::json!(mock.events), output["events"], "{case}");
            assert_eq!(
                serde_json::json!(mock.seen),
                output["chunk_compression"],
                "{case}"
            );
            assert_eq!(
                u64::from(mock.separate),
                output["current_separate"],
                "{case}"
            );
            assert_eq!(
                result.err().map(|e| e.error),
                output["stopped"].as_str().map(str::to_owned),
                "{case}"
            );
            count += 1;
        }
        assert_eq!(count, 396);
    }
}
