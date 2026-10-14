"""Golden fixture for the Rust port of o4fix: synthetic 1 kHz attitude with false bursts,
a real fast manoeuvre and noise, run through o4fix.correction.correct()."""
import numpy as np, struct, sys
from o4fix import quat, correction

rng = np.random.default_rng(40609)
hz = 1000.0
n = 3000
t = np.arange(n - 1) / hz
rates = np.stack([
    25 * np.sin(2 * np.pi * 0.7 * t) + 8 * np.sin(2 * np.pi * 3.1 * t),
    -18 * np.sin(2 * np.pi * 0.4 * t + 0.5) + 6 * np.sin(2 * np.pi * 5.3 * t),
    12 * np.cos(2 * np.pi * 0.9 * t),
], axis=1)
rates += rng.normal(0, 2.0, rates.shape)                      # sensor noise
# real fast roll: 3-axis, smooth, 200 ms, ~300 deg/s peak
k = np.arange(200); bump = np.sin(np.pi * k / 200) ** 2
rates[1800:2000, 0] += 300 * bump; rates[1800:2000, 1] += 120 * bump; rates[1800:2000, 2] -= 90 * bump
# false single-axis bursts (gyrogate signature): narrow, one axis
def burst(i, axis, peak, width):
    kk = np.arange(width); rates[i:i + width, axis] += peak * np.sin(np.pi * (kk + 0.5) / width)
burst(500, 2, 150, 10)          # ~0.95 deg
burst(1100, 1, -110, 14)        # ~-0.98 deg
burst(1180, 1, 95, 8)           # close to previous -> merge
burst(2400, 0, 200, 18)         # ~2.3 deg
q0 = np.array([0.9, 0.1, -0.3, 0.2]); q0 /= np.linalg.norm(q0)
quats = quat.integrate(q0, rates, hz)
res = correction.correct(quats, hz)
for b in res.bursts: print(b, file=sys.stderr)
out = sys.argv[1]
with open(out, "wb") as f:
    f.write(struct.pack("<II", n, len(res.bursts)))
    for b in res.bursts: f.write(struct.pack("<iiid", b.start_sample, b.end_sample, b.axis, b.area_deg))
    f.write(quats.astype("<f8").tobytes())
    f.write(res.quats.astype("<f8").tobytes())
