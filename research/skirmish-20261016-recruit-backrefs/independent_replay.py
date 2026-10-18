"""Read-only independent original/v14 replay of frozen Recruit fixtures.

Does not call the producer main(), recapture inputs, or overwrite its JSON.
"""
from pathlib import Path
import sys,json,hashlib,time
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from native_recruit_backrefs import Backrefs,MODES,ORIGINAL,ACCEPTED,sha

def main():
    checked=[]
    for name in ['native-recruit-backrefs.json','native-real-recruit-backrefs.json']:
        path=HERE/name; original=path.read_bytes(); doc=json.loads(original)
        for window in doc['function_windows'].values():
            start,end=int(window['start'],16),int(window['end'],16)
            raw=ORIGINAL.at(start,end-start)
            assert raw==ACCEPTED.at(start,end-start)
            assert sha(raw)==window['sha256']
        modes=dict(MODES); start=time.monotonic()
        for index,row in enumerate(doc['cases']):
            actual=json.loads(json.dumps(Backrefs(modes[row['precision']],row['input'],row['image']).run()))
            assert actual==row['output'],(name,index,actual,row['output'])
        assert path.read_bytes()==original,'review must not mutate native evidence'
        checked.append(dict(file=name,cases=len(doc['cases']),windows=len(doc['function_windows']),
                            sha256=sha(original),seconds=round(time.monotonic()-start,3)))
        print(json.dumps(checked[-1]),flush=True)
    (HERE/'independent-replay.json').write_text(json.dumps(dict(checked=checked,fixture_files_unchanged=True),indent=2)+'\n')

if __name__=='__main__':main()
