"""scan.py: Fix AI's upstream audit. Lists every upstream FigCraft commit after the fork point,
groups it, and measures how much of what it changed is still in this tree.

Read-only. It needs a throwaway clone with `upstream` fetched (never this working copy's remotes):

    git clone --no-checkout <this repo> ~/.cache/figcraft-upstream-audit
    cd ~/.cache/figcraft-upstream-audit && git remote add upstream https://github.com/storytold/figcraft.git
    git fetch upstream
    python3 -I docs/design/upstream-sync/scan.py ~/.cache/figcraft-upstream-audit .

Output in docs/design/upstream-sync/: commits.tsv (every commit: hash, date, author, group, subject,
files touched, pre/post scores) and a count per group on stdout.

Groups: a bug fix, b security, c performance, d dependency, e feature, f docs/tests/CI/chores.
A keyword rule groups each subject; overrides.tsv (hash prefix, group, note) corrects it by hand.

Scores (code files only, tests left out):
  pre  = share of the lines the commit removed (and didn't add back) still found in this tree, in
         the same crate: a high pre means the code it fixed is still here as it was;
  post = share of the lines it added found here: a high post means the fix (or the same code) is
         already here.
They point at where to read; the applicability table is decided by reading the code.
"""
import collections, os, re, subprocess, sys

FORK = 'a0d1162976fac1e4a8b8fd49a3948e9d6bad6f40'
HEAD = 'upstream/main'
HERE = os.path.dirname(os.path.abspath(__file__))
CODE = ('.rs', '.toml', '.sh', '.yml', '.yaml', '.js', '.py', '.plist', '.desktop', '.ps1', '.nsi', '.wxs')


def git(repo, *args):
    return subprocess.run(['git', '-C', repo, *args], capture_output=True, text=True, check=True,
                          stdin=subprocess.DEVNULL, errors='replace').stdout


def is_test(path):
    base = os.path.basename(path)
    return '/tests/' in path or base.endswith('_tests.rs') or base in ('tests.rs', 'test.rs') or '/tests_' in path


def scope(path):
    parts = path.split('/')
    return '/'.join(parts[:2]) if parts[0] in ('crates', 'apps') and len(parts) > 2 else ''


def norm(line):
    s = ' '.join(line.strip().split())
    if len(s) < 20 or not re.search(r'[A-Za-z0-9]', s):
        return None
    if s.startswith(('//', '#', '*', '/*')):
        return None
    return s


def add_lines(idx, path, text):
    sc = scope(path)
    for line in text.splitlines():
        n = norm(line)
        if n:
            idx[n].add(sc)


def index_ours(ours):
    idx = collections.defaultdict(set)
    for path in git(ours, 'ls-files').splitlines():
        if not path.endswith(CODE) or is_test(path):
            continue
        try:
            add_lines(idx, path, open(os.path.join(ours, path), encoding='utf-8', errors='replace').read())
        except OSError:
            continue
    return idx


def index_rev(repo, rev):
    """The same index for a commit of the clone (the fork point)."""
    idx = collections.defaultdict(set)
    paths = [p for p in git(repo, 'ls-tree', '-r', '--name-only', rev).splitlines() if p.endswith(CODE) and not is_test(p)]
    blobs = subprocess.run(['git', '-C', repo, 'cat-file', '--batch'], input=('\n'.join(f'{rev}:{p}' for p in paths) + '\n').encode(),
                           capture_output=True, check=True).stdout
    pos = 0
    for path in paths:
        end = blobs.index(b'\n', pos)
        size = int(blobs[pos:end].split()[2])
        add_lines(idx, path, blobs[end + 1:end + 1 + size].decode('utf-8', 'replace'))
        pos = end + 1 + size + 1
    return idx


def ratio(s):
    if not s:
        return None
    a, b = s.split('/')
    return int(a) / int(b)


def hint(fork, pre, post):
    """What the scores suggest; the applicability table says what reading the code found."""
    f, o, q = ratio(fork), ratio(pre), ratio(post)
    if f is None:
        return 'additive: read'
    if f < 0.3:
        return 'post-fork code'
    if q is not None and q >= 0.6:
        return 'fix lines here: read'
    if o >= 0.6:
        return 'pre-image here'
    if o < 0.3:
        return 'changed here: read'
    return 'mixed: read'


RULES = [
    ('d', r'^(chore|ci)\(deps\)|dependabot|dependency refresh|^build: (one craft-libs|pin craft)|^craft-libs:|egui 0\.37|core-text 22|use chrono|craft-font-catalog|krilla|lockfile|cargo\.lock'),
    ('b', r'security|slowloris|symlink|0700|owner-only|noopener|https on|secret|operator key|join codes? found by hash|automation roots|confined|refuse non-local|deny library|refuse text\.loadfont|bidi control|per-address|per ipv6|pre-hello|hello (size|deadline)|close-drain|close drain|regular file within a cap|window\.figcraft is opt-in|reach no server|contacts no team library|file names, not paths|validate image member|hostile|compression ratio|nesting depth|decoded-value budget|unpack budget|byte budget|budgets? four values|bound attached mcp|clamp control-channel|cap the input|cap the people|replay log is bounded|shut kicked|storage caps|temp files at server|u64 overflow|fail closed|refuse oversized|cap files read|reads at most max_font|cap layout\.settrack|cap the issue list|damaged table offsets|progressive jpeg|cap the clipboard png|images-past-budget|zip export stops|kiwi fields|recursion limit|loops in files are repaired|would contain themselves|pinned? the signing client|pin github actions'),
    ('c', r'^perf|speed up|faster|linear( time)?\b|o\(spans\)|quadratic|binary search|occupancy bitmap|once per|smaller copy|stop going over|kept for frames|only what sits|shifts what was worked|set lookups|without .*copies|copied once per blob|one undo snapshot|stash only closed tabs|duplicate buffers|no disk i/o|hit-tests once|checks only the layers it changed|by the layers it visits|reads only the fallback fonts|refines a zoom only once|a band at a time|keep glass shapes|instance expansion'),
    ('f', r'^(docs?|test|tests|ci|chore|refactor|contributors?|readme|i18n|packaging|freebsd package\.sh|release \d|web: (raise|build)|macos signing|revert|merge fix|server tests)\b|^#\d+: (drop|leave|move|split|explicit|import|label|test|the .* test|one roadmap|revision|restore the blank|gaps and roadmap|share)|^m\d+(\.\d+)?: (re-run ci|credit|use .* contributor|sync .* contributor|retain github|document|doc comments|test|a test|normalize contribution|keep the shared roadmap|leave roadmap|correct the migration|correct egui|cover|clarify german|bosnian|record|move|parity)|^batch r2-1 review: (status lines|vector_up in smaller)|sonar|rustfmt|ci gate fix|roadmap|catalog|translat|^m10:|i18n|^updates: gate|unused (tr )?import|^add .* (to )?contributors|^change order of characters|^update supported languages|^record korean|^add bengali ui|^update bengali|clippy|lint policy|^m-egui037|^e21:|^m14:|^m0: correct|^m0: '),
    ('a', r'\bfix|\bkeep|\bkeeps\b|preserv|refus|reject|\bstop|no longer|don\'t|doesn\'t|never|avoid|restor|correct|honou?r|clamp|prevent|surviv|missing|instead of|crash|panic|stays|\bstill\b|can\'t|isn\'t|says why|survive|wrong|stale|is refused|guard|repair|drop(s|ped)? |ends the|goes when|only when|counts once|ask before|follow|match|escape|neutralise|strip|validate|require|enforce|resolve|exactly|full precision|finite|overflow|scale down|skip|grow back|right-click on|dismiss|settles|waits?|with a reason|unique|duplicate svg|double alpha|disjoint|treat|accept|apply|cap '),
]


def classify(subject):
    s = subject.lower()
    for group, pattern in RULES:
        if re.search(pattern, s):
            return group
    return 'e'


def overrides():
    out = {}
    path = os.path.join(HERE, 'overrides.tsv')
    if os.path.exists(path):
        for line in open(path, encoding='utf-8'):
            if line.strip() and not line.startswith('#'):
                h, g, *note = line.rstrip('\n').split('\t')
                out[h.strip()] = (g.strip(), (note[0] if note else '').strip())
    return out


def changed_lines(repo, sha):
    removed, added, per_scope = collections.Counter(), collections.Counter(), {}
    path = None
    for line in git(repo, 'show', '--format=', '-U0', '--no-renames', sha).splitlines():
        if line.startswith('+++ '):
            path = line[6:] if line.startswith('+++ b/') else None
        elif line.startswith('--- '):
            if line != '--- /dev/null':
                path = line[6:]
        elif path and path.endswith(CODE) and not is_test(path):
            if line.startswith('-') and not line.startswith('---'):
                n = norm(line[1:])
                if n:
                    removed[n] += 1
                    per_scope[n] = scope(path)
            elif line.startswith('+') and not line.startswith('+++'):
                n = norm(line[1:])
                if n:
                    added[n] += 1
                    per_scope.setdefault(n, scope(path))
    pre = [l for l in removed if l not in added]
    post = [l for l in added if l not in removed]
    return pre, post, per_scope


def score(lines, sc, idx):
    if not lines:
        return ''
    hit = sum(1 for l in lines if l in idx and (not sc[l] or sc[l] in idx[l]))
    return f'{hit}/{len(lines)}'


def main():
    clone, ours = sys.argv[1], sys.argv[2]
    idx = index_ours(ours)
    fork_idx = index_rev(clone, FORK)
    over = overrides()
    log = git(clone, 'log', '--no-merges', '--format=%H%x1f%ad%x1f%an%x1f%s', '--date=short', f'{FORK}..{HEAD}')
    rows, counts = [], collections.Counter()
    for entry in log.splitlines():
        sha, date, author, subject = entry.split('\x1f')
        files = [f for f in git(clone, 'show', '--format=', '--name-only', '--no-renames', sha).splitlines() if f]
        group, note = classify(subject), ''
        for prefix, (g, n) in over.items():
            if sha.startswith(prefix):
                group, note = g, n
        fork = pre = post = hnt = ''
        if group in 'abcd':
            p, q, sc = changed_lines(clone, sha)
            fork, pre, post = score(p, sc, fork_idx), score(p, sc, idx), score(q, sc, idx)
            hnt = hint(fork, pre, post)
        counts[group] += 1
        areas = sorted({'/'.join(f.split('/')[:2]) if f.startswith(('crates/', 'apps/')) else f.split('/')[0] for f in files})
        rows.append((sha[:8], date, author, group, subject, str(len(files)), ' '.join(areas), fork, pre, post, hnt, note, ' '.join(files)))
    with open(os.path.join(HERE, 'commits.tsv'), 'w', encoding='utf-8') as f:
        f.write('hash\tdate\tauthor\tgroup\tsubject\tfiles\tareas\tfork\tpre\tpost\thint\tnote\tpaths\n')
        for r in rows:
            f.write('\t'.join(c.replace('\t', ' ') for c in r) + '\n')
    names = {'a': 'bug fixes', 'b': 'security', 'c': 'performance', 'd': 'dependencies', 'e': 'features', 'f': 'docs/tests/CI/chores'}
    for g in 'abcdef':
        print(f'{g} {names[g]}: {counts[g]}')
    print('total', sum(counts.values()))


if __name__ == '__main__':
    main()
