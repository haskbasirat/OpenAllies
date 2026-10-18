//!5A3250's fallible current-resource/depot caller. Concrete depot guards own
//!the actual manager lock across this synchronous operation. A backend error
//!never becomes an empty lookup, a warning omission or successful publication.
use crate::{film_resource::Resource, film_resource_factory};
use std::sync::Arc;

///Portable identity of an actual backend depot allocation. Only the runtime
///resource-depot producer may construct it; no raw address or numeric token.
#[derive(Clone, Debug)]
pub struct DepotId {
    identity: Arc<()>,
}
impl DepotId {
    pub(crate) fn from_work_owner() -> Self {
        Self {
            identity: Arc::new(()),
        }
    }
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity)
    }
}
pub trait Depot {
    type Existing;
    type Error;
    fn find(&mut self, ids: &[u16]) -> Result<Option<Self::Existing>, Self::Error>;
    ///5AA3C0: count exactly1 and that entry equals the actual work depot.
    fn only_work_depot(&mut self, existing: &Self::Existing) -> Result<bool, Self::Error>;
    fn warn_existing_outside_work(&mut self, ids: &[u16]) -> Result<(), Self::Error>;
    fn remove(&mut self, existing: Self::Existing) -> Result<(), Self::Error>;
    ///5AA390 then conditional5AAAE0, including its reached vf30 effect.
    fn ensure_work_depot(&mut self, resource: &mut Resource) -> Result<(), Self::Error>;
    fn work_is_topmost(&mut self, resource: &Resource) -> Result<bool, Self::Error>;
    fn warn_work_not_topmost(&mut self, ids: &[u16]) -> Result<(), Self::Error>;
    fn publish(&mut self, resource: &mut Resource) -> Result<(), Self::Error>;
}
pub trait Constructor {
    fn construct(&mut self) -> Result<Resource, film_resource_factory::Failure>;
    fn recording_name(&mut self) -> Result<Vec<u16>, String>;
}
pub enum Failure<E> {
    Depot {
        error: E,
        resource: Option<Resource>,
    },
    Construct(film_resource_factory::Failure),
    RecordingName {
        error: String,
        resource: Resource,
    },
}
impl<E: std::fmt::Display> std::fmt::Display for Failure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Depot { error, .. } => std::fmt::Display::fmt(error, f),
            Self::Construct(error) => std::fmt::Display::fmt(error, f),
            Self::RecordingName { error, .. } => f.write_str(error),
        }
    }
}
impl<E: std::fmt::Debug> std::fmt::Debug for Failure<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Depot { error, resource } => f
                .debug_struct("FilmDepotFailure")
                .field("error", error)
                .field("resource", &resource.is_some())
                .finish(),
            Self::Construct(error) => f
                .debug_tuple("FilmConstructorFailure")
                .field(error)
                .finish(),
            Self::RecordingName { error, .. } => f
                .debug_tuple("FilmRecordingNameFailure")
                .field(error)
                .finish(),
        }
    }
}
///The constructor callback owns the actual5A31D0 inputs and is invoked only
///after any old resource has been removed. The final recording-name read/copy
///occurs AFTER successful publication, matching5A3250's final405B80.
pub fn create<D: Depot>(
    depot: &mut D,
    ids: &[u16],
    construct: impl FnOnce() -> Result<Resource, film_resource_factory::Failure>,
    recording_name: impl FnOnce() -> Result<Vec<u16>, String>,
) -> Result<Resource, Failure<D::Error>> {
    struct Callbacks<C, N> {
        construct: Option<C>,
        name: Option<N>,
    }
    impl<C, N> Constructor for Callbacks<C, N>
    where
        C: FnOnce() -> Result<Resource, film_resource_factory::Failure>,
        N: FnOnce() -> Result<Vec<u16>, String>,
    {
        fn construct(&mut self) -> Result<Resource, film_resource_factory::Failure> {
            (self
                .construct
                .take()
                .expect("single native constructor call"))()
        }
        fn recording_name(&mut self) -> Result<Vec<u16>, String> {
            (self.name.take().expect("single native name read"))()
        }
    }
    create_with(
        depot,
        ids,
        &mut Callbacks {
            construct: Some(construct),
            name: Some(recording_name),
        },
    )
}
pub fn create_with<D: Depot, C: Constructor>(
    depot: &mut D,
    ids: &[u16],
    constructor: &mut C,
) -> Result<Resource, Failure<D::Error>> {
    let mut actual = Actual {
        depot,
        ids,
        constructor,
    };
    sequence(&mut actual).map_err(|failure| match failure {
        SequenceFailure::Depot { error, resource } => Failure::Depot { error, resource },
        SequenceFailure::Construct(error) => Failure::Construct(error),
        SequenceFailure::RecordingName { error, resource } => {
            Failure::RecordingName { error, resource }
        }
    })
}
//The same ordered kernel runs for the concrete borrowed services and the
//native wrapper fixtures. Its generic resource is a laboratory seam only;
//the public path always uses the one real Resource allocation above.
trait Flow {
    type Resource;
    type Existing;
    type Error;
    type ConstructError;
    fn find(&mut self) -> Result<Option<Self::Existing>, Self::Error>;
    fn only_work(&mut self, existing: &Self::Existing) -> Result<bool, Self::Error>;
    fn warn_existing(&mut self) -> Result<(), Self::Error>;
    fn remove(&mut self, existing: Self::Existing) -> Result<(), Self::Error>;
    fn construct(&mut self) -> Result<Self::Resource, Self::ConstructError>;
    fn ensure_work(&mut self, resource: &mut Self::Resource) -> Result<(), Self::Error>;
    fn topmost(&mut self, resource: &Self::Resource) -> Result<bool, Self::Error>;
    fn warn_topmost(&mut self) -> Result<(), Self::Error>;
    fn publish(&mut self, resource: &mut Self::Resource) -> Result<(), Self::Error>;
    fn copy_recording_name(&mut self, resource: &mut Self::Resource) -> Result<(), String>;
}
enum SequenceFailure<R, E, C> {
    Depot { error: E, resource: Option<R> },
    Construct(C),
    RecordingName { error: String, resource: R },
}
fn sequence<F: Flow>(
    flow: &mut F,
) -> Result<F::Resource, SequenceFailure<F::Resource, F::Error, F::ConstructError>> {
    let existing = flow.find().map_err(|error| SequenceFailure::Depot {
        error,
        resource: None,
    })?;
    if let Some(existing) = existing {
        if !flow
            .only_work(&existing)
            .map_err(|error| SequenceFailure::Depot {
                error,
                resource: None,
            })?
        {
            flow.warn_existing()
                .map_err(|error| SequenceFailure::Depot {
                    error,
                    resource: None,
                })?;
        }
        flow.remove(existing)
            .map_err(|error| SequenceFailure::Depot {
                error,
                resource: None,
            })?;
    }
    let mut resource = flow.construct().map_err(SequenceFailure::Construct)?;
    let result = (|| {
        flow.ensure_work(&mut resource)?;
        if !flow.topmost(&resource)? {
            flow.warn_topmost()?;
        }
        flow.publish(&mut resource)
    })();
    if let Err(error) = result {
        return Err(SequenceFailure::Depot {
            error,
            resource: Some(resource),
        });
    }
    if let Err(error) = flow.copy_recording_name(&mut resource) {
        return Err(SequenceFailure::RecordingName { error, resource });
    }
    Ok(resource)
}
struct Actual<'a, D, C> {
    depot: &'a mut D,
    ids: &'a [u16],
    constructor: &'a mut C,
}
impl<D: Depot, C: Constructor> Flow for Actual<'_, D, C> {
    type Resource = Resource;
    type Existing = D::Existing;
    type Error = D::Error;
    type ConstructError = film_resource_factory::Failure;
    fn find(&mut self) -> Result<Option<Self::Existing>, Self::Error> {
        self.depot.find(self.ids)
    }
    fn only_work(&mut self, e: &Self::Existing) -> Result<bool, Self::Error> {
        self.depot.only_work_depot(e)
    }
    fn warn_existing(&mut self) -> Result<(), Self::Error> {
        self.depot.warn_existing_outside_work(self.ids)
    }
    fn remove(&mut self, e: Self::Existing) -> Result<(), Self::Error> {
        self.depot.remove(e)
    }
    fn construct(&mut self) -> Result<Resource, Self::ConstructError> {
        self.constructor.construct()
    }
    fn ensure_work(&mut self, r: &mut Resource) -> Result<(), Self::Error> {
        self.depot.ensure_work_depot(r)
    }
    fn topmost(&mut self, r: &Resource) -> Result<bool, Self::Error> {
        self.depot.work_is_topmost(r)
    }
    fn warn_topmost(&mut self) -> Result<(), Self::Error> {
        self.depot.warn_work_not_topmost(self.ids)
    }
    fn publish(&mut self, r: &mut Resource) -> Result<(), Self::Error> {
        self.depot.publish(r)
    }
    fn copy_recording_name(&mut self, r: &mut Resource) -> Result<(), String> {
        r.set_recording_name_5a3250(&self.constructor.recording_name()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    struct Mock<'a> {
        case: &'a Value,
        events: Vec<String>,
        name: Option<String>,
    }
    impl Mock<'_> {
        fn event(&mut self, name: &str) -> Result<(), String> {
            if self.case["stop"].as_str() == Some(name) {
                return Err(name.into());
            }
            self.events.push(name.into());
            Ok(())
        }
    }
    impl Flow for Mock<'_> {
        type Resource = ();
        type Existing = ();
        type Error = String;
        type ConstructError = String;
        fn find(&mut self) -> Result<Option<()>, String> {
            self.event("find")?;
            Ok(self.case["existing"].as_bool().unwrap().then_some(()))
        }
        fn only_work(&mut self, _: &()) -> Result<bool, String> {
            self.event("only_work")?;
            Ok(self.case["only_work"].as_bool().unwrap())
        }
        fn warn_existing(&mut self) -> Result<(), String> {
            self.event("warn_existing")
        }
        fn remove(&mut self, _: ()) -> Result<(), String> {
            self.event("remove")
        }
        fn construct(&mut self) -> Result<(), String> {
            self.event("construct")?;
            self.name = Some("old-header-name".into());
            Ok(())
        }
        fn ensure_work(&mut self, _: &mut ()) -> Result<(), String> {
            self.event("has_work")?;
            if !self.case["has_work"].as_bool().unwrap() {
                self.event("add_work")?;
            }
            Ok(())
        }
        fn topmost(&mut self, _: &()) -> Result<bool, String> {
            self.event("topmost")?;
            Ok(self.case["topmost"].as_bool().unwrap())
        }
        fn warn_topmost(&mut self) -> Result<(), String> {
            self.event("warn_topmost")
        }
        fn publish(&mut self, _: &mut ()) -> Result<(), String> {
            self.event("publish")
        }
        fn copy_recording_name(&mut self, _: &mut ()) -> Result<(), String> {
            self.event("name")?;
            self.name = Some(
                if self.case["mutate_name"].as_bool().unwrap() {
                    "changed-during-publication"
                } else {
                    "current-map-name"
                }
                .into(),
            );
            Ok(())
        }
    }
    #[test]
    fn original_v14_create_current_tail_and_all_required_service_prefixes() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-film/native-film-create.json"
        ))
        .unwrap();
        let rows = fixture["cases"].as_array().unwrap();
        assert_eq!(rows.len(), 432);
        for row in rows {
            let mut mock = Mock {
                case: &row["input"],
                events: Vec::new(),
                name: None,
            };
            let result = sequence(&mut mock);
            let error = result.err().map(|f| match f {
                SequenceFailure::Depot { error, .. }
                | SequenceFailure::RecordingName { error, .. }
                | SequenceFailure::Construct(error) => error,
            });
            let output = &row["output"];
            let expected = output["events"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| *v != "lock" && *v != "unlock")
                .cloned()
                .collect::<Vec<_>>();
            assert_eq!(
                serde_json::json!(mock.events),
                serde_json::json!(expected),
                "{row}"
            );
            assert_eq!(
                error,
                output["stopped"].as_str().map(str::to_owned),
                "{row}"
            );
            assert_eq!(
                serde_json::json!(mock.name),
                output["recorded_name"],
                "{row}"
            );
        }
    }
}
