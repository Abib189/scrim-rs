"""tables.py: the Markdown tables in docs/design/upstream-sync.md, from commits.tsv (scan.py) and
verdicts.tsv (one line per bug, security, performance or dependency commit: what reading Scrim's
code found). Read-only; prints to stdout:

    python3 -I docs/design/upstream-sync/tables.py counts|groups|applicability
"""
import collections, csv, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
NAMES = {'a': 'Bug fixes', 'b': 'Security fixes', 'c': 'Performance', 'd': 'Dependency bumps', 'e': 'New features', 'f': 'Docs, tests, CI and chores'}
VERDICTS = ['still-broken', 'fixed', 'n/a', 'unclear']
LABEL = {'still-broken': 'applies, still broken', 'fixed': 'already fixed in Scrim', 'n/a': 'not applicable', 'unclear': 'unclear'}


def cell(s):
    return (s or '').replace('|', '\\|').replace('\n', ' ').strip()


def load():
    commits = list(csv.DictReader(open(os.path.join(HERE, 'commits.tsv'), encoding='utf-8'), delimiter='\t'))
    verdicts = {}
    path = os.path.join(HERE, 'verdicts.tsv')
    if os.path.exists(path):
        for r in csv.DictReader((l for l in open(path, encoding='utf-8') if not l.startswith('#')), delimiter='\t'):
            verdicts[r['hash']] = r
    return commits, verdicts


def areas(r):
    a = [x.replace('crates/', '').replace('apps/', '') for x in r['areas'].split() if x not in ('ROADMAP.md', 'docs')]
    return ', '.join(a[:4]) + (' …' if len(a) > 4 else '')


def counts(commits, verdicts):
    by = collections.Counter(r['group'] for r in commits)
    print('| Group | Commits |')
    print('|---|---:|')
    for g in 'abcdef':
        print(f'| ({g}) {NAMES[g]} | {by[g]} |')
    print(f'| **Total** (merges left out) | **{len(commits)}** |')
    print()
    print('| Group | ' + ' | '.join(LABEL[v] for v in VERDICTS) + ' |')
    print('|---|' + '---:|' * len(VERDICTS))
    tot = collections.Counter()
    for g in 'abcd':
        c = collections.Counter(verdicts.get(r['hash'], {}).get('verdict', '?') for r in commits if r['group'] == g)
        tot.update(c)
        print(f'| ({g}) {NAMES[g]} | ' + ' | '.join(str(c[v]) for v in VERDICTS) + ' |')
    print('| **Total** | ' + ' | '.join(f'**{tot[v]}**' for v in VERDICTS) + ' |')


def groups(commits, _):
    for g in 'abcdef':
        rows = [r for r in commits if r['group'] == g]
        print(f'#### ({g}) {NAMES[g]}: {len(rows)}\n')
        print('| Commit | Date | Author | Subject | Files | Areas |')
        print('|---|---|---|---|---:|---|')
        for r in rows:
            print(f"| `{r['hash']}` | {r['date']} | {cell(r['author'])} | {cell(r['subject'])} | {r['files']} | {cell(areas(r))} |")
        print()


def applicability(commits, verdicts):
    order = {'b': 0, 'a': 1, 'c': 2, 'd': 3}
    rows = sorted((r for r in commits if r['group'] in 'abcd'), key=lambda r: (order[r['group']], VERDICTS.index(verdicts.get(r['hash'], {}).get('verdict', 'unclear')) if verdicts.get(r['hash'], {}).get('verdict') in VERDICTS else 9))
    print('| Commit | Group | Subject | Verdict | Port | Frozen files | Users hit it | Evidence (Scrim\'s code) |')
    print('|---|---|---|---|---|---|---|---|')
    for r in rows:
        v = verdicts.get(r['hash'], {})
        note = f" ({r['note']})" if r.get('note') else ''
        ev = cell(v.get('evidence', 'not reviewed'))
        if v.get('repro') and v['repro'] != '-':
            ev += f" Reproduction: {cell(v['repro'])}"
        print(f"| `{r['hash']}` | {r['group']} | {cell(r['subject'])}{cell(note)} | {LABEL.get(v.get('verdict'), v.get('verdict', '?'))} | {cell(v.get('effort', '-'))} | {cell(v.get('frozen', '-'))} | {cell(v.get('user', '-'))} | {ev} |")


if __name__ == '__main__':
    c, v = load()
    {'counts': counts, 'groups': groups, 'applicability': applicability}[sys.argv[1]](c, v)
