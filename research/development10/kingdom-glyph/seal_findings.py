"""Seal frozen glyph author sources and completed native/Rust evidence."""
from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
H=lambda b:hashlib.sha256(b).hexdigest()
def pin(p):
 p=Path(p);return {'path':p.relative_to(ROOT).as_posix(),'sha256':H(p.read_bytes()),'bytes':p.stat().st_size}
def main():
 log=(HERE/'tests-02.log').read_text()
 assert 'test result: ok.'in log and '0 failed; 0 ignored'in log
 sources=[ROOT/'crates/aa-runtime/src'/n for n in ['load_kingdom_glyph.rs','load_kingdom_glyph_tests.rs','player_glyph.rs']]
 known=['88f964976f9d46bd217a900bb0991b37419fee03b0c6444a6674abb5aa34e7ce','5571eed20d51a32825bdaa43cffbeed326082b9f883ed601d98b1b9a6e157331','618b8838b4993e597da763bedb9d3e13dc58444a44a4e634610b63557e47a580']
 for p,h in zip(sources,known):assert H(p.read_bytes())==h,p
 for name in ['native-final-immutable.log','native-character-immutable.log','general-contract.log']:assert (HERE/name).read_text().strip()
 native=json.loads((HERE/'native-kingdom-glyph.json').read_text());actual=json.loads((HERE/'native-actual-palette.json').read_text())
 snapshot=HERE/'source-snapshot';snapshot.mkdir(exist_ok=True)
 for p in sources:
  q=snapshot/p.name
  if q.exists():assert q.read_bytes()==p.read_bytes()
  else:q.write_bytes(p.read_bytes())
 doc={'status':'author-frozen-validated','scope':'Native4E93C0 current Kingdom/player/glyph/palette formatter; not completed actual film/launch/WorldReady or playable battle.',
 'sources':[pin(p)for p in sources],'shared_export':'Only appended pub mod load_kingdom_glyph; no exclusive lib.rs freeze.',
 'native':{'selection_rows':len(native['rows']),'format_rows':len(native['format_rows']),'color_rows':len(native['color_rows']),'selection_color_executed_pins':len(native['instructions']),'crt_executed_pins':len(native['crt_instructions']),'character_rows':60,'character_executed_pins':232,'machines':'Four reused source/CRT image instances in main producer; two image instances in character producer. Rows are function invocations, not fresh-machine counts. Both executables/all3PC.','seams':'4B2E60 variable lookup,7666C0 player child,48D500 Glyph lookup; CString storage transport; actualCRT96C7EC executes separately with9738A9 C-locale thread-data leaf. Replaced entries are excluded from executed pins.'},
 'actual_palette':{'count':actual['palette_count'],'native_rows':len(actual['rows']),'source_precision':actual['source_precision'],'scope':'Actual registered source values/variable owner guards + originalCRT differential, not a completed liveActorStore film call.'},
 'rust_summary':[s for s in log.splitlines()if 'test result:'in s],
 'limits':['Nonfinite RGB and%cFFFF WEOF failure explicitly unsupported','Positive online-account glyph remains required unless actual peer/manager services exist','Current Formatter/variables must persist at actual application lifetime','No completed actual full-map film call or visual/rendering parity claim'],
 'artifacts':[pin(p)for p in sorted(HERE.iterdir())if p.is_file()and p.suffix in ['.py','.json','.log','.txt']and p.name not in ['findings.json','seal.log']],
 'preservation':'No original assets,archives,saves,profiles,releases or workspace manifests/lock modified by this component.'}
 p=HERE/'findings.json';p.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'sha256':H(p.read_bytes()),'rust':doc['rust_summary'],'palette':actual['palette_count']}))
if __name__=='__main__':main()
