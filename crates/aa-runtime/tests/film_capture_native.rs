use aa_mapworld::header::{HeaderEntry, MapHeader, Rule, WorldParameters};
use aa_runtime::film_header_capture::{self, Result, World};
use serde_json::{Value, json};
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
impl World for Current {
    type Campaign = u32;
    fn current_local_filetime(&mut self) -> Result<[u32; 2]> {
        self.event("time")?;
        Ok([0x55667788, 0x11223344])
    }
    fn mode_10(&mut self) -> Result<u32> {
        self.event("mode")?;
        Ok(0x11223344)
    }
    fn game_type_14(&mut self) -> Result<Vec<u16>> {
        self.event("game_type")?;
        Ok(vec![0x7788, 0x5566])
    }
    fn copy_rules_18(&mut self, target: &mut Vec<Rule>) -> Result<()> {
        self.event("copy_rules")?;
        target.push(Rule {
            key: vec![],
            enabled: 0,
            kind: 1,
            value: 0xabc,
        });
        self.extra = 0xdeadbeef;
        Ok(())
    }
    fn rule_extra_24(&mut self) -> Result<u32> {
        self.event("rule_extra")?;
        Ok(self.extra)
    }
    fn capture_parameters_510eb0(&mut self, target: &mut WorldParameters) -> Result<()> {
        self.event("parameters")?;
        target.version = 0x789;
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
    fn current_campaign_4f1310(&mut self) -> Result<Option<u32>> {
        self.event("current_campaign")?;
        Ok(self.input["current"].as_bool().unwrap().then_some(1))
    }
    fn lookup_campaign_411790(&mut self, create: bool) -> Result<Option<u32>> {
        self.event("lookup_campaign")?;
        assert_eq!(create, self.input["current"].as_bool().unwrap());
        *self.events.last_mut().unwrap() = json!({"name":"lookup_campaign","create":create});
        Ok(self.input["lookup"].as_bool().unwrap().then_some(2))
    }
    fn campaign_name_08(&mut self, handle: u32) -> Result<Vec<u16>> {
        self.event("campaign_name")?;
        assert_eq!(handle, 1);
        Ok("CurrentCampaign".encode_utf16().collect())
    }
    fn campaign_index_14(&mut self, handle: u32) -> Result<u32> {
        self.event("campaign_index")?;
        assert_eq!(handle, 2);
        Ok(0xaabbccdd)
    }
    fn capture_entries_5a17d0(
        &mut self,
        _: &mut WorldParameters,
        target: &mut Vec<HeaderEntry>,
    ) -> Result<()> {
        self.event("entries")?;
        target.push(HeaderEntry {
            flag: 0,
            string_20: vec![0x456],
            string_18: vec![],
            flag_v5: Some(0),
        });
        Ok(())
    }
}
#[test]
fn original_v14_all_pc_current_header_lazy_reads_and_each_required_prefix() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-capture.json"
    ))
    .unwrap();
    let seed: Value = serde_json::from_str(include_str!(
        "../../../research/skirmish-20261017-film/native-film-header.json"
    ))
    .unwrap();
    let rows = data["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 51);
    for row in rows {
        let mut header: MapHeader =
            serde_json::from_value(seed["cases"][0]["input"].clone()).unwrap();
        header.words_18_1c = [0xcccccccc; 2];
        header.rules.mode = 0xcccccccc;
        header.rules.game_type = vec![0xcccc; 2];
        header.rules.rules.clear();
        header.parameters.version = 0;
        header.entries.clear();
        header.display_extents_bits = Some([0xcccccccc; 2]);
        header.string_38 = "unchanged original id".encode_utf16().collect();
        header.string_3c = "old campaign".encode_utf16().collect();
        header.word_40 = Some(0xcccccccc);
        let mut extra = 0xcccccccc;
        let mut current = Current {
            input: row["input"].clone(),
            events: vec![],
            extra: 0x12345678,
            width: 0x80000000,
            height: 0x7fc01234,
        };
        let result = film_header_capture::capture(&mut header, &mut extra, &mut current);
        let game_type =
            u32::from(header.rules.game_type[0]) | (u32::from(header.rules.game_type[1]) << 16);
        let output = json!({"events":current.events,"stopped":result.err(),"time":header.words_18_1c,
   "mode":header.rules.mode,"game_type":game_type,"rule_list":header.rules.rules.first().map_or(0,|r|r.value),
   "rule_extra":extra,"parameters":header.parameters.version,"extents":header.display_extents_bits.unwrap(),
   "campaign":String::from_utf16(&header.string_3c).unwrap(),"index":header.word_40.unwrap(),
   "entries":header.entries.first().map_or(0,|r|u32::from(r.string_20[0])),"original_id":String::from_utf16(&header.string_38).unwrap()});
        assert_eq!(output, row["output"], "{} {}", row["input"], row["pc"]);
    }
}
