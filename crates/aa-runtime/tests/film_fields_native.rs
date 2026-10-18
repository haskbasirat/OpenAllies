use aa_runtime::film_header_fields::{self, Destination, Result, Source};
use serde_json::{Value, json};
struct Definition(u32); //A move-only explicit fixture token, not a native address cast.
struct Header {
    time: [u32; 2],
    mode: u32,
    game: Definition,
    rules: u32,
    extra: u32,
    parameters: u32,
    extents: [u32; 2],
    campaign: Vec<u16>,
    index: u32,
    entries: u32,
    stores: Vec<&'static str>,
}
impl Destination for Header {
    type GameType = Definition;
    type Rules = u32;
    type Parameters = u32;
    type Entries = u32;
    fn set_time_high(&mut self, v: u32) {
        self.stores.push("high");
        self.time[1] = v
    }
    fn set_time_low(&mut self, v: u32) {
        self.stores.push("low");
        self.time[0] = v
    }
    fn set_rule_mode(&mut self, v: u32) {
        self.mode = v
    }
    fn set_game_type(&mut self, v: Definition) {
        self.game = v
    }
    fn rules(&mut self) -> &mut u32 {
        &mut self.rules
    }
    fn set_rule_extra(&mut self, v: u32) {
        self.extra = v
    }
    fn parameters(&mut self) -> &mut u32 {
        &mut self.parameters
    }
    fn set_width(&mut self, v: u32) {
        self.extents[0] = v
    }
    fn set_height(&mut self, v: u32) {
        self.extents[1] = v
    }
    fn set_campaign_name_3c(&mut self, v: Vec<u16>) {
        self.campaign = v
    }
    fn set_campaign_index_40(&mut self, v: u32) {
        self.index = v
    }
    fn parameters_and_entries(&mut self) -> (&mut u32, &mut u32) {
        (&mut self.parameters, &mut self.entries)
    }
}
struct Current {
    input: Value,
    events: Vec<Value>,
    extra: u32,
    width: u32,
    height: u32,
}
impl Current {
    fn event(&mut self, name: &str) -> Result<()> {
        if self.input["stop"].as_u64() == Some(self.events.len() as u64) {
            return Err(name.into());
        }
        self.events.push(json!({"name":name}));
        Ok(())
    }
}
struct Campaign(u32); //Opaque/move-only handles need noCopy orClone.
impl Source<Header> for Current {
    type Campaign = Campaign;
    fn current_local_filetime(&mut self) -> Result<[u32; 2]> {
        self.event("time")?;
        Ok([0x55667788, 0x11223344])
    }
    fn mode_10(&mut self) -> Result<u32> {
        self.event("mode")?;
        Ok(0x11223344)
    }
    fn game_type_14(&mut self) -> Result<Definition> {
        self.event("game_type")?;
        Ok(Definition(0x55667788))
    }
    fn copy_rules_18(&mut self, target: &mut u32) -> Result<()> {
        self.event("copy_rules")?;
        *target = 0xabc;
        self.extra = 0xdeadbeef;
        Ok(())
    }
    fn rule_extra_24(&mut self) -> Result<u32> {
        self.event("rule_extra")?;
        Ok(self.extra)
    }
    fn capture_parameters_510eb0(&mut self, target: &mut u32) -> Result<()> {
        self.event("parameters")?;
        *target = 0x789;
        self.width = 0x7f800001;
        self.height = 0xff800000;
        Ok(())
    }
    fn extent_3c(&mut self) -> Result<u32> {
        self.event("width")?;
        Ok(self.width)
    }
    fn extent_40(&mut self) -> Result<u32> {
        self.event("height")?;
        Ok(self.height)
    }
    fn current_campaign_4f1310(&mut self) -> Result<Option<Campaign>> {
        self.event("current_campaign")?;
        Ok(self.input["current"]
            .as_bool()
            .unwrap()
            .then_some(Campaign(1)))
    }
    fn lookup_campaign_411790(&mut self, create: bool) -> Result<Option<Campaign>> {
        self.event("lookup_campaign")?;
        assert_eq!(create, self.input["current"].as_bool().unwrap());
        *self.events.last_mut().unwrap() = json!({"name":"lookup_campaign","create":create});
        Ok(self.input["lookup"]
            .as_bool()
            .unwrap()
            .then_some(Campaign(2)))
    }
    fn campaign_name_08(&mut self, p: &Campaign) -> Result<Vec<u16>> {
        self.event("campaign_name")?;
        assert_eq!(p.0, 1);
        Ok("CurrentCampaign".encode_utf16().collect())
    }
    fn campaign_index_14(&mut self, p: &Campaign) -> Result<u32> {
        self.event("campaign_index")?;
        assert_eq!(p.0, 2);
        Ok(0xaabbccdd)
    }
    fn capture_entries_5a17d0(&mut self, _: &mut u32, target: &mut u32) -> Result<()> {
        self.event("entries")?;
        *target = 0x456;
        Ok(())
    }
}
#[test]
fn original_v14_all_pc_fresh_field_owner_order_without_chunk_identity() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-capture.json"
    ))
    .unwrap();
    assert_eq!(data["cases"].as_array().unwrap().len(), 51);
    for row in data["cases"].as_array().unwrap() {
        let mut target = Header {
            time: [0xcccccccc; 2],
            mode: 0xcccccccc,
            game: Definition(0xcccccccc),
            rules: 0,
            extra: 0xcccccccc,
            parameters: 0,
            extents: [0xcccccccc; 2],
            campaign: "old campaign".encode_utf16().collect(),
            index: 0xcccccccc,
            entries: 0,
            stores: vec![],
        };
        let mut source = Current {
            input: row["input"].clone(),
            events: vec![],
            extra: 0x12345678,
            width: 0x80000000,
            height: 0x7fc01234,
        };
        let result = film_header_fields::capture(&mut target, &mut source);
        assert_eq!(
            target.stores,
            if source.events.is_empty() {
                vec![]
            } else {
                vec!["high", "low"]
            }
        );
        assert_eq!(
            json!({"events":source.events,"stopped":result.err(),"time":target.time,"mode":target.mode,"game_type":target.game.0,"rule_list":target.rules,"rule_extra":target.extra,"parameters":target.parameters,"extents":target.extents,"campaign":String::from_utf16(&target.campaign).unwrap(),"index":target.index,"entries":target.entries,"original_id":"unchanged original id"}),
            row["output"],
            "{} {}",
            row["input"],
            row["pc"]
        );
    }
}
