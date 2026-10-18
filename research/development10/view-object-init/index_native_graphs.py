"""Index exact path/source/converted pairs; identical NIF bytes may be aliases.

by_source is authoritative. graphs is a historical first-canonical-source view
by content SHA, which alone cannot choose a different path's render artifact.
The index is published atomically after all referenced graph files exist.
"""
from pathlib import Path,PurePosixPath
import json,hashlib,re,uuid,argparse
ROOT=Path(__file__).resolve().parents[3]
ARTIFACT=re.compile(r'[0-9a-f]{64}(?:-[0-9a-f]{16})?\.json\Z')

def build_index(folder,paths=None):
 graphs={};by_source={}
 for path in sorted(paths if paths is not None else folder.glob('*.json')):
  if path.name=='index.json':continue
  if not ARTIFACT.fullmatch(path.name)or path.is_symlink():raise ValueError(f'Unrecognized raw graph artifact: {path}')
  raw=path.read_bytes();d=json.loads(raw)
  source=d['source'];parts=PurePosixPath(source).parts
  if not parts or '\\'in source or ':'in source or source.startswith('/')or any(x in('', '.', '..')for x in source.split('/')):raise ValueError(f'Invalid canonical source: {source}')
  if d['schema']!=2 or d['source_sha256']!=path.name[:64]:raise ValueError(f'Graph schema/source SHA differs: {path}')
  row=dict(source=source,source_sha256=d['source_sha256'],artifact=path.name,sha256=hashlib.sha256(raw).hexdigest())
  key=source.lower()
  if key in by_source:raise ValueError(f'Duplicate canonical source graph: {source}')
  by_source[key]=row
 for row in sorted(by_source.values(),key=lambda r:r['source']):graphs.setdefault(row['source_sha256'],row)
 return dict(schema=2,scope='Paired raw source binary32 graph fields; by_source preserves canonical path/converted pairing. Source coverage is not native processing, render or game readiness.',raw_graph_count=len(by_source),distinct_source_sha256=len(graphs),graphs=graphs,by_source=by_source)

def write_atomic(path,doc):
 path.parent.mkdir(parents=True,exist_ok=True)
 if path.is_symlink():raise ValueError(f'Refusing linked output: {path}')
 raw=(json.dumps(doc,indent=2,ensure_ascii=False)+'\n').encode()
 temporary=path.with_name(path.name+'.'+uuid.uuid4().hex+'.tmp')
 with temporary.open('xb')as f:f.write(raw)
 temporary.replace(path)
 return hashlib.sha256(raw).hexdigest()

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--check',action='store_true');args=parser.parse_args()
 folder=ROOT/'assets-cache/model-native-graphs';doc=build_index(folder);path=folder/'index.json'
 if args.check:
  assert json.loads(path.read_bytes())==doc;digest=hashlib.sha256(path.read_bytes()).hexdigest()
 else:digest=write_atomic(path,doc)
 print(json.dumps(dict(raw_graph_count=doc['raw_graph_count'],distinct_source_sha256=doc['distinct_source_sha256'],index_sha256=digest)))
if __name__=='__main__':main()
