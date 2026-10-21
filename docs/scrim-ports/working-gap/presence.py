"""presence.py RUN_DIR — per capture, the dots' ink: the dot row's rectangle (from ui.inspect, in
points; the captures are 2x), counting only the dots' orange (red over 180, green under 170, blue
under 140, strong saturation) so a title or the canvas there never counts. Prints the frames with no ink
between the first batch and finish_working (or the last request + grace), and the longest gap."""
import json, sys, subprocess
d = sys.argv[1]
L = json.load(open(f'{d}/log.json'))
ev = {(e['what'], e.get('i')): e['t'] for e in L['events']}
t0 = ev[('batch', 0)] + 0.3
end = ev.get(('finish', None)) or (ev[('last-request', None)] + L['grace'])
rect = None
rows = []
for c in L['captures']:
    if c.get('indicator'):
        rect = c['indicator']['rect']
    if rect is None or not c['ok']:
        continue
    x0, y0, x1, y1 = [int(round(v * 2)) for v in rect]
    # Count pixels in the row's box that aren't the canvas colour (sampled just left of the box).
    out = subprocess.run(['magick', f"{d}/f{c['k']:04d}.png", '-crop', f'{x1 - x0}x{y1 - y0}+{x0}+{y0}', '+repage', '-fx',
                          '(r>0.70 && g<0.67 && b<0.55 && r-b>0.25) ? 1 : 0', '-format', '%[fx:mean*w*h]', 'info:'], capture_output=True, text=True)
    ink = float(out.stdout or 0)
    rows.append({'k': c['k'], 't': c['t'], 'ink': ink, 'alpha': c['alpha']})
json.dump(rows, open(f'{d}/presence.json', 'w'))
window = [r for r in rows if t0 <= r['t'] <= end]
on = [r for r in window if r['ink'] > 20]
gap, longest, last = 0, 0.0, None
for r in window:
    if r['ink'] > 20:
        if last is not None:
            longest = max(longest, r['t'] - last)
        last = r['t']
print(f"{d.split('/')[-1]}: dots in {len(on)} of {len(window)} frames from the first batch to the end; longest time between frames with dots {longest * 1000:.0f} ms; ink min {min(r['ink'] for r in window):.0f} median {sorted(r['ink'] for r in window)[len(window)//2]:.0f}")
for r in window:
    if r['ink'] <= 20:
        print('   no dots at', round(r['t'] - ev[('batch', 0)], 3), r)
