"""Which of the Fix AI stage 1 audit verdicts (still-broken: the 300 that apply to Scrim) touch board
code upstream: a board path, or a board identifier in a changed .rs line. Usage:
python3 -I audit_fixes.py <clone with upstream/main> docs/design/upstream-sync/verdicts.tsv. Read-only."""
import re, subprocess, sys, csv
REPO, VERD = sys.argv[1], sys.argv[2]
def git(*a):
    return subprocess.run(["git","-C",REPO,*a],capture_output=True,text=True,stdin=subprocess.DEVNULL,timeout=60,errors="replace").stdout
pathre=re.compile(r"(?i)(^|/)(boards?(/|\.rs|_)|board_|sticky|stamp|jam_open|facilitat|connector_anywhere)")
tok=re.compile(r"(?<![A-Za-z])(BoardItem|BoardShape|BoardTool|FileKind::Board|is_board\w*|boards?::|crate::board\w*|\.board\b|Facilitat\w*|facilitat\w*|End::Node|board_[a-z_]+)")
rows=[l.rstrip("\n").split("\t") for l in open(VERD) if not l.startswith("#")]
hdr=rows[0]; rows=rows[1:]
counts={}
for r in rows:
    h,v=r[0],r[1]; counts[v]=counts.get(v,0)+1
    if v!="still-broken": continue
    full=git("rev-parse",h+"^{commit}").strip()
    if not full: print("MISSING",h); continue
    names=git("show","--name-only","--format=","--no-renames",full).split()
    bp=[n for n in names if pathre.search(n) and not re.search(r"(?i)clipboard|dashboard|keyboard",n)]
    diff=git("show","-U0","--format=","--no-renames",full)
    cur=None; hits={}
    for line in diff.splitlines():
        if line.startswith("diff --git"):
            m=re.match(r"diff --git a/(\S+) b/(\S+)",line); cur=m.group(2) if m else None; continue
        if line.startswith(("+++","---")): continue
        if cur and cur.endswith(".rs") and line[:1] in "+-" and tok.search(line[1:]): hits[cur]=hits.get(cur,0)+1
    if bp or hits:
        subj=git("show","-s","--format=%s",full).strip()
        print(f"{h}\t{subj[:90]}\tpaths={','.join(bp) or '-'}\tlines={sum(hits.values())}\t{','.join(f'{k}:{v}' for k,v in hits.items())[:160]}")
print(counts)
