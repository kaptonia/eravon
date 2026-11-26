#!/usr/bin/env python3
"""Run a candidate gl1 (GL1_CANDIDATE) and the Python criterion over the corpus
and report every divergence and every disagreement with the generator's
prediction; without a candidate, check the criterion against the predictions
alone and say so. The manifest defines the corpus: every entry is a case,
and the files it names must exist. A case passes only where its comparison
ran to the end and found the two sides equal and equal to the prediction: an
answer that does not parse, a predicted member the answer lacks, an audit
answer of another shape than its predicted label's (`NO_LABEL` is exactly
{"ok":false,"reason":"NO_LABEL"}), an entry lacking a prediction key of its
category, a file the manifest names that is absent, a file of the corpus no
entry names, an unreadable manifest, and a corpus with no case each count as
a disagreement. The divergence
record is written to GL1_DIVERGENCES, or to divergences.json in the working
directory. The exit code is 1 where any divergence or disagreement was
found, 0 otherwise."""
import hashlib, json, os, subprocess, sys, collections
ROOT = os.path.dirname(os.path.abspath(__file__))
CORPUS = os.environ.get('GL1_CORPUS') or os.path.join(ROOT, 'corpus')
OUT = os.environ.get('GL1_DIVERGENCES') or os.path.join(os.getcwd(), 'divergences.json')
PY = ['python3', os.path.join(ROOT, 'impl-py', 'gl1.py')]
RUST = [os.environ['GL1_CANDIDATE']] if os.environ.get('GL1_CANDIDATE') else PY
if RUST is PY:
    print('=== mode criterion versus predictions only (no candidate)')
ENV = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')

def run(cmd, args, timeout=60):
    try:
        p = subprocess.run(cmd + args, capture_output=True, timeout=timeout, env=ENV, cwd=CORPUS)
    except subprocess.TimeoutExpired:
        return ('TIMEOUT', b'')
    if p.returncode not in (0,):
        return ('EXIT%d' % p.returncode, p.stdout[:400])
    return ('OK', p.stdout)

summary = collections.Counter()
divergences = []
predmiss = []

def miss(name, want, got, note=None, key=None):
    if key:
        summary[key] += 1
    predmiss.append((name, want, got, note))

# the manifest defines the corpus: every entry is a case, and its files must exist
entries = []
mp = os.path.join(CORPUS, 'manifest.json')
try:
    m = json.load(open(mp))
    entries = m.get('cases') if isinstance(m, dict) else None
    if not isinstance(entries, list):
        raise ValueError('no cases list')
except Exception as ex:
    miss('(manifest)', 'a manifest.json with a cases list', str(ex), CORPUS, 'manifest_unreadable')
    entries = []

def listed(sub, ok):
    d = os.path.join(CORPUS, sub)
    if not os.path.isdir(d):
        return []
    return [sub + '/' + n for n in sorted(os.listdir(d)) if os.path.isfile(os.path.join(d, n)) and ok(n)]

def counts_of(got):
    """The predictor's count keys read off a report."""
    c = {'entries': got.get('entries'), 'findings': len(got.get('findings', [])),
         'hard': sum(1 for f in got.get('findings', []) if f.get('hard'))}
    for f in got.get('findings', []):
        c['f_' + f['name']] = c.get('f_' + f['name'], 0) + 1
    for k in ('missing', 'anchored', 'excluded', 'unknown_type', 'malformed', 'unproven', 'void', 'discarded'):
        c[k] = len(got.get(k, []))
    return c

def member_of(k):
    """The report member a count key is read off."""
    if k in ('hard',) or k.startswith('f_'):
        return 'findings'
    return k

def both(args):
    a = run(RUST, args)
    b = a if RUST is PY else run(PY, args)
    return a, b

def need(e, keys, name):
    lacking = [k for k in keys if k not in e]
    if lacking:
        miss(name, 'prediction keys ' + ', '.join(lacking), 'absent from the manifest entry', e.get('note'), e.get('category', '?') + '_vs_prediction_diff')
        return False
    return True

def check_canon(e):
    rel = e['path']; name = os.path.basename(rel)
    for cmd_name in ('check', 'canon'):
        a, b = both([cmd_name, rel])
        summary[cmd_name] += 1
        if a != b:
            divergences.append((cmd_name, name, a, b)); summary[cmd_name + '_div'] += 1
            continue
        key = cmd_name + '_vs_prediction_diff'
        if a[0] != 'OK':
            # every check and canon case is answered
            miss(name, 'an answer', a[0], e.get('note'), key); continue
        try:
            got = json.loads(a[1])
        except Exception:
            miss(name, 'a parsable answer', a[1][:80], e.get('note'), cmd_name + '_unparsable'); continue
        if not isinstance(got, dict):
            miss(name, 'an object', a[1][:80], e.get('note'), key); continue
        if cmd_name == 'check':
            pred = e['predicted']; got_t = 'ok' if got.get('ok') is True else got.get('token')
            if pred is None or pred != got_t:
                miss(name, pred, got_t, e.get('note'), key)
            elif pred == 'ok' and e.get('predicted_entry_id') != got.get('entry_id'):
                miss(name, e.get('predicted_entry_id'), got.get('entry_id'), e.get('note'), key)
        else:
            pred = e['predicted_canon']
            got_t = got.get('token') if got.get('ok') is False else ('ok' if got.get('ok') is True else None)
            if pred is None or pred != got_t:
                miss(name, pred, got_t, e.get('note'), key)
            elif pred == 'ok':
                try:
                    got_sha = '0x' + hashlib.sha256(bytes.fromhex(got['canon'][2:])).hexdigest()
                except Exception:
                    got_sha = None
                if e.get('predicted_canon_sha256') is None or got_sha != e['predicted_canon_sha256']:
                    miss(name, e.get('predicted_canon_sha256'), got_sha, e.get('note'), key)

def check_audit(e):
    rel = e['path']; name = os.path.basename(rel)
    a, b = both(['audit', rel])
    summary['audit'] += 1
    if a != b:
        divergences.append(('audit', name, a, b)); summary['audit_div'] += 1
        return
    key = 'audit_vs_prediction_diff'
    if a[0] != 'OK':
        # every audit case is answered (an audit input the law refuses answers NO_LABEL)
        miss(name, 'an answer', a[0], e.get('note'), key); return
    try:
        got = json.loads(a[1])
    except Exception:
        miss(name, 'a parsable answer', a[1][:80], e.get('note'), 'audit_unparsable'); return
    pred = e['predicted_label']
    if pred == 'NO_LABEL':
        if got != {'ok': False, 'reason': 'NO_LABEL'}:
            miss(name, 'exactly {"ok":false,"reason":"NO_LABEL"}', a[1][:80], e.get('note'), key)
        return
    if not isinstance(got, dict) or 'label' not in got:
        miss(name, pred, 'an answer with no label', e.get('note'), key); return
    if pred is None or pred != got['label']:
        miss(name, pred, got['label'], e.get('note'), key); return
    pc = e['predicted_counts'] or {}
    absent = sorted({member_of(k) for k in pc if member_of(k) not in got})
    if absent:
        miss(name, 'members ' + ', '.join(absent), 'absent from the answer', e.get('note'), 'audit_counts_vs_prediction_diff'); return
    gc = counts_of(got)
    bad = {k: (v, gc.get(k, 0)) for k, v in pc.items() if gc.get(k, 0) != v}
    if bad:
        miss(name, 'counts', bad, e.get('note'), 'audit_counts_vs_prediction_diff')

def check_sign(e):
    name = os.path.basename(e['meta'])
    try:
        meta = json.load(open(os.path.join(CORPUS, e['meta'])))
    except Exception as ex:
        miss(name, 'a readable meta file', str(ex), e.get('note'), 'sign_vs_prediction_diff'); return
    args = [meta['privkey'], os.path.join('sign', meta['input'])]
    if 'domain' in meta:
        args.append(meta['domain'])
    args += list(meta.get('extra_args', []))
    a, b = both(['sign'] + args)
    summary['sign'] += 1
    if a != b:
        divergences.append(('sign', name, a, b)); summary['sign_div'] += 1
        return
    pred = e['predicted']
    key = 'sign_vs_prediction_diff'
    if pred is None:
        if a[0] != 'EXIT2' or a[1] != b'':
            miss(name, 'exit 2', a[0], e.get('note'), key)
    elif not isinstance(pred, dict) or not pred:
        miss(name, 'a prediction naming the members', pred, e.get('note'), key)
    elif a[0] != 'OK':
        miss(name, 'an answer', a[0], e.get('note'), key)
    else:
        try:
            got = json.loads(a[1])
        except Exception:
            miss(name, 'a parsable answer', a[1][:80], e.get('note'), 'sign_unparsable'); return
        bad = {k: (v, got.get(k)) for k, v in pred.items() if not isinstance(got, dict) or got.get(k) != v}
        if bad:
            miss(name, 'members', bad, e.get('note'), key)

def check_misuse(e):
    a, b = both(list(e['argv']))
    summary['misuse'] += 1
    shown = ' '.join(e['argv']) or '(none)'
    if a != b:
        divergences.append(('misuse', shown, a, b)); summary['misuse_div'] += 1
    elif e['predicted'] is not None:
        # an argv case is a misuse case: exit 2 and no answer is its only prediction
        miss(shown, 'a null prediction', e['predicted'], e.get('note'), 'misuse_vs_prediction_diff')
    elif a[0] != 'EXIT2' or a[1] != b'':
        miss(shown, 'exit 2', a[0], e.get('note'), 'misuse_vs_prediction_diff')

REQUIRED = {
    'canon': ('path', 'predicted', 'predicted_canon', 'predicted_entry_id', 'predicted_canon_sha256'),
    'audit': ('path', 'predicted_label', 'predicted_counts'),
    'sign': ('path', 'meta', 'predicted'),
    'misuse': ('argv', 'predicted'),
}
RUNNERS = {'canon': check_canon, 'audit': check_audit, 'sign': check_sign, 'misuse': check_misuse}
named = set()
for e in entries:
    cat = e.get('category') if isinstance(e, dict) else None
    if cat not in RUNNERS:
        miss(str(e)[:80], 'an entry of a known category', cat, None, 'manifest_entry_unknown'); continue
    name = e.get('path') or ' '.join(e.get('argv', []))
    named.update(f for f in (e.get('path'), e.get('meta')) if isinstance(f, str))
    if not need(e, REQUIRED[cat], name):
        continue
    for f in [e['path']] + ([e['meta']] if cat == 'sign' else []):
        if f != '(argv)':
            named.add(f)
            if not os.path.isfile(os.path.join(CORPUS, f)):
                miss(f, 'the file the manifest names', 'absent', e.get('note'), cat + '_vs_prediction_diff')
                break
    else:
        RUNNERS[cat](e)
# a file of the corpus that no entry names is a case with no prediction
for f in listed('canon', lambda n: not n.endswith('.note') and not n.endswith('.md')) \
        + listed('audit', lambda n: n.endswith('.json')) + listed('sign', lambda n: n.endswith('.meta.json')):
    if f not in named:
        miss(f, 'a prediction', 'none in the manifest', None, 'unlisted')

if not any(summary[k] for k in ('check', 'canon', 'sign', 'audit', 'misuse')):
    predmiss.append(('(corpus)', 'cases', 0, CORPUS))
print('=== summary'); 
for k in sorted(summary): print(k, summary[k])
print('=== divergences', len(divergences))
groups = collections.defaultdict(list)
for cmd, name, a, b in divergences:
    key = (cmd, a[0], a[1][:120], b[0], b[1][:120])
    groups[key].append(name)
for key, names in sorted(groups.items(), key=lambda kv: -len(kv[1])):
    cmd, sa, oa, sb, ob = key
    print('\n--- %s x%d e.g. %s' % (cmd, len(names), ', '.join(names[:4])))
    print('  cand:', sa, oa)
    print('  py  :', sb, ob)
    for n in names[:2]:
        for e in entries:
            if isinstance(e, dict) and os.path.basename(e.get('meta') or e.get('path') or '') == n:
                print('  note:', {k: v for k, v in e.items() if k in ('note', 'predicted', 'predicted_label', 'category')})
print('=== agreed-but-differs-from-prediction', len(predmiss))
for x in predmiss[:40]: print('  ', x)
json.dump([{'cmd': c, 'file': n, 'cand': [a[0], a[1].decode('utf-8', 'replace')], 'py': [b[0], b[1].decode('utf-8', 'replace')]} for c, n, a, b in divergences],
          open(OUT, 'w'), indent=1)
sys.exit(1 if divergences or predmiss else 0)
