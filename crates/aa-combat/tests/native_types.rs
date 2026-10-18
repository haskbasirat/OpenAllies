use aa_body::F32;
use aa_combat::types::{DamageTypePresentation, Source, load};
use aa_combat::{Error, Result};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
struct Input {
    name: String,
    initial: DamageTypePresentation,
    color: [F32; 3],
    glyph: u32,
    animation_filter: i32,
}
struct Fields {
    color: [F32; 3],
    glyph: u32,
    filter: i32,
    events: Vec<&'static str>,
}
impl Source for Fields {
    fn load_base_definition(&mut self) -> Result<()> {
        self.events.push("base");
        Ok(())
    }
    fn color(&mut self) -> Result<[F32; 3]> {
        self.events.push("color");
        Ok(self.color)
    }
    fn glyph(&mut self) -> Result<u32> {
        self.events.push("glyph");
        Ok(self.glyph)
    }
    fn animation_filter(&mut self) -> Result<i32> {
        self.events.push("animation_filter");
        Ok(self.filter)
    }
}

#[test]
fn original_damage_type_writes_and_exception_order_match() {
    let document: Value = serde_json::from_str(include_str!(
        "../../../research/phase6-weapon/native-damage-type-fixtures.json"
    ))
    .unwrap();
    let cases = document["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 8);
    for row in cases {
        let input: Input = serde_json::from_value(row["input"].clone()).unwrap();
        let mut state = input.initial;
        let mut source = Fields {
            color: input.color,
            glyph: input.glyph,
            filter: input.animation_filter,
            events: vec![],
        };
        let error = match load(&mut state, &mut source) {
            Ok(()) => None,
            Err(Error::Invalid(message)) => Some(message),
            Err(other) => panic!("{other}"),
        };
        assert_eq!(
            json!({"state":state,"events":source.events,"error":error}),
            row["output"],
            "{}",
            input.name
        );
    }
}

#[test]
fn nonfinite_color_does_not_publish_unusable_metadata() {
    let mut state = DamageTypePresentation {
        color: [F32::ZERO; 4],
        glyph: 7,
        animation_filter: 3,
    };
    let original = state.clone();
    let mut source = Fields {
        color: [F32(f32::NAN.to_bits()), F32::ZERO, F32::ONE],
        glyph: 1,
        filter: 1,
        events: vec![],
    };
    assert!(load(&mut state, &mut source).is_err());
    assert_eq!(state, original);
    assert_eq!(source.events, vec!["base", "color"]);
}
