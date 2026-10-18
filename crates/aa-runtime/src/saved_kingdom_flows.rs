//! Native 4E48C0/4E4930 flow changes on a saved Kingdom's authoritative storage.
//!
//! This adapter does not complete GKingdom::Init (4E5DF0). Its three flow vectors
//! must have been initialized by the actual lifecycle before an accessed index
//! can exist. Constructor-only vectors remain empty and fail at the first
//! reached access. No initialization, resize, ownership transfer or rollback is
//! performed here; earlier stores survive a later missing dependency.
use aa_economy::{Catalog, EconomyError, F32Bits, ResourceKind, borrowed_flows};
use aa_mapworld::special_state::SpecialShell;

type Result<T> = std::result::Result<T, EconomyError>;

/// GameData 48A8E0 embeds Resource manager 47F670 at +10; that constructor
/// installs B3A660=this. Consequently B3A660+28 and B36530+38 name the same
/// count word. This reads the bound registry count, never a Kingdom vector.
pub fn consumption_count(definitions: &crate::kingdom_economy::Definitions) -> Result<i32> {
    i32::try_from(definitions.resource_count()).map_err(|_| EconomyError::IntegerRange)
}

/// The integer SVar cached at B39AB0+1B4 (native fixed index109). 50880C
/// registers its initial150; sequential Settings4B3C10 assignments overwrite
/// that same Int variable. Existing native wildcard proof establishes
/// SVars.tgi before SVars_AA.tgi. Call this lazily from provider_maximum, after
/// the raw store. Any newly authored WorldRule override remains unsupported.
pub fn provider_maximum(
    catalog: &crate::catalog::Catalog<'_>,
    _precision: aa_numeric::Precision,
) -> Result<i32> {
    let fail = |reason: &str| EconomyError::World(format!("EconomyLimitedResourceMax: {reason}"));
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or_else(|| fail("native WorldRule registry is unavailable"))?;
    if registry
        .entries
        .iter()
        .any(|row| row.ids.eq_ignore_ascii_case("EconomyLimitedResourceMax"))
    {
        return Err(fail(
            "WorldRule override is outside the proven source scope",
        ));
    }
    let rows = catalog
        .settings
        .as_ref()
        .ok_or_else(|| fail("source settings are unavailable"))?
        .declarations
        .get("economylimitedresourcemax")
        .ok_or_else(|| fail("source declarations are unavailable"))?;
    if rows.len() != 2 {
        return Err(fail("both exact source declarations are required"));
    }
    let mut effective = None;
    for path in ["Game/SVars.tgi", "Game/SVars_AA.tgi"] {
        let mut matches = rows
            .iter()
            .filter(|row| row.source.path.eq_ignore_ascii_case(path));
        let row = matches
            .next()
            .ok_or_else(|| fail("source declaration is missing"))?;
        if matches.next().is_some() {
            return Err(fail("source declaration is ambiguous"));
        }
        if !row
            .field_type
            .as_deref()
            .is_some_and(|kind| kind.eq_ignore_ascii_case("int"))
        {
            return Err(fail("source declaration must be Int"));
        }
        let mut files = catalog
            .actors
            .sources
            .iter()
            .filter(|file| file.path.eq_ignore_ascii_case(path));
        let file = files
            .next()
            .ok_or_else(|| fail("Actor source identity is absent"))?;
        if files.next().is_some() || file.sha256 != row.source.sha256 {
            return Err(fail("Settings and Actor source identities differ"));
        }
        effective = Some(
            row.raw
                .trim()
                .parse::<i32>()
                .map_err(|_| fail("unsupported signed decimal integer"))?,
        );
    }
    effective.ok_or_else(|| fail("no source value"))
}

/// Required source and owner bindings for the native borrowed arithmetic.
/// `resources` is in native B36530 registry order. `consumption_count` denotes
/// the B3A660 manager count, which aliases the Resource registry count; it is
/// never inferred from a Kingdom vector length. The cap is
/// lazy: 4E3940 reaches it only after storing raw production for a limited
/// provider. Stockpiles at +1C4 are neither copied nor inspected.
pub struct BorrowedKingdom<'a> {
    pub kingdom: &'a mut SpecialShell,
    pub resources: &'a Catalog,
    pub consumption_count: i32,
    pub provider_maximum: &'a mut dyn FnMut() -> Result<i32>,
}

fn missing(offset: u32, index: Option<usize>) -> EconomyError {
    EconomyError::World(match index {
        Some(index) => format!(
            "saved Kingdom vector {offset:x}[{index}] is absent; actual Init/storage binding is required"
        ),
        None => format!("saved Kingdom vector {offset:x} is absent"),
    })
}
impl BorrowedKingdom<'_> {
    fn read(&self, offset: u32, index: usize) -> Result<F32Bits> {
        self.kingdom
            .vectors
            .get(&offset)
            .ok_or_else(|| missing(offset, None))?
            .values
            .get(index)
            .copied()
            .map(F32Bits)
            .ok_or_else(|| missing(offset, Some(index)))
    }
    fn write(&mut self, offset: u32, index: usize, value: F32Bits) -> Result<()> {
        let target = self
            .kingdom
            .vectors
            .get_mut(&offset)
            .ok_or_else(|| missing(offset, None))?
            .values
            .get_mut(index)
            .ok_or_else(|| missing(offset, Some(index)))?;
        *target = value.0;
        Ok(())
    }
}
impl borrowed_flows::World for BorrowedKingdom<'_> {
    fn consumption_count(&self) -> Result<i32> {
        Ok(self.consumption_count)
    }
    fn resource_count(&self) -> Result<i32> {
        i32::try_from(self.resources.resources.len()).map_err(|_| EconomyError::IntegerRange)
    }
    fn effective_count(&self) -> Result<i32> {
        let vector = self
            .kingdom
            .vectors
            .get(&0x1a0)
            .ok_or_else(|| missing(0x1a0, None))?;
        i32::try_from(vector.values.len()).map_err(|_| EconomyError::IntegerRange)
    }
    fn effective_provided(&self, index: usize) -> Result<F32Bits> {
        self.read(0x1a0, index)
    }
    fn raw_provided(&self, index: usize) -> Result<F32Bits> {
        self.read(0x1ac, index)
    }
    fn consumed(&self, index: usize) -> Result<F32Bits> {
        self.read(0x1b8, index)
    }
    fn set_effective_provided(&mut self, index: usize, value: F32Bits) -> Result<()> {
        self.write(0x1a0, index, value)
    }
    fn set_raw_provided(&mut self, index: usize, value: F32Bits) -> Result<()> {
        self.write(0x1ac, index, value)
    }
    fn set_consumed(&mut self, index: usize, value: F32Bits) -> Result<()> {
        self.write(0x1b8, index, value)
    }
    fn resource_kind(&self, index: usize) -> Result<ResourceKind> {
        self.resources
            .resources
            .get(index)
            .map(|resource| resource.kind)
            .ok_or(EconomyError::ResourceIndex)
    }
    fn purchase_cost(&self, source: usize, destination: usize) -> Result<F32Bits> {
        self.resources
            .resources
            .get(source)
            .ok_or(EconomyError::ResourceIndex)?
            .purchase_cost
            .get(destination)
            .copied()
            .ok_or(EconomyError::Shape)
    }
    fn limited_provider(&self, index: usize) -> Result<bool> {
        self.resources
            .resources
            .get(index)
            .map(|resource| resource.limited_provider)
            .ok_or(EconomyError::ResourceIndex)
    }
    fn provider_maximum(&mut self) -> Result<i32> {
        (self.provider_maximum)()
    }
}
