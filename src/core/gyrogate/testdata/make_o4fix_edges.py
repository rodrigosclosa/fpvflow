"""Second golden fixture for the Rust port of o4fix: edge cases run through o4fix.correction.correct().

Layout (little-endian): u32 ncases, then per case the same layout as o4fix_synthetic.bin:
u32 n, u32 nbursts, nbursts x (i32 start, i32 end, i32 axis, f64 area), n x 4 f64 input quats,
n x 4 f64 o4fix output quats. 1 kHz, default settings."""
import numpy as np, struct, sys
from o4fix import quat, correction

hz = 1000.0

def make_case(seed, n, setup):
    rng = np.random.default_rng(seed)
    t = np.arange(n - 1) / hz
    rates = np.stack([
        18 * np.sin(2 * np.pi * 0.5 * t + 1.0) + 5 * np.sin(2 * np.pi * 4.3 * t),
        -22 * np.cos(2 * np.pi * 0.8 * t) + 4 * np.sin(2 * np.pi * 6.1 * t),
        9 * np.sin(2 * np.pi * 1.3 * t + 0.3),
    ], axis=1)
    rates += rng.normal(0, 1.5, rates.shape)
    setup(rates)
    q0 = rng.normal(size=4); q0 /= np.linalg.norm(q0)
    quats = quat.integrate(q0, rates, hz)
    res = correction.correct(quats, hz)
    for b in res.bursts: print(seed, b, file=sys.stderr)
    return n, quats, res

def burst(rates, i, axis, peak, width):
    k = np.arange(width); rates[i:i + width, axis] += peak * np.sin(np.pi * (k + 0.5) / width)

def long_case(rates):
    n = len(rates)
    burst(rates, 0, 1, 160, 9)              # at the very start: no clean rates before it
    burst(rates, n - 10, 2, -140, 10)       # at the very end: no clean rates after it
    burst(rates, 600, 0, 120, 40)           # 40 ms single-axis bump: wider than 25 ms, kept
    # real three-axis manoeuvre, 150 ms
    k = np.arange(150); bump = np.sin(np.pi * k / 150) ** 2
    rates[1000:1150, 0] += 250 * bump; rates[1000:1150, 1] -= 200 * bump; rates[1000:1150, 2] += 150 * bump
    # two impulses ~100 ms apart with jittery false content between them -> noisy merge
    burst(rates, 1400, 0, 130, 10)
    rates[1410:1500, 0] += 10 * (-1.0) ** np.arange(90)
    burst(rates, 1500, 0, 120, 10)

def short_case(rates):
    burst(rates, 90, 0, 180, 8)

cases = [make_case(4060901, 1800, long_case), make_case(4060902, 200, short_case)]
with open(sys.argv[1], "wb") as f:
    f.write(struct.pack("<I", len(cases)))
    for n, quats, res in cases:
        f.write(struct.pack("<II", n, len(res.bursts)))
        for b in res.bursts: f.write(struct.pack("<iiid", b.start_sample, b.end_sample, b.axis, b.area_deg))
        f.write(quats.astype("<f8").tobytes())
        f.write(res.quats.astype("<f8").tobytes())
