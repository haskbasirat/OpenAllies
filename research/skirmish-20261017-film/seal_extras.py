from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
names=['crates/aa-runtime/src/film_parameter_extras.rs','crates/aa-runtime/src/film_parameters.rs',
 'crates/aa-runtime/src/film_kingdom_parameters.rs','crates/aa-runtime/tests/film_extras_native.rs',
 'research/skirmish-20261017-film/native_film_extras.py','research/skirmish-20261017-film/native-film-extras.json',
 'research/skirmish-20261017-film/extras-tests.log','research/skirmish-20261017-film/extras-current-bindings-check.log',
 'research/skirmish-20261017-film/native-extras-final-replay.log','research/skirmish-20261017-film/EXTRAS.txt']
doc=dict(schema=1,date='2026-10-17',status='bounded trigger objective extras replay passed; latest concrete helper wiring compiled',
 pins=[dict(file=n,sha256=hashlib.sha256((ROOT/n).read_bytes()).hexdigest())for n in names],
 native=dict(paired_rows=90,image_sessions=180,block_pins=263,pc=[24,53,64],seams=['4F8F00 named Kingdom lookup','4BAA20 existing text entry lookup']),
 rust=dict(passed=1,failed=0,log='extras-tests.log',later_additive_bindings='production check passed; no complete managed positive run'),
 limitations=['Explicit already-captured parameter table input at511297, not full510EB0.',
 'No native heap failure or SEH unwinding proof; invalid destination explicit cutoff.',
 'Positive external localized parameter companion and missing Text entry creation remain required.',
 'No complete current Header/resource/recording/battle/save/multiplayer claim.'])
out=HERE/'extras-findings.json';out.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(sha256=hashlib.sha256(out.read_bytes()).hexdigest(),production=doc['pins'][:3]),indent=2))
