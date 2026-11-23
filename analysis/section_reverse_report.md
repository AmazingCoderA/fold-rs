# Section renderer and DFM input — reverse-engineering pass (v0.10.5)

Primary sources: `analysis/callgraph_report.txt`, `analysis/dfm_values_out.txt`,
`analysis/memory/unpacked_image.bin` (base `0x400000`). The screenshot is used
only as an optional scale check, never as a source of coordinates.

## 1. Buffer chain

`coora @ 0x468934`:

* `0x4689BD` calls `0x466980` with a local 30005-dword buffer;
* `0x4689C4–0x4689CE` copies 30005 dwords to global `0x46CD68`;
* `0x4689D2–0x4689DD` calls renderer `0x465D68(Image2, 0x46CD68, 0)`.

`0x466980` does **not** write the final buffer directly. It fills a small
source array and calls `0x465BD4 @ 0x4673CF`; that routine writes the header
and all samples.

Final buffer layout:

| Offset | Value |
|---|---|
| `+0` | `1` |
| `+4` | `0x12C = 300` samples per trace |
| `+8` / `+0xC` | `t_min` / `t_max` (`f32`) |
| `+0x10` | `n_traces` |
| `+0x14` | samples, stride `0x4B0` = 300 dwords |

Source array copied by `0x465BD4` is 414 dwords = two 207-dword records.
Within a record: `+6` is the integer wavelet denominator (20),
`+7..+107` are `f32 t_i`, `+107..+207` are `f32 amplitude` (`-1.0`).
The inner loop at `0x465C8A` always has `edi = 1`, so `fild [esi]` at
`0x465CB0` reads `src[6] = 20` for **every** trace: `w = 1/20 = 0.05 s`.
There is no overlap with `t_i` and no “tiny width for later traces” bug.

## 2. `0x465BD4` — wavelet synthesis

`0x465B40` computes, for sample `j` of trace `i`:

```
w  = 1/20                        // 0x466A55, 0x465CB0
d  = tj - ti - w
if -w < d < 0.99*w:              // 0x465BBC = 0.99
    s = a * exp(-2.4*|d|/w) * sin(2*pi*d/w)   // a = -1, mean = 0
else:
    s = 0
sample = trunc(1000 * sum_j)     // 0x465CEB..0x465CFC
```

The `0.99` upper gate is exclusive (`jbe` at `0x465B62`). `sum_j` accumulates
`f32` values; the stored sample is an `i32` after truncation toward zero.

## 3. `0x466980` — section construction

* `theta = angle * pi / 180` (`0x467418`, `0x467424`).
* `len` is the f32 profile length; `n = trunc(len/200)+1` with the defensive
  `0x466AF7` decrement; buffer holds at most 100 traces.
* `0x40292C` is `cos`, not a passthrough. The two orientation branches use
  `200*cos(atan(|dy/dx|)) = 200*|dx|/len` and
  `200*cos(atan(|dx/dy|)) = 200*|dy|/len` as the two axis steps. This is the
  same 200 m sampling the previous report described, but the derivation is now
  confirmed byte-for-byte.
* The branches are **not** equivalent:

  | Condition | Time shift | Noise sigma |
  |---|---|---|
  | `|dy| > |dx|` (`0x466B31`) | `0.004` (`0x467444`) | `0.006` (`0x466E1C`) |
  | `|dx| >= |dy|` (`0x466E9A`) | `0.04` (`0x467450`) | `0.002` (`0x67184`) |

* `t_i = f32(2*H/V - shift + noise)`.
* The `-1.0f` at `[ebx+0x190]` (`0x466E61`) is the amplitude array, not a
  sentinel sample at time index 100.
* Tail at `0x67399`/`0x673AB`: `fsubr 0.02` applies to **min** and `fadd 0.084`
  applies to **max**. The old report had these signs reversed.

## 4. `0x465D68` — Image2 renderer

Geometry:

```
scale_t = Height / (t_max - t_min)      // Height = 201
grid_y  = Height / 10                   // [0x466150] = 10
x_step  = Width / (n_traces + 1)        // Width = 800
```

`0x46783C` clears with white. Grid is `RGB(0xD2,0xD2,0xD2)`:

* vertical: `i = 1..n`, `x = trunc(i*x_step)`, `y = 1..Height-1`;
* horizontal: `i = 1..11`, `y = trunc(i*grid_y)`; the 11th line is clipped.

All conversions use `0x402970` (FPU `RC=11` + `fistp`), i.e. **truncation**,
not `Round`. There is no `+1` in the sample Y coordinate:

```
t   = t_min + j/300 * (t_max - t_min)
y   = trunc((t - t_min)*scale_t)        // = trunc(j*Height/300)
x0  = trunc(i*x_step)
rms = sqrt((Σ first_trace_sample² + 1e-8) / 300)
amp = trunc(0.12 / rms * sample * x_step)
```

The RMS sum at `0x465FB0` loops over `0x465D5C = 300` dwords from the first
trace only; it is **not** over all traces. `0.12` is the 80-bit constant at
`0x466154`, `1e-8` is at `0x465F9F`.

For `amp >= 0`, `0x4676CC` draws an independent horizontal `MoveTo+LineTo`
segment. For `amp < 0`, `0x46608B` calls only `LineTo`, so a negative sample
is connected from the previous endpoint; this produces the diagonal joins in
the original. The GUI rasterizer reproduces both cases.

## 5. Noise generator `0x423FF4`

Delphi `Random` (`0x402BC8`) plus Box–Muller:

```
seed = seed * 0x08088405 + 1  (mod 2^32)
rnd  = seed * 2^-32           // [0x46A068]
u    = 2*rnd - 1
r    = u1² + u2², retry while r >= 1
g    = sqrt(-2*ln(r)/r) * u1
out  = g*sigma + mean
```

`DelphiRandom` in `src/grid.rs` preserves this. The original calls
`Randomize` at startup, so the exact noise sequence is process-seeded and
cannot be recovered from the binary. The GUI uses the fixed seed `0` so the
σ=0.002/0.006 jitter is deterministic and does not flicker between frames.

## 6. GUI implementation

`src/grid.rs::build_section` now synthesizes the full 300-sample buffer;
`src/gui.rs::section_image` rasterizes it into an 800×201 `ColorImage` with
`TextureOptions::NEAREST`. It is rebuilt only when the profile, V or angle
changes, so per-frame cost is one textured quad instead of thousands of
egui shapes. Envelope/Ricker wiggles were removed.

## 7. DFM facts applied to the GUI

* `clMoneyGreen` = `$C0DCC0` → RGB `(192,220,192)`.
* Menu captions: `Структура ` (trailing space), `Cтереть таблицу` (Latin C).
* Shortcuts: `F2=113`, `F3=114`, `F4=115`, `Del=46`, `Alt+Del=8238`.
* `TabOrder`: Edit2=0, StringGrid=1, Edit1=2, Edit3=3, Memo1=4,
  Edit4=1, Edit5=2. The GUI expands the grid into four stops and adds a
  focus proxy for Memo1.
* The pick handler at `0x468780` (the callgraph label `N1Click` is unreliable)
  receives `[ebp+8]=Y`, `[ebp+0xC]=X`. It draws a 12×12 marker at `(X−6, Y−6)`,
  formats `t = t_min + Y/Image2.Height·(t_max−t_min)` with three decimals, and
  appends `t01`, `t02`, … to Memo1. Time grows **downward**.
* Map↔Image1 transforms (`0x466198`, `0x4661F0`):
  `sx = trunc(x/8000 * 799)`, `sy = 399 - trunc(y/4000 * 399)`;
  inverse `x = sx/799*8000`, `y = (399-sy)/399*4000`.
* `Image1.OnMouseDown=begin_prf` (`0x46805C`) maps to `drag_started()` in egui.
  `Image2.OnMouseDown=coora` rebuilds the section; pick addition is the
  separate `0x468780` body and fires on mouse-down.

## 8. Correctness caveat

The labels at `0x46805C` in the older callgraph report are unreliable: that
body is byte-identical to `0x469514 EntryHandleClick`. DFM event bindings were
treated as ground truth. FPU `dword ptr` operands were re-decoded as `f32`,
80-bit constants as extended precision, and `0x40292C` as `cos`, instead of
trusting the report's `dbl=` annotations and callgraph names.
