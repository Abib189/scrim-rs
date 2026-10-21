"""gapsheet.py RUN_DIR OUT_PNG TITLE — a contact sheet of every 100 ms capture from 0.5 s before to 1.0 s
after each batch boundary, cropped to the dot row (where `ui.inspect` placed it), each tile labelled
with its time from the batch, the dots' opacity and the row's ink (pixels unlike the canvas)."""
import json, subprocess, sys, tempfile
d, out, title = sys.argv[1], sys.argv[2], sys.argv[3]
L = json.load(open(f'{d}/log.json'))
ink = {r['k']: r['ink'] for r in json.load(open(f'{d}/presence.json'))}
batches = [e['t'] for e in L['events'] if e['what'] == 'batch-done'][1:]
tmp, tiles, rect = tempfile.mkdtemp(), [], None
for c in L['captures']:
    rect = c['indicator']['rect'] if c.get('indicator') else rect
    near = [b for b in batches if -0.5 <= c['t'] - b <= 1.0]
    if not near or rect is None:
        continue
    x0, y0, x1, y1 = [int(v * 2) for v in rect]
    tile = f"{tmp}/{c['k']:04d}.png"
    subprocess.run(['magick', f"{d}/f{c['k']:04d}.png", '-crop', f'{x1 - x0 + 80}x{y1 - y0 + 60}+{x0 - 40}+{y0 - 30}', '+repage', tile], check=True)
    tiles += ['-label', f"{c['t'] - near[0]:+.2f}s  a={c['alpha']}  ink={ink.get(c['k'], 0):.0f}", tile]
subprocess.run(['magick', 'montage', *tiles, '-font', '/System/Library/Fonts/Supplemental/Arial.ttf', '-pointsize', '10', '-tile', '5x',
                '-geometry', '+4+4', '-background', '#f2f2f2', '-title', title, out], check=True)
print('wrote', out, len(tiles) // 3, 'tiles')
