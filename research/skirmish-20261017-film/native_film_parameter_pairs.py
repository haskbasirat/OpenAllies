"""Native50D730+49AA30 pair-aware writer. Each5A1120 seam sees BOTH actual
CString words after the key's low16-length stream writes. Raw stream write and
collection are explicit seams; all write/collection prefixes are covered.
Nativeformat only, no depot/filesystem creation or text-entry creation claim.
"""
from pathlib import Path
import hashlib,json,sys
import native_film_header as H
HERE=Path(__file__).resolve().parent
def case(seed):
 header=H.sample(seed);pairs=dict(teams=[[0,0xd800,65535]for _ in header['parameters']['teams']],
  kingdoms=[dict(text=[84,0,0xdfff],entries=[[65+j,0,0xd800]for j,_ in enumerate(k['localized_entries'])])for k in header['parameters']['kingdoms']])
 return dict(header=header,pairs=pairs)
def cases():
 yield case(0);yield case(1)
 c=case(1);c['header']['parameters']['teams'][0]['display_key']=[75]*65537;yield c
 c=case(1)
 n=len(H.run(c['header'],'original','Double53',pair_texts=c['pairs'],parameters_only=True)['events'])
 for stop in range(n):yield dict(c,stop=stop)
def main():
 rows=[]
 for case in cases():
  for pc in H.h.PRECISION:
   args=dict(pair_texts=case['pairs'],parameters_only=True,stop=case.get('stop'))
   a=H.run(case['header'],'original',pc,**args);b=H.run(case['header'],'v14',pc,**args);assert a==b
   rows.append(dict(input=case,pc=pc,output=a))
 doc=dict(schema=1,scope=__doc__,cases=rows,instructions=[dict(va=hex(a),bytes=b)for a,b in sorted(H.h.VISITED.items())])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-parameter-pairs.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(H.h.VISITED),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
