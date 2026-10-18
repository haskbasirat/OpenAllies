"""Draft Rust schema functions from the recorded native read traces.

Only for descriptors whose normalized token sequence is identical in every
recorded run. Each draft is then reviewed against the native decoder (see the
`native:` addresses in its doc comment) before being accepted; descriptors
with data-dependent structure are written by hand in payload/complex.rs.
Output: crates/aa-orders/src/payload/{game,lobby,editor}.rs (overwritten only
when --write is given).
"""
from pathlib import Path
import json, sys
HERE = Path(__file__).resolve().parent; ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from tokens import tokens
COMPLEX = {22, 23, 51, 70, 64, 83, 91, 93, 94, 95, 321, 322, 84, 85, 86, 87, 117, 118, 122, 123, 177, 196, 293, 296, 356, 357}
CHECK = {'Thing': 'Any', 'ThingB4': 'B4', 'ThingA8': 'A8', 'ThingB0': 'B0', 'ThingAC': 'Ac', 'ThingBC': 'Bc', 'ThingC0': 'C0'}
def group(i):
    if i <= 99: return 'game'
    if 154 <= i <= 297: return 'editor'
    return 'lobby'
def code(tok):
    k = tok[0]
    if k == 'U': return f's.uint({tok[1]})?;'
    if k == 'Str': return 's.named(NameKind::RandomMapConfig)?;' if tok[1] == '0076a9c0' else 's.text()?;'
    if k == 'Def': return f's.def(Registry({tok[1]}))?;'
    if k == 'Bounded': return f's.bounded({tok[1]})?;'
    if k in CHECK: return f's.thing(ThingCheck::{CHECK[k]})?;'
    if k == 'Player': return 's.player()?;'
    if k == 'NetAddr': return 's.net_addr()?;'
    raise ValueError(tok)
def main(write):
    d = json.load(open(HERE / 'order-traces.json'))
    managers = {c + 1: g for g, c in d['managers'].items()}
    static = {r['id']: r for r in json.load(open(HERE / 'static-handler-calls.json'))}
    bodies = {}; members = {}; fns = {}
    variable = []
    for x in d['descriptors']:
        i = x['id']
        if i in COMPLEX: continue
        seqs = {tuple(tokens(r['reads'], managers)) for r in x['runs']}
        if len(seqs) != 1:
            variable.append((i, x['name'], len(seqs))); continue
        seq = next(iter(seqs))
        body = tuple(code(t) for t in seq)
        key = (group(i), body)
        members.setdefault(key, []).append(x)
        natives = sorted({e['fn'] for r in x['runs'] for e in r['reads']})
        fns.setdefault(key, set()).update(natives)
    out = {'game': [], 'lobby': [], 'editor': []}
    table = {'game': [], 'lobby': [], 'editor': []}
    for (g, body), xs in sorted(members.items(), key=lambda kv: kv[1][0]['id']):
        name = f"p{xs[0]['id']}"
        doc = [f"/// {x['id']} {x['name']} (handler {x['handler']}{', process ' + static[x['id']].get('process') if static[x['id']].get('process') else ''})" for x in xs]
        nat = ', '.join('0x' + f.lstrip('0').upper() for f in sorted(fns[(g, body)]))
        doc.append(f'/// native reads: {nat or "none (empty payload)"}')
        arg = 's' if body else '_s'
        lines = '\n'.join(doc) + f'\npub(super) fn {name}({arg}: &mut dyn Ser) -> Result<()> {{\n'
        for b in body: lines += f'    {b}\n'
        lines += '    Ok(())\n}\n'
        out[g].append(lines)
        for x in xs: table[g].append((x['id'], name))
    header = '''//! {title} order payload schemas, drafted from the original-reader traces
//! (research/phase8-orders/gen_schemas.py) and reviewed against the native
//! decoders named in each doc comment. Wire order is the native read order.
#![allow(clippy::unnecessary_wraps)]
use super::{{NameKind, Registry, Ser, ThingCheck}};
use crate::Result;

pub(super) fn schema(descriptor: usize) -> Option<super::Schema> {{
    Some(match descriptor {{
{arms}
        _ => return None,
    }})
}}

'''
    titles = {'game': 'In-game and lobby-core (descriptors 0..=99)', 'lobby': 'Session/lobby (100..=153, 298..=357)', 'editor': 'Scenario editor (154..=297)'}
    for g in out:
        arms = '\n'.join(f'        {i} => {n},' for i, n in sorted(table[g]))
        text = header.format(title=titles[g], arms=arms) + '\n'.join(out[g])
        if 'NameKind::' not in text.split('pub(super) fn schema', 1)[1]:
            text = text.replace('use super::{NameKind, Registry, Ser, ThingCheck};', 'use super::{Registry, Ser, ThingCheck};')
        target = ROOT / f'crates/aa-orders/src/payload/{g}.rs'
        if write: target.write_text(text)
        print(g, len(table[g]), 'descriptors', len(out[g]), 'functions')
    print('variable (manual):', variable)
if __name__ == '__main__':
    main('--write' in sys.argv)
