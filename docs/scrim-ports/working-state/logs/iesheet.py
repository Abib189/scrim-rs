"""iesheet.py RUN_DIR OUT_PNG TITLE — a contact sheet of a steps/grace run: every 250 ms capture, cropped
around the working indicator's row (where `ui.inspect` reported it, or where it last was), labelled with
its time after the first step, the dots (columns, or none) and the AccessKit text."""
import json, subprocess, sys, os, tempfile

d, out, title = sys.argv[1], sys.argv[2], sys.argv[3]
L = json.load(open(f'{d}/log.json'))
ev = {(e['what'], e.get('i')): e['t'] for e in L['events']}
t0 = ev[('step', 0)]
marks = {ev[('step', i)]: f'step {i + 1}' for i in range(6)}
if ('finish', None) in ev:
    marks[ev[('finish', None)]] = 'finish_working'
if ('last-request', None) in ev:
    marks[ev[('step-done', 5)]] = 'last request'
tmp = tempfile.mkdtemp()
tiles = []
last = None
scale = 2  # the captures are 2x
for c in L['captures']:
    if c['t'] < t0 - 0.3:
        continue
    ind = c.get('indicator')
    # A fixed band across the canvas's top (where the rows hang), so each tile shows exactly what its
    # screenshot shows: `ui.inspect` is read just after it, when a flying camera has moved on.
    tile = f'{tmp}/t{c["k"]:03d}.png'
    subprocess.run(['magick', f'{d}/f{c["k"]:03d}.png', '-crop', f'{904 * scale}x{170 * scale}+{296 * scale}+{90 * scale}', '+repage', '-resize', '540x102', tile], check=True)
    near = [m for t, m in marks.items() if 0 <= c['t'] - t < 0.26]
    dots = f"dots ({ind['columns']} cols)" if ind else 'NO DOTS'
    text = (c.get('working') or c.get('finished') or '—').replace('Codex is working on ', 'working: ')
    tiles += ['-label', f"t={c['t'] - t0:+.2f}s  {dots}{'  ← ' + near[0] if near else ''}\n{text}", tile]
subprocess.run(['magick', 'montage', *tiles, '-font', '/System/Library/Fonts/Supplemental/Arial.ttf', '-pointsize', '11', '-tile', '4x',
                '-geometry', '540x102+6+6', '-background', '#f2f2f2', '-title', title, out], check=True)
print('wrote', out, len(tiles) // 3, 'tiles')
