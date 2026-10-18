from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
names=[
 'crates/aa-runtime/src/film_parameters.rs','crates/aa-runtime/src/film_kingdom_parameters.rs',
 'crates/aa-runtime/src/film_header_entries.rs','crates/aa-runtime/src/film_header.rs',
 'crates/aa-runtime/src/film_parameter_serialization.rs','crates/aa-runtime/src/film_current_header.rs',
 'crates/aa-runtime/tests/film_kingdoms_native.rs','crates/aa-runtime/tests/film_parameter_pairs_native.rs',
 'crates/aa-runtime/tests/film_entries_native.rs','crates/aa-runtime/tests/film_header_native.rs',
 'research/skirmish-20261017-film/native_film_kingdoms.py','research/skirmish-20261017-film/native-film-kingdoms.json',
 'research/skirmish-20261017-film/native_film_parameter_pairs.py','research/skirmish-20261017-film/native-film-parameter-pairs.json',
 'research/skirmish-20261017-film/native_film_header.py','research/skirmish-20261017-film/native-film-header.json',
 'research/skirmish-20261017-film/current-writer-tests.log','research/skirmish-20261017-film/native-header-pair-extension-replay.log',
 'research/skirmish-20261017-film/native-pairs-final-replay.log','research/skirmish-20261017-film/native-kingdoms-final-replay.log',
 'research/skirmish-20261017-film/KINGDOMS-WRITER.txt']
def pin(n):return dict(file=n,sha256=hashlib.sha256((ROOT/n).read_bytes()).hexdigest())
doc=dict(schema=1,date='2026-10-17',status='bounded current Kingdom capture and pair-aware current-owner writers passed',
 pins=[pin(n)for n in names],native_new=dict(kingdom_rows=156,kingdom_sessions=312,kingdom_block_pins=308,pair_rows=189,pair_sessions=378,pair_instruction_pins=597,precision=[24,53,64]),
 rust=dict(passed=5,failed=0,log='current-writer-tests.log'),
 limitations=['Concrete live Kingdom/source adapter compiled, no complete managed positive capture executed.',
 'Glyph/lookup child seams explicitly retained by native Kingdom fixture.',
 'Trigger objective extras/full510EB0 still required.',
 'No full recording/resource/depot/filesystem/battle/save/multiplayer claim.'])
out=HERE/'kingdoms-writer-findings.json';out.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(sha256=hashlib.sha256(out.read_bytes()).hexdigest(),production=doc['pins'][:6]),indent=2))
