"""crop.py SHOTS OUT: each window shot capture.py saved (2x), cut to the AI panel (from the header's
left, top and right edges recorded at shot time in {theme}-{width}-states.json, to the window's
bottom) and scaled to 1x, as OUT/{theme}-{state}.png. The window can grow between runs, so each
shot is cut by its own size."""
import json, os, subprocess, sys

shots, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)


def run(*args):
    return subprocess.run(list(args), capture_output=True, text=True, timeout=60, stdin=subprocess.DEVNULL).stdout


def size(p):
    o = run('sips', '-g', 'pixelWidth', '-g', 'pixelHeight', p).split()
    return int(o[o.index('pixelWidth:') + 1]), int(o[o.index('pixelHeight:') + 1])


for name in sorted(os.listdir(shots)):
    if not name.endswith('-states.json'):
        continue
    theme = name.split('-')[0]
    for state, v in json.load(open(os.path.join(shots, name))).items():
        src, dst = os.path.join(shots, f'{theme}-{state}.png'), os.path.join(out, f'{theme}-{state}.png')
        _, h = size(src)
        x0, y0, x1 = (int(v['header'][i]) * 2 for i in (0, 1, 2))
        run('sips', '-c', str(h - y0), str(x1 - x0), '--cropOffset', str(y0), str(x0), src, '--out', dst)
        run('sips', '-z', str((h - y0) // 2), str((x1 - x0) // 2), dst)
        print(dst)
