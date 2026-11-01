"""Files and lines added per area for the board commits listed inside (hand-checked from
find_commits.py). Usage: python3 -I commit_stats.py <clone with upstream/main> <out.tsv>. Read-only."""
import subprocess, sys, re
REPO, OUT = sys.argv[1], sys.argv[2]
def git(*a):
    return subprocess.run(["git","-C",REPO,*a],capture_output=True,text=True,stdin=subprocess.DEVNULL,timeout=60,errors="replace").stdout
# kind: F feature, X fix, I import (FigJam/.fig), D docs, R refactor/move (board code moved, no behaviour)
L = """8167f13a F
26291aff F
4fad29a6 F
d3782fdc F
948358ee D
4e650a1c I
01bc4741 I
d4e6ad5d F
2d507d95 F
59af2bc4 X
9d86403f X
b00cccb1 X
8ee8ba6c I
a5f88f14 D
34fce9bf F
9099f1d5 I
4f5a25cc F
7e9d6ce8 F
838d3687 I
e97961ba F
9f4d1ce7 F
4bcfd524 F
b462a483 F
59c76245 X
12e9b409 F
7f18940c X
6cc466a8 D
7081bbdd D
5f7aea91 R
236f3185 R
5d1b826f R
3e23a0f8 R
49ca56c3 R
688eeccc R
60c671da R
5e166839 R"""
def area(p):
    if re.search(r"(_tests\.rs|/tests?\.rs|/tests/)", p): return "tests"
    if p.startswith("crates/ui-egui/locales/"): return "locales"
    if p.startswith(("docs/","ROADMAP","README")) or p.endswith(".md"): return "docs"
    m=re.match(r"crates/([^/]+)/",p)
    if m: return m.group(1)
    m=re.match(r"apps/([^/]+)/",p)
    if m: return "app:"+m.group(1)
    return "other"
AREAS=["doc","collab","engine","ui-egui","format","render","mcp","tests","locales","docs","other"]
with open(OUT,"w") as f:
    f.write("hash\tdate\tkind\tsubject\tfiles\tadded\tdeleted\t"+"\t".join("+"+a for a in AREAS)+"\tpaths\n")
    for line in L.split("\n"):
        h,k=line.split()
        d,s=git("show","-s","--format=%ad%x09%s","--date=short",h).strip().split("\t",1)
        ns=git("show","--numstat","--format=","-M",h)
        add=dele=0; per={a:0 for a in AREAS}; paths=[]
        for l in ns.splitlines():
            p=l.split("\t")
            if len(p)!=3: continue
            path=p[2]
            if "=>" in path:
                path=re.sub(r"\{[^}]*=> ([^}]*)\}",r"\1",path); path=re.sub(r".* => ","",path).replace("//","/")
            paths.append(path)
            if p[0]=="-": continue
            a,b=int(p[0]),int(p[1]); add+=a; dele+=b
            ar=area(path); per[ar if ar in per else "other"]+=a
        f.write(f"{h}\t{d}\t{k}\t{s}\t{len(paths)}\t{add}\t{dele}\t"+"\t".join(str(per[a]) for a in AREAS)+"\t"+" ".join(paths)+"\n")
