

# ---- spots-2: the smash charge ---------------------------------------------------------------------------------------------

# Where Marth's forward smash lands, by spacing (walk frames toward the CPU after `reset-center`): only the tip reaches at the first, only the
# blades at the second. This is the rig's independent oracle for a charged hit: the geometry does not change with the charge, so the label must be
# the one of the spacing whatever the damage. The uncharged swings of the same call show the spacing still holds (their labels are checked the same way).
SPACING = {8: 'Sweet', 28: 'Sour'}


def _charge_swings(ctx, holds, walks, rounds):
    """Marth's forward smash with the C-stick held `hold` extra frames (0, 30, 66: none, about half, full charge) at each spacing, `rounds` times.
    Every swing keeps the diag build's `[SpotsProbe]` (raw AttackModule floats per change), `[Spots] stop` and P1's `[FrameStrip]` lines."""
    swings = []
    for rnd in range(rounds):
        for hold in holds:
            for walk in walks:
                attack = 'sp-csmash' if hold == 0 else 'sp-csmash-h%d' % hold
                ctx.play('reset-center')
                time.sleep(2.0)
                if walk:
                    ctx.play(_walk(walk))
                    time.sleep(0.4)
                seq = hs.diag_mark(ctx)
                ctx.play(attack)
                time.sleep(4.0 + hold / 30.0)
                lines = hs.diag_since(ctx, seq)
                keep = [t for _, _, t in lines if '[SpotsProbe]' in t or '[Spots]' in t or '[FrameStrip] p1' in t]
                swings.append({'round': rnd, 'hold': hold, 'walk': walk, 'lines': keep, 'hits': _hits(lines),
                               'probe_lines': sum('[SpotsProbe]' in t for t in keep)})
    return swings


def sp_charge_probe(ctx, holds=(0, 30, 66), walks=(8, 28), rounds=2):
    """The probe row: the swings and every kept line, for charge_fit.py; it judges nothing but needs the probe lines."""
    swings = _charge_swings(ctx, holds, walks, rounds)
    out = {'swings': swings, 'holds': list(holds), 'walks': list(walks)}
    steps.save(ctx, 'charge-probe', out)
    if not any(s['probe_lines'] for s in swings):
        raise RuntimeError('no [SpotsProbe] line in any swing (is this the probe build?)')
    return {'swings': len(swings), 'with_hits': sum(1 for s in swings if any(h['gained'] > 0 for h in s['hits']))}


def sp_charge(ctx, holds=(0, 30, 66), walks=(8, 28), rounds=2):
    """The final row: every labelled hit of every swing agrees with the spacing's class (the geometry oracle) and with the damage oracle
    (`_classes`, the SPEC's rules with the logged multiplier); a hit with no label is one the damage oracle also cannot tell; and every spacing is
    labelled at least once at every charge."""
    swings = _charge_swings(ctx, holds, walks, rounds)
    bad, seen = [], {}
    for s in swings:
        want = SPACING.get(s['walk'])
        for hit in s['hits']:
            if hit['gained'] <= 0:
                continue
            why = _verdict(hit)
            label = hit['label']
            classes = sorted(c or 'none' for c in _classes(hit['gained'], hit['powers'], mul=hit['mul']))
            if why:
                bad.append((s['hold'], s['walk'], why))
            elif want and label and label != want:
                bad.append((s['hold'], s['walk'], 'label %s but the spacing %d reaches only the %s' % (label, s['walk'], want)))
            elif want and label is None and len(classes) == 1:
                bad.append((s['hold'], s['walk'], 'no label although the damage tells the class %s' % classes))
            if label:
                seen[(s['hold'], s['walk'])] = label
    result = {'swings': swings, 'bad': bad, 'labelled': {'%d@%d' % k: v for k, v in sorted(seen.items())}}
    steps.save(ctx, 'charge', result)
    missing = [(h, w) for h in holds for w in walks if (h, w) not in seen]
    if bad:
        raise RuntimeError('labels the oracles disagree with: %s' % bad)
    if missing:
        raise RuntimeError('no label at (hold, walk) %s' % missing)
    return {'swings': len(swings), 'labelled': result['labelled']}


def sp_knee(ctx, leads=(4, 5, 6), walks=(20, 24, 28, 32, 36, 44)):
    """spots-2: Captain Falcon's forward air with the C-stick 4, 5 and 6 frames after the short hop's X (`sp-fair-e<lead>`), at each spacing: the knee
    is low and early, where the x1 sweeps of the other rows (stick 15 frames after the X, at the top of the hop) found only its late hits. Every label
    is checked by the damage oracle; the row passes when a Sweet and a Sour label each landed (the knee's early frames and the middle frames)."""
    sp_setup(ctx, FALCON, MARIO, frame_data=False, labels=True)
    classes, bad, landed = set(), [], []
    for lead in leads:
        for walk in walks:
            for hit in _swing(ctx, 'sp-fair-e%d' % lead, walk):
                why = _verdict(hit)
                if why:
                    bad.append((lead, walk, why))
                if hit['label']:
                    classes.add(hit['label'])
                if hit['gained'] > 0:
                    landed.append({'lead': lead, 'walk': walk, 'gained': hit['gained'], 'powers': hit['powers'][:4], 'label': hit['label'],
                                   'ratios': hit['ratios'][:4]})
    steps.save(ctx, 'knee', {'landed': landed, 'classes': sorted(classes), 'bad': bad})
    if bad:
        raise RuntimeError('labels the oracle disagrees with: %s' % bad)
    missing = sorted({'Sweet', 'Sour'} - classes)
    if missing:
        raise RuntimeError('the knee\'s %s hit did not land or was not labelled (classes seen %s; %d hits)' % (missing, sorted(classes), len(landed)))
    return {'classes': sorted(classes), 'hits': len(landed)}


def sp_knee_slow(ctx, walks=(88, 96, 104, 112, 144)):
    """spots-2: the first boot's knee hits came from the 1/4-speed takes (`sp_gifs`: Game speed 1/4, `sp-fair` whose macro frames are displayed frames,
    the walk four times the x1 spacing). The x1 rows, the early forward airs included, found only the late hits. This sweeps `sp-fair` at 1/4 speed at the
    spacings around the GIF takes' (28 x 4 = 112) and checks every label with the damage oracle; it passes when a Sweet and a Sour label land."""
    classes, bad, landed = set(), [], []
    _speed_hold(ctx, ['left', 'left', 'left'])
    try:
        for walk in walks:
            ctx.play('reset-center')
            time.sleep(4.0)
            ctx.play(_walk(walk))
            time.sleep(1.0)
            seq = hs.diag_mark(ctx)
            ctx.play('sp-fair')
            time.sleep(9.0)
            for hit in _hits(hs.diag_since(ctx, seq)):
                why = _verdict(hit)
                hit['walk'] = walk
                if why:
                    bad.append((walk, why))
                if hit['label']:
                    classes.add(hit['label'])
                if hit['gained'] > 0:
                    landed.append({'walk': walk, 'gained': hit['gained'], 'powers': hit['powers'][:4], 'label': hit['label'], 'ratios': hit['ratios'][:4]})
    finally:
        _speed_hold(ctx, ['right', 'right', 'right'])
    steps.save(ctx, 'knee-slow', {'landed': landed, 'classes': sorted(classes), 'bad': bad})
    if bad:
        raise RuntimeError('labels the oracle disagrees with: %s' % bad)
    missing = sorted({'Sweet', 'Sour'} - classes)
    if missing:
        raise RuntimeError('the knee\'s %s hit did not land or was not labelled at 1/4 speed (classes seen %s; %d hits)' % (missing, sorted(classes), len(landed)))
    return {'classes': sorted(classes), 'hits': len(landed)}
