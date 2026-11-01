"""Find upstream commits that touch boards: board paths, board words in the subject, or board
identifiers in a diff line. Usage: python3 -I find_commits.py <clone with upstream/main> <out.json>.
Read-only (git show / rev-list). The hand-checked result is ../commits.tsv."""
import re, subprocess, sys, json
REPO, OUT = sys.argv[1], sys.argv[2]
def git(*a):
    return subprocess.run(["git","-C",REPO,*a],capture_output=True,text=True,stdin=subprocess.DEVNULL,timeout=60,errors="replace").stdout
code = re.compile(r"BoardItem|BoardShape|BoardTool|FileKind::Board|is_board|board_atom|\bboards?::|crate::board|\.board\b|\bboard:|board_[a-z_]+\(|Facilitat|facilitat|BoardFacil|\bStamp\b|Stamp::|\bSticky\b|Sticky\s*\{|board\.[a-z]+|\"board\b|board-[a-z]|\bboard_bar\b|board_picker|board_connect|board_route|board_grid|board_table|board_templates|ConnectorStyle|connector\.[a-z]+|End::Node|stickies|is_jam|\.jam\b|FigJam")
subj = re.compile(r"(?i)\bboards?\b|stick(y|ies)|connector|figjam|\.jam\b|\bstamps?\b|\bvot(e|es|ing)\b|whiteboard|marker|highlighter|facilitat|M12")
pathre = re.compile(r"(?i)(board|sticky|stamp|jam_open|facilitat|connector_anywhere)")
commits = git("rev-list","--no-merges","--reverse","a0d1162..upstream/main").split()
rows=[]
for c in commits:
    h,d,a,s = git("show","-s","--format=%h%x09%ad%x09%an%x09%s","--date=short",c).rstrip("\n").split("\t",3)
    diff = git("show","-U0","--format=","--no-renames",c)
    cur=None; hitfiles={}; n=0
    for line in diff.splitlines():
        if line.startswith("diff --git"):
            m=re.match(r"diff --git a/(\S+) b/(\S+)",line); cur=m.group(2) if m else None; continue
        if line.startswith(("+++","---")): continue
        if line[:1] in "+-" and cur and code.search(line[1:]):
            n+=1; hitfiles[cur]=hitfiles.get(cur,0)+1
    ns = git("show","--numstat","--format=","--no-renames",c)
    files=[]; add=dele=0
    for l in ns.splitlines():
        p=l.split("\t")
        if len(p)==3:
            files.append((p[2], p[0], p[1]))
            if p[0]!="-": add+=int(p[0]); dele+=int(p[1])
    bpaths=[f for f,_,_ in files if pathre.search(f) and not re.search(r"(?i)clipboard|dashboard|keyboard",f)]
    rows.append(dict(hash=h,date=d,author=a,subject=s,codehits=n,hitfiles=hitfiles,subj=bool(subj.search(s)),bpaths=bpaths,files=files,added=add,deleted=dele))
json.dump(rows,open(OUT,"w"))
