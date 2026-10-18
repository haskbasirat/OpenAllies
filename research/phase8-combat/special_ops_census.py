"""Text census of the shipped special operations (Commander/Faction abilities)
for the round-5 inventory. Read-only over the extracted original Data tree;
this is a guide to coverage, NOT a loader: definitions are resolved by the
native-path loader in aa-rules (abilities::native_faction_ability_effects).

Writes special-ops-census.json:
* generals: Faction ids -> AASpecialOp templates they list (Generals/*/*.tgi)
* templates: AASpecialOp* template -> ability_ids / cost_faction_experience
  (Templates/template_ability_special_op_aa.tgi)
* operations: SpecialOps/*.tgi file -> AbilityDefinition IDS, effect templates,
  modifier templates, [Flags] keys, recharge_time values, FXC hooks
* totals across operations.
"""
from collections import Counter
from pathlib import Path
import hashlib
import json
import re

HERE = Path(__file__).resolve().parent
DATA = HERE.parents[2] / 'reconstruction/assets/original/Data'


def text(path):
    return re.sub(r';;[^\n]*', '', path.read_text(encoding='latin-1'))


def blocks(source, header):
    """[Header ...] { body } pairs (body without nested braces resolution)."""
    out = []
    for m in re.finditer(r'\[' + header + r'([^\]]*)\]', source):
        rest = source[m.end():]
        start = rest.find('{')
        if start < 0 or rest[:start].strip():
            out.append((m.group(1).strip(), ''))
            continue
        depth = 0
        for i, ch in enumerate(rest[start:], start):
            depth += ch == '{'
            depth -= ch == '}'
            if depth == 0:
                out.append((m.group(1).strip(), rest[start + 1:i]))
                break
    return out


def main():
    generals = {}
    for path in sorted((DATA / 'Generals').rglob('*.tgi')):
        src = text(path)
        for head, body in blocks(src, 'Faction'):
            ids = re.search(r'^\s*ids\s*=\s*(\S+)', body, re.M | re.I)
            ops = re.findall(r'\[Ability\s+Template=(\w+)\]', body, re.I)
            generals[ids.group(1) if ids else path.stem] = {'file': str(path.relative_to(DATA)), 'templates': ops}
    templates = {}
    src = text(DATA / 'Templates/template_ability_special_op_aa.tgi')
    for m in re.finditer(r'\[Template\s+(\w+)\s+Template=(\w+)\]\s*\{([^}]*)\}', src, re.I):
        body = m.group(3)
        templates[m.group(1)] = {
            'parent': m.group(2),
            'ability_ids': (re.search(r'ability_ids\s*=\s*(\S+)', body) or [None, None])[1],
            'cost_faction_experience': (re.search(r'cost_faction_experience\s*=\s*(\S+)', body) or [None, None])[1],
        }
    operations = {}
    totals = {k: Counter() for k in ('effects', 'modifiers', 'flags', 'fxc')}
    for path in sorted((DATA / 'SpecialOps').glob('*.tgi')):
        src = text(path)
        row = {
            'ids': re.findall(r'^\s*IDS\s*=\s*(\S+)', src, re.M),
            'effects': re.findall(r'\[Effect\s+Template=(\w+)', src, re.I),
            'modifiers': re.findall(r'\[Modifier\s+Template=(\w+)', src, re.I),
            'flags': sorted({k for b in re.findall(r'\[Flags\]([^\[]*)', src, re.I)
                             for k in re.findall(r'(\w+)\s*=', b)}),
            'recharge_time': re.findall(r'recharge_time\s*=\s*(\S+)', src),
            'fxc': re.findall(r'\[(When\w+)\s+Template=(\w+)', src, re.I),
            'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
        }
        row['fxc'] = [f'{a}:{b}' for a, b in row['fxc']]
        operations[path.stem] = row
        totals['effects'].update(row['effects'])
        totals['modifiers'].update(row['modifiers'])
        totals['flags'].update(row['flags'])
        totals['fxc'].update(row['fxc'])
    doc = {
        'schema': 1,
        'scope': __doc__,
        'generals': generals,
        'templates': templates,
        'operations': operations,
        'totals': {k: dict(v.most_common()) for k, v in totals.items()},
    }
    (HERE / 'special-ops-census.json').write_text(json.dumps(doc, indent=1) + '\n')
    print(json.dumps({'generals': len(generals), 'templates': len(templates), 'operations': len(operations),
                      **{k: sum(v.values()) for k, v in totals.items()}}))


if __name__ == '__main__':
    main()
