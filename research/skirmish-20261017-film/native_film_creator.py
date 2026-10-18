"""Actual CreatorLoad512C30 writer including base512A00 and class tag.
Null/non-null SAMEsourceResource+50, low16 count/truncation and current base
fields. Only stream writes/text-entry collection are required service seams.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE))
from native_film_header import h,run,sample
def main():
 rows=[]
 for seed in [0,1]:
  for source in [dict(present=False,ids=[]),dict(present=True,ids=[]),dict(present=True,ids=[ord(c)for c in 'Maps/Campaign/tutorial02.RMP']),dict(present=True,ids=[65]*65537)]:
   header=sample(seed);header['creator']['class_name']=[ord(c)for c in 'KKC_WorldCreatorLoad']
   for pc in h.PRECISION:
    a=run(header,'original',pc,source);b=run(header,'v14',pc,source);assert a==b
    rows.append(dict(input=header,source=source,pc=pc,output=a))
 doc=dict(schema=1,scope=__doc__,cases=rows,instructions=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-creator.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(h.VISITED),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
