#!/usr/bin/env python3
"""Run a candidate gev (GEV_CANDIDATE) and the evidence criterion over the evidence
corpus and report every divergence, and every disagreement with the generator's
prediction. With GEV_CANDIDATE unset, the criterion is compared to the
predictions alone. Run from anywhere; paths in the manifest are relative to
the corpus root. The manifest defines the corpus: every corpus path a case
names must carry a file, except the paths the case lists as `absent`, which
must carry none; a corpus with no case, and an unreadable manifest, count as
a disagreement. The first sixty disagreements are printed; the exit code is
1 where any divergence or disagreement was found, 0 otherwise."""
import collections
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), 'impl-py'))
from glcanon import Obj, canon  # noqa: E402


def _obj(v):
    if isinstance(v, dict):
        return Obj([(k, _obj(x)) for k, x in v.items()])
    if isinstance(v, list):
        return [_obj(x) for x in v]
    return v

HERE = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.path.join(HERE, 'evidence-corpus')
PY = ['python3', os.path.join(HERE, 'evidence-py', 'gev.py')]
CAND = os.environ.get('GEV_CANDIDATE')
RUST = [CAND] if CAND else None


ENV = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')


def run(cmd, args):
    try:
        p = subprocess.run(cmd + args, capture_output=True, timeout=120, cwd=CORPUS, env=ENV)
    except subprocess.TimeoutExpired:
        return ('TIMEOUT', b'')
    if p.returncode != 0:
        # the diagnostic on stderr is outside the contract; only the exit code
        # and the (empty) stdout are compared
        return ('EXIT%d' % p.returncode, p.stdout[:300])
    return ('OK', p.stdout)


summary = collections.Counter(); div = []; predmiss = []
try:
    m = json.load(open(os.path.join(CORPUS, 'manifest.json')))
    cases = m['cases']
    if not isinstance(cases, list):
        raise ValueError('no cases list')
except Exception as ex:
    m = {}; cases = []
    predmiss.append(('(manifest)', 'a manifest.json with a cases list', str(ex), CORPUS))
if not cases:
    predmiss.append(('(corpus)', 'cases', 0, CORPUS))
DIRS = {d for d in os.listdir(CORPUS) if os.path.isdir(os.path.join(CORPUS, d))} if os.path.isdir(CORPUS) else set()
for c in cases:
    args = c['argv'] if 'argv' in c else [c['cmd']] + c['args']
    # every corpus path the case names carries a file, except those it lists as absent
    absent = set(c.get('absent', []))
    named = [a for a in args if isinstance(a, str) and '/' in a and a.split('/')[0] in DIRS]
    bad = [a for a in named if (a in absent) == os.path.isfile(os.path.join(CORPUS, a))]
    bad += [a for a in absent if a not in named]
    if bad:
        predmiss.append((c.get('path'), 'files present, the absent ones absent', bad, c.get('note', '')[:80]))
        summary['files_disagree'] += 1
        continue
    label = c.get('cmd') or (args[0] if args else '(none)')
    c['_label'] = label
    b = run(PY, args)
    summary[label] += 1
    if RUST:
        a = run(RUST, args)
        if a != b:
            div.append((c, a, b)); summary[label + '_div'] += 1
            continue
    pred = c.get('predicted')
    if pred is None:
        # a misuse case: exit 2 and no JSON
        if b != ('EXIT2', b''):
            predmiss.append((c['path'], 'EXIT2', b, c.get('note', '')[:80]))
        continue
    if b[0] != 'OK':
        predmiss.append((c['path'], pred, b, c.get('note', '')[:80])); continue
    # the prediction is compared as canonical bytes, so member order, integer
    # spelling, escaping and a trailing newline all count
    if canon(_obj(pred)) != b[1]:
        try:
            got = json.loads(b[1])
        except Exception:
            got = b[1][:300]
        predmiss.append((c['path'], pred, got, c.get('note', '')[:80]))
print('=== mode', 'candidate versus criterion' if RUST else 'criterion versus predictions only (no candidate)')
print('=== summary')
for k in sorted(summary):
    print(' ', k, summary[k])
print('=== divergences', len(div) if RUST else 'not measured (no candidate)')
groups = collections.defaultdict(list)
for c, a, b in div:
    groups[(c['_label'], a[0], a[1][:160], b[0], b[1][:160])].append(c['path'])
for key, paths in sorted(groups.items(), key=lambda kv: -len(kv[1])):
    print('\n--- %s x%d e.g. %s' % (key[0], len(paths), ', '.join(paths[:3])))
    print('  cand:', key[1], key[2]); print('  py  :', key[3], key[4])
print('=== differs from prediction', len(predmiss))
for p, pred, got, note in predmiss[:60]:
    print('\n---', p, '|', note)
    print('  predicted:', json.dumps(pred, ensure_ascii=False)[:300])
    print('  got      :', json.dumps(got, ensure_ascii=False, default=str)[:300])
sys.exit(1 if div or predmiss else 0)
