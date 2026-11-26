#!/usr/bin/env python3
"""Arc-level branch witnessing of a criterion program over its corpus (a measure,
never a criterion).
Usage: bcov.py terms|link|dossier <out.json> [corpus-dir]
Records every (prev_line -> line) arc executed inside the criterion files, then
lists every branch point (if/elif, while, for, try-handler) whose arms were not
both taken. Tooling only; the criterion is never modified."""
import ast
import collections
import contextlib
import io
import json
import os
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
mode, out = sys.argv[1], os.path.abspath(sys.argv[2])
LAWS = {
    'terms': ('terms-py', ('ert.py', 'ertlaw.py', 'ertcrypto.py', 'ertcanon.py'), 'terms-corpus', 'ert'),
    'link': ('link-py', ('erl.py', 'erllaw.py'), 'link-corpus', 'erl'),
    'dossier': ('dossier-py', ('erd.py', 'erdlaw.py', 'erdentry.py'), 'dossier-corpus', 'erd'),
}
pydir, names, corpus, exe = LAWS[mode]
FILES = [os.path.join(ROOT, pydir, f) for f in names]
TARGET = set(FILES)
ARCS = collections.defaultdict(set)
PREV = {}


def tracer(frame, event, arg):
    f = frame.f_code.co_filename
    if f not in TARGET:
        return None
    k = id(frame)
    if event == 'call':
        PREV[k] = frame.f_lineno
        ARCS[f].add((0, frame.f_lineno))
    elif event == 'line':
        ARCS[f].add((PREV.get(k, 0), frame.f_lineno))
        PREV[k] = frame.f_lineno
    elif event == 'return':
        ARCS[f].add((PREV.get(k, 0), -1))
        PREV.pop(k, None)
    elif event == 'exception':
        ARCS[f].add((frame.f_lineno, -2))
        PREV[k] = frame.f_lineno
    return tracer


def run_main(main, argv):
    buf = io.TextIOWrapper(io.BytesIO())
    err = io.StringIO()
    with contextlib.redirect_stdout(buf), contextlib.redirect_stderr(err):
        try:
            main(argv)
        except SystemExit:
            pass
        except RecursionError:
            pass


sys.setrecursionlimit(10000)
sys.path.insert(0, os.path.join(ROOT, pydir))
prog = __import__(exe)
C = os.path.abspath(sys.argv[3]) if len(sys.argv) > 3 else os.path.join(ROOT, corpus)
m = json.load(open(os.path.join(C, 'manifest.json')))
cwd = os.getcwd()
os.chdir(C)
sys.settrace(tracer)
cases = 0
for c in m['cases']:
    run_main(prog.main, [exe] + (c['argv'] if 'argv' in c else [c['cmd']] + c['args']))
    cases += 1
sys.settrace(None)
os.chdir(cwd)


def lines_of(nodes):
    s = set()
    for n in nodes:
        for x in ast.walk(n):
            if hasattr(x, 'lineno'):
                s.update(range(x.lineno, (x.end_lineno or x.lineno) + 1))
    return s


misses = []
total = 0
for f in FILES:
    src = open(f, encoding='utf-8').read().splitlines()
    tree = ast.parse('\n'.join(src))
    arcs = ARCS.get(f, set())
    executed = {b for a, b in arcs} | {a for a, b in arcs}

    def add(kind, line, arm, ok):
        global total
        total += 1
        if not ok:
            misses.append({'file': os.path.relpath(f, ROOT), 'line': line, 'kind': kind, 'arm': arm, 'src': src[line - 1].strip()[:110]})
    for node in ast.walk(tree):
        if isinstance(node, (ast.If, ast.While)):
            test = set(range(node.test.lineno, node.test.end_lineno + 1)) | {node.lineno}
            body = lines_of(node.body)
            orelse = lines_of(node.orelse)
            outs = {b for a, b in arcs if a in test and b not in test}
            if not outs and node.lineno not in executed:
                add(type(node).__name__, node.lineno, 'reached', False)
                continue
            add(type(node).__name__, node.lineno, 'true', bool(outs & body))
            if orelse:
                add(type(node).__name__, node.lineno, 'false', bool(outs & orelse))
            else:
                add(type(node).__name__, node.lineno, 'false', bool(outs - body))
        elif isinstance(node, ast.For):
            head = set(range(node.lineno, node.iter.end_lineno + 1))
            body = lines_of(node.body)
            outs = {b for a, b in arcs if a in head and b not in head}
            if not outs and node.lineno not in executed:
                add('For', node.lineno, 'reached', False)
                continue
            add('For', node.lineno, 'enter', bool(outs & body))
            add('For', node.lineno, 'exhaust', bool(outs - body))
        elif isinstance(node, ast.Try):
            for h in node.handlers:
                hl = lines_of([h])
                add('Except', h.lineno, 'caught', h.lineno in executed or bool(hl & executed))
        elif isinstance(node, ast.FunctionDef):
            add('Def', node.lineno, 'called', bool([b for a, b in arcs if a == 0 and b in lines_of([node])]))
json.dump({'mode': mode, 'cases': cases, 'branch_arms': total, 'misses': misses}, open(out, 'w'), indent=1)
print(mode, 'cases', cases, 'arms', total, 'missed', len(misses))
for mm in misses:
    print('  %s:%d %s/%s  %s' % (mm['file'], mm['line'], mm['kind'], mm['arm'], mm['src']))
