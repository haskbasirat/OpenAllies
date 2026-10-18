import json,sys,re,collections
def summarize(path):
    d=json.load(open(path))
    groups=collections.OrderedDict()
    pats=[('1 class mismatch',r'^source class [0-9]+ differs from definition class [0-9]+$'),
          ('2 missing definition',r'missing definition'),
          ('3 resource ipcs',r'Resource IDS ipcs'),
          ('4 ambiguous template',r'ambiguous template'),
          ('5 ambiguous Thing IDS',r'ambiguous Thing IDS'),
          ('6 construction economy',r'Construction without Economy')]
    res={k:[0,set()] for k,_ in pats}
    total=0
    for m in d['maps']:
        for f in m.get('failures',[]):
            total+=1
            for k,p in pats:
                if re.search(p,f['error']):
                    res[k][0]+=1; res[k][1].add(m['map'])
    return {k:(v[0],len(v[1])) for k,v in res.items()}, total, d['maps_without_failures']
for p in sys.argv[1:]:
    print(p, summarize(p))
