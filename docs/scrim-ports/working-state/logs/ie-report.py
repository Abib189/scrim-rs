"""ie-report.py RUNS — the numbers for docs/scrim-ports/working-state/README.md from ie-all.sh's runs."""
import json, statistics as st, subprocess, sys, os

R = sys.argv[1]
canvas = (296, 40, 1200, 900)  # points; the captures are 2x


def load(run):
    return json.load(open(f'{R}/{run}/log.json')), json.load(open(f'{R}/{run}/conditions.json'))


def visible(ind):
    if not ind:
        return False
    x0, y0, x1, y1 = ind['rect']
    return canvas[0] <= x0 and x1 <= canvas[2] and canvas[1] <= y0 and y1 <= canvas[3]


for run in ('steps', 'grace'):
    L, c = load(run)
    ev = {(e['what'], e.get('i')): e['t'] for e in L['events']}
    first = ev[('step', 0)]
    end = ev.get(('finish', None), ev[('step-done', 5)] + 4.9)
    caps = L['captures']
    gaps = [b['t'] - a['t'] for a, b in zip(caps, caps[1:])]
    during = [x for x in caps if first + 0.4 <= x['t'] <= end]
    seen = [x for x in during if visible(x['indicator'])]
    print(f"\n## {run}: {len(caps)} captures, every {st.median(gaps) * 1000:.0f} ms (max {max(gaps) * 1000:.0f}); load {c['load']}, swapouts {c['swapouts']}")
    print(f"  from the first step + 0.4 s to {'finish_working' if run == 'steps' else 'the last request + 4.9 s'}: {len(during)} captures; the row drawn and inside the canvas in {len(seen)}")
    for x in during:
        if not visible(x['indicator']):
            print('   MISSING', round(x['t'] - first, 3), x['indicator'])
    ref = ev[('finish', None)] if run == 'steps' else ev[('step-done', 5)]
    for x in caps:
        d = x['t'] - ref
        if (run == 'steps' and -0.3 <= d <= 2.3) or (run == 'grace' and 4.0 <= d <= 6.3):
            print(f"   {d:+.3f} s: {'dots' if x['indicator'] else 'no dots'}  working={x['working']!r} finished={x['finished']!r}")

ON, OFF = f'{R}/sync-on', f'{R}/sync-off'
print('\n## the pixel diff, dots and reveal on vs off, the same moments')
for f in ['s0', 's1', 's2', 's3', 's4', 's5', 's9-finished']:
    masked = []
    for d in (ON, OFF):
        m = f'{d}/{f}-masked.png'
        subprocess.run(['magick', f'{d}/{f}.png', '-fill', 'black', '-draw', f'rectangle {canvas[0] * 2},{canvas[1] * 2} {canvas[2] * 2 - 1},{canvas[3] * 2 - 1}', m], check=True)
        masked.append(m)
    out = subprocess.run(['magick', 'compare', '-metric', 'AE', *masked, 'null:'], capture_output=True, text=True).stderr.split()[0]
    full = subprocess.run(['magick', 'compare', '-metric', 'AE', f'{ON}/{f}.png', f'{OFF}/{f}.png', 'null:'], capture_output=True, text=True).stderr.split()[0]
    print(f'  {f}: outside the canvas {out} px; inside it {full} px')
    for m in masked:
        os.remove(m)
for d in (ON, OFF):
    print('  conditions', d.rsplit('/', 1)[1], json.load(open(f'{d}/conditions.json')))

print('\n## reveal durations (ui.inspect sampled every ~30 ms)')
want = {'fast': (1.14, 1.2), 'normal': (3.01, 4.0), 'slow': (4.84, 6.0)}
for sp in ('fast', 'normal', 'slow'):
    L, c = load(f'speed-{sp}')
    for r, w in zip(L['reveals'], want[sp]):
        print(f"  {sp} {r['layers']} layers: {r['duration_s']} s (expected {w}); load {c['load']}, swapouts {c['swapouts']}")

print('\n## the Fixture (782 layers), a 16-layer batch')
for sp in ('normal', 'slow'):
    for n in (1, 2, 3):
        L, c = load(f'perf-{sp}-{n}')
        s = L['perf']
        rev = [x for x in s if (x['revealing'] or 0) > 0]
        f = [x['frameMs'] for x in s if x['frameMs'] is not None]
        r = [x['renderMs'] for x in s if x['renderMs'] is not None]
        print(f"  {sp} {n}: reveal {rev[-1]['t'] - rev[0]['t']:.2f} s; frame {st.median(f):.1f} / {max(f):.1f} ms; render {st.median(r):.1f} / {max(r):.1f} ms; "
              f"dots in {sum(1 for x in s if x['indicator'])}/{len(s)} samples; degraded {max(x['degraded'] or 0 for x in s)}; load {c['load']}; swapouts {c['swapouts']}")
