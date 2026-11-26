#!/usr/bin/env python3
"""Run a candidate and the criterion over one of the three ERAVON corpora and
report every divergence, and every disagreement with the generator's
prediction.  Usage: compare.py terms|link|dossier [corpus-dir]; the
optional corpus-dir names a corpus generated elsewhere (a seed other than
the committed one).  The candidate is named
by ERT_CANDIDATE, ERL_CANDIDATE, or ERD_CANDIDATE (an executable path); with
it unset, the criterion is compared to the predictions alone.  Run from
anywhere; paths in the manifest are relative to the corpus root."""
import collections
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, '..', 'galeed-conformance', 'impl-py'))
from glcanon import Obj, canon  # noqa: E402

LAWS = {
    'terms': ('terms-corpus', os.path.join('terms-py', 'ert.py'), 'ERT_CANDIDATE'),
    'link': ('link-corpus', os.path.join('link-py', 'erl.py'), 'ERL_CANDIDATE'),
    'dossier': ('dossier-corpus', os.path.join('dossier-py', 'erd.py'), 'ERD_CANDIDATE'),
}


def _obj(v):
    if isinstance(v, dict):
        return Obj([(k, _obj(x)) for k, x in v.items()])
    if isinstance(v, list):
        return [_obj(x) for x in v]
    return v


def run(cmd, args, cwd):
    try:
        p = subprocess.run(cmd + args, capture_output=True, timeout=120, cwd=cwd)
    except subprocess.TimeoutExpired:
        return ('TIMEOUT', b'')
    if p.returncode != 0:
        # the diagnostic on stderr is outside the contract; only the exit code
        # and the (empty) stdout are compared
        return ('EXIT%d' % p.returncode, p.stdout[:300])
    return ('OK', p.stdout)


def main():
    if len(sys.argv) not in (2, 3) or sys.argv[1] not in LAWS:
        print('usage: compare.py terms|link|dossier [corpus-dir]', file=sys.stderr)
        return 2
    corpus_dir, crit, env = LAWS[sys.argv[1]]
    corpus = os.path.abspath(sys.argv[2]) if len(sys.argv) == 3 else os.path.join(HERE, corpus_dir)
    py = ['python3', os.path.join(HERE, crit)]
    cand = os.environ.get(env)
    cand_cmd = [cand] if cand else None
    m = json.load(open(os.path.join(corpus, 'manifest.json')))
    summary = collections.Counter()
    div = []
    predmiss = []
    for c in m['cases']:
        args = c['argv'] if 'argv' in c else [c['cmd']] + c['args']
        label = c.get('cmd') or (args[0] if args else '(none)')
        c['_label'] = label
        b = run(py, args, corpus)
        summary[label] += 1
        if cand_cmd:
            a = run(cand_cmd, args, corpus)
            if a != b:
                div.append((c, a, b))
                summary[label + '_div'] += 1
                continue
        pred = c.get('predicted')
        if pred is None:
            if b != ('EXIT2', b''):
                predmiss.append((c['path'] + (' [' + c['slug'] + ']' if 'slug' in c else ''), 'EXIT2', b, c.get('note', '')[:80]))
            continue
        label_p = c['path'] + (' [' + c['slug'] + ']' if 'slug' in c else '')
        if b[0] != 'OK':
            predmiss.append((label_p, pred, b, c.get('note', '')[:80]))
            continue
        # the prediction is compared as canonical bytes, so integer spelling,
        # escaping and a trailing newline all count (member order is canonical)
        if canon(_obj(pred)) != b[1]:
            try:
                got = json.loads(b[1])
            except Exception:
                got = b[1][:300]
            predmiss.append((label_p, pred, got, c.get('note', '')[:80]))
    print('=== mode', 'candidate versus criterion' if cand_cmd else 'criterion versus predictions only (no candidate)')
    print('=== summary')
    for k in sorted(summary):
        print(' ', k, summary[k])
    print('=== divergences', len(div) if cand_cmd else 'not measured (no candidate)')
    groups = collections.defaultdict(list)
    for c, a, b in div:
        groups[(c['_label'], a[0], a[1][:160], b[0], b[1][:160])].append(c['path'] + (' [' + c['slug'] + ']' if 'slug' in c else ''))
    for key, paths in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        print('\n--- %s x%d e.g. %s' % (key[0], len(paths), ', '.join(paths[:3])))
        print('  cand:', key[1], key[2])
        print('  py  :', key[3], key[4])
    print('=== differs from prediction', len(predmiss))
    for p, pred, got, note in predmiss[:60]:
        print('\n---', p, '|', note)
        print('  predicted:', json.dumps(pred, ensure_ascii=False)[:300])
        print('  got      :', json.dumps(got, ensure_ascii=False, default=str)[:300])
    return 1 if div or predmiss else 0


if __name__ == '__main__':
    sys.exit(main())
