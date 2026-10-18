use aa_economy::{
    Catalog, EconomyError, F32Bits, ResourceKind, State, X87Precision,
    borrowed_flows::{self, World},
};
use serde_json::{Value, json};
type Result<T> = std::result::Result<T, EconomyError>;

/// The implementation owns no flow arrays: its setters operate on the exact
/// borrowed Vec allocations supplied by the test's state owner.
struct Borrowed<'a> {
    catalog: &'a Catalog,
    raw: &'a mut [F32Bits],
    effective: &'a mut [F32Bits],
    consumed: &'a mut [F32Bits],
    consumption_count: i32,
    resource_count: i32,
    effective_count: i32,
    maximum: i32,
    failure: &'a Value,
    writes: Vec<Value>,
}
fn missing() -> EconomyError {
    EconomyError::World("explicit fixture dependency unavailable".into())
}
fn get(values: &[F32Bits], index: usize) -> Result<F32Bits> {
    values.get(index).copied().ok_or(EconomyError::Shape)
}
impl Borrowed<'_> {
    fn before_write(&self) -> Result<()> {
        if self.failure["write"].as_u64() == Some(self.writes.len() as u64) {
            Err(missing())
        } else {
            Ok(())
        }
    }
}
impl World for Borrowed<'_> {
    fn consumption_count(&self) -> Result<i32> {
        Ok(self.consumption_count)
    }
    fn resource_count(&self) -> Result<i32> {
        Ok(self.resource_count)
    }
    fn effective_count(&self) -> Result<i32> {
        Ok(self.effective_count)
    }
    fn effective_provided(&self, i: usize) -> Result<F32Bits> {
        get(self.effective, i)
    }
    fn raw_provided(&self, i: usize) -> Result<F32Bits> {
        get(self.raw, i)
    }
    fn consumed(&self, i: usize) -> Result<F32Bits> {
        if self.failure["consumed_missing"] == true {
            return Err(missing());
        }
        get(self.consumed, i)
    }
    fn set_effective_provided(&mut self, i: usize, value: F32Bits) -> Result<()> {
        self.before_write()?;
        *self.effective.get_mut(i).ok_or(EconomyError::Shape)? = value;
        self.writes.push(json!(["effective_provided", i, value.0]));
        Ok(())
    }
    fn set_raw_provided(&mut self, i: usize, value: F32Bits) -> Result<()> {
        self.before_write()?;
        *self.raw.get_mut(i).ok_or(EconomyError::Shape)? = value;
        self.writes.push(json!(["raw_provided", i, value.0]));
        Ok(())
    }
    fn set_consumed(&mut self, i: usize, value: F32Bits) -> Result<()> {
        self.before_write()?;
        *self.consumed.get_mut(i).ok_or(EconomyError::Shape)? = value;
        self.writes.push(json!(["consumed", i, value.0]));
        Ok(())
    }
    fn resource_kind(&self, i: usize) -> Result<ResourceKind> {
        self.catalog
            .resources
            .get(i)
            .map(|v| v.kind)
            .ok_or(EconomyError::ResourceIndex)
    }
    fn purchase_cost(&self, source: usize, destination: usize) -> Result<F32Bits> {
        if self.failure["coefficient"] == true {
            return Err(missing());
        }
        get(
            &self
                .catalog
                .resources
                .get(source)
                .ok_or(EconomyError::ResourceIndex)?
                .purchase_cost,
            destination,
        )
    }
    fn limited_provider(&self, i: usize) -> Result<bool> {
        self.catalog
            .resources
            .get(i)
            .map(|v| v.limited_provider)
            .ok_or(EconomyError::ResourceIndex)
    }
    fn provider_maximum(&mut self) -> Result<i32> {
        if self.failure["maximum"] == true {
            Err(missing())
        } else {
            Ok(self.maximum)
        }
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../research/skirmish-20261016-borrowed-flows/native-borrowed-flows.json"
    ))
    .unwrap()
}
fn view<'a>(c: &'a Value, catalog: &'a Catalog, state: &'a mut State) -> Borrowed<'a> {
    let count = catalog.resources.len() as i32;
    Borrowed {
        catalog,
        raw: &mut state.raw_provided,
        effective: &mut state.effective_provided,
        consumed: &mut state.consumed,
        consumption_count: c["consumption_count"].as_i64().map_or(count, |v| v as i32),
        resource_count: c["resource_count"].as_i64().map_or(count, |v| v as i32),
        effective_count: c["effective_count"].as_i64().map_or(count, |v| v as i32),
        maximum: c["limits"]["provided_maximum"].as_i64().unwrap() as i32,
        failure: &c["failure"],
        writes: vec![],
    }
}
#[test]
fn original_and_v14_array_stores_and_partial_errors_match() {
    let doc = fixture();
    let rows = doc["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 1026);
    for row in rows {
        let c = &row["input"];
        let catalog: Catalog = serde_json::from_value(c["catalog"].clone()).unwrap();
        let mut state: State = serde_json::from_value(c["initial"].clone()).unwrap();
        let ptrs = [
            state.raw_provided.as_ptr(),
            state.effective_provided.as_ptr(),
            state.consumed.as_ptr(),
        ];
        let pc: X87Precision = serde_json::from_value(row["precision"].clone()).unwrap();
        let mut borrowed = view(c, &catalog, &mut state);
        let action = &c["action"];
        let mut values: Vec<F32Bits> = action
            .get("values")
            .map(|v| serde_json::from_value(v.clone()).unwrap())
            .unwrap_or_default();
        if let Some(length) = c["failure"]["input_length"].as_u64() {
            values.truncate(length as usize)
        }
        let result = match action["kind"].as_str().unwrap() {
            "Provider" => borrowed_flows::adjust_provider(
                &mut borrowed,
                action["index"].as_u64().unwrap() as usize,
                F32Bits(action["delta"].as_u64().unwrap() as u32),
                pc,
            ),
            "Providers" => borrowed_flows::adjust_providers(
                &mut borrowed,
                &values,
                action["add"].as_bool().unwrap(),
                pc,
            ),
            "Consumption" => borrowed_flows::adjust_consumption(
                &mut borrowed,
                &values,
                action["add"].as_bool().unwrap(),
                pc,
            ),
            _ => panic!("unknown action"),
        };
        assert_eq!(
            result.is_ok(),
            row["output"]["outcome"] == "ok",
            "{row} {result:?}"
        );
        assert_eq!(json!(borrowed.writes), row["output"]["writes"], "{row}");
        assert_eq!(
            serde_json::to_value(&state).unwrap(),
            row["output"]["state"],
            "{row}"
        );
        assert_eq!(
            ptrs,
            [
                state.raw_provided.as_ptr(),
                state.effective_provided.as_ptr(),
                state.consumed.as_ptr()
            ]
        );
    }
}

#[test]
fn invalid_unreached_input_tail_is_not_prevalidated() {
    let doc = fixture();
    let c = &doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "provider_missing_maximum")
        .unwrap()["input"];
    let catalog: Catalog = serde_json::from_value(c["catalog"].clone()).unwrap();
    let mut state: State = serde_json::from_value(c["initial"].clone()).unwrap();
    let mut c = c.clone();
    c["failure"] = Value::Null;
    c["consumption_count"] = json!(1);
    let mut borrowed = view(&c, &catalog, &mut state);
    borrowed_flows::adjust_consumption(
        &mut borrowed,
        &[F32Bits::ZERO, F32Bits(0x7fc00000)],
        true,
        X87Precision::Double53,
    )
    .unwrap();
}

#[test]
fn unsupported_overflow_preserves_completed_shortage_stores() {
    let doc = fixture();
    let c = &doc["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["input"]["name"] == "provider_missing_maximum")
        .unwrap()["input"];
    let catalog: Catalog = serde_json::from_value(c["catalog"].clone()).unwrap();
    let mut state: State = serde_json::from_value(c["initial"].clone()).unwrap();
    // This explicit finite-domain boundary stops before a binary32 infinity
    // store. It is not a claim that the native machine throws on overflow.
    state.raw_provided[1] = F32Bits(0x7f7fffff);
    let mut borrowed = view(c, &catalog, &mut state);
    assert_eq!(
        borrowed_flows::adjust_provider(
            &mut borrowed,
            1,
            F32Bits(0x7f7fffff),
            X87Precision::Double53
        ),
        Err(EconomyError::NonFiniteResult)
    );
    assert_eq!(borrowed.writes.len(), 2);
    assert_eq!(borrowed.consumed[0], F32Bits::from_f32(5.0));
    assert_eq!(borrowed.consumed[2], F32Bits::from_f32(5.0));
    assert_eq!(borrowed.raw[1], F32Bits(0x7f7fffff));
}
