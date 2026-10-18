"""Read-only replay of the frozen null-FX constructor/list/42A590 oracle.
Executes the original producer up to its output writer, then compares its
complete structured output with the immutable fixture. Never opens writer.
"""
from pathlib import Path
import json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[3]
producer=ROOT/'research/skirmish-20261016-load-post-create/native_empty_fx_source.py'
text=producer.read_text();marker="p=HERE/'native-empty-fx-source.json'"
assert text.count(marker)==1
namespace={'__file__':str(producer),'__name__':'reviewed_immutable_replay'}
exec(compile(text[:text.index(marker)],str(producer),'exec'),namespace)
fixture=producer.with_name('native-empty-fx-source.json');expected=json.loads(fixture.read_text())
assert namespace['out']==expected
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
print(json.dumps(dict(scope=__doc__,native_runs=len(expected['cases']),fixture_sha256=H(fixture),producer_sha256=H(producer),replay_sha256=H(Path(__file__)),declared_seams=['5B4220 source count0','4976B0 enum-name registration sink'],result='PASS')))
