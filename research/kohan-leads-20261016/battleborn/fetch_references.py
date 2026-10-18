"""Fetch pinned selected text references; never execute repository code."""
from pathlib import Path
import hashlib
import json
import urllib.request

REPO='Ensrick/kohan2-battleborn'
COMMIT='7f911fa8da5d53c1b7d0264f1a03d0cfab88f250'
TREE='f486540655236cb190d3ddaf4c809db92ee2cca1'
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
DEST=ROOT.parent/'reconstruction/references/kohan-leads-20261016/battleborn'
SELECTED={
    'README.md':'project source/mod distinction and stated native loading clues',
    'LICENSE':'exact repository license and TimeGate content exclusion notice',
    'CONTRIBUTING.md':'project content/provenance notices',
    'CHECKLIST.md':'author test and unresolved scope record',
    'CHANGELOG.md':'mod changes and observed startup failures',
    'mod.json':'package metadata and compatibility claims',
    'tools/animations.py':'static animation reference checker, read as text only',
    'tools/load_order.py':'static definition load-order checker, read as text only',
    'tools/template_fields.py':'static inherited template checker, read as text only',
    'tools/validate.py':'reference validator, read as text only',
    'tools/rwd.py':'archive format notes, read as text only',
    'tools/bbpatch.py':'delta patch format/content policy, read as text only',
    'tools/bb.py':'mod tooling scope and startup mount notes, read as text only',
}

def get(url):
    request=urllib.request.Request(url,headers={'User-Agent':'AA-Rust-pinned-reference-review'})
    with urllib.request.urlopen(request,timeout=30) as response:return response.read()

def sha(data):return hashlib.sha256(data).hexdigest()

def main():
    if DEST.exists():raise SystemExit(f'Refusing to overwrite existing reference directory: {DEST}')
    metadata=get(f'https://api.github.com/repos/{REPO}')
    commit=get(f'https://api.github.com/repos/{REPO}/commits/{COMMIT}')
    tree=get(f'https://api.github.com/repos/{REPO}/git/trees/{TREE}?recursive=1')
    committed=json.loads(commit);inventory=json.loads(tree)
    assert committed['sha']==COMMIT and committed['commit']['tree']['sha']==TREE
    assert not inventory['truncated']
    blobs={entry['path']:entry for entry in inventory['tree'] if entry['type']=='blob'}
    records=[]
    data=[]
    for path,purpose in SELECTED.items():
        entry=blobs[path];url=f'https://raw.githubusercontent.com/{REPO}/{COMMIT}/{path}'
        content=get(url);assert len(content)==entry['size']
        assert b'\0' not in content,'selected reference must be text'
        digest=hashlib.sha1(b'blob '+str(len(content)).encode()+b'\0'+content).hexdigest()
        assert digest==entry['sha'],path
        content.decode('utf-8')
        data.append((path,content))
        records.append({'path':f'files/{path}','url':url,'bytes':len(content),
                        'git_blob_sha1':digest,'sha256':sha(content),'purpose':purpose})
    DEST.mkdir(parents=True)
    for path,content in data:
        output=DEST/'files'/path;output.parent.mkdir(parents=True,exist_ok=True);output.write_bytes(content)
    for name,content in [('repository-metadata.json',metadata),('commit.json',commit),('tree.json',tree)]:
        (DEST/name).write_bytes(content)
        records.append({'path':name,'bytes':len(content),'sha256':sha(content),'purpose':'pinned GitHub API provenance snapshot'})
    manifest={'schema':1,'repository':f'https://github.com/{REPO}','commit':COMMIT,'tree':TREE,
              'commit_date':committed['commit']['committer']['date'],'selected_text_only':True,
              'repository_code_executed':False,'installed_or_integrated':False,
              'downloaded_executables_archives_DLLs_game_assets':False,
              'license_detection':json.loads(metadata).get('license'),
              'license_reading_required':'LICENSE text is authoritative; API reports NOASSERTION',
              'files':records,'fetcher_sha256':sha(Path(__file__).read_bytes())}
    (DEST/'download-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps({'directory':str(DEST),'commit':COMMIT,'text_files':len(SELECTED),
                      'bytes':sum(len(content) for _,content in data),
                      'manifest_sha256':sha((DEST/'download-manifest.json').read_bytes())}))

if __name__=='__main__':main()
