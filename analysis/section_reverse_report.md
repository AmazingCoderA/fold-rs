# Section renderer and DFM input — reverse-engineering pass (v0.10.3)

Primary sources: `analysis/callgraph_report.txt`, `analysis/dfm_values_out.txt`,
`analysis/memory/unpacked_image.bin` (base `0x400000`). The screenshot is used
only as an optional scale check, never as a source of coordinates.

## 1. `0x465D68` — Image2 renderer

Header at `0x46CD68`, copied into a local 30005-dword buffer:

| Offset | Value |
|---|---|
| `+0` | `1` |
| `+4` | `0x12C = 300` samples per trace |
| `+8` / `+0xC` | `t_min` / `t_max` |
| `+0x10` | `n_traces` |
| `+0x14` | samples, stride `0x4B0` = 300 dwords |

Geometry (decoded from the dump):

```
scale_t = Height / (t_max - t_min)      // 0x465DC8, height = 201
grid_y  = Height / 10                   // [0x466150] = 10
x_step  = Width / (n_traces + 1)        // Width = 800
```

`0x46783C` clears Image2 with a pure white brush (`0xFFFFFF`). Vertical axes
are drawn at `Round(i*x_step)`, `i = 1..n_traces`, from `y=1` to `y=Height-1`
in `RGB(0xD2,0xD2,0xD2)`. Horizontal lines use `Round(i*grid_y)`, `i=1..11`;
the 11th is outside the control and therefore invisible.

For each of the 300 samples:

```
t   = t_min + (i-1)/300 * (t_max - t_min)
y   = Round(1 + (t - t_min)*scale_t)
rms = sqrt((Σ sample² + 1e-8) / 300)     // [0x465F24] = 0x322BCC77
amp = Round(0.12 / rms * sample * x_step)
```

Negative lobes are filled with `LineTo`; the outline is `MoveTo`+`LineTo`
with a black pen. The pick list is **not** used to position traces.

## 2. `0x466980` — section construction

* `t = 2H/V`, `theta = angle * pi / 180` (`0x467418`, `0x467424`, `0x467428`).
* Step along the profile is a fixed `200.0` m (`0x46742C`).
* `n = trunc(len/200)+1` (`0x402970` sets FPU trunc), with a defensive
  decrement branch at `0x466AF7` that cannot trigger for this `n`; the sample
  buffer holds at most 100 traces.
* Before the buffer header is written, the time range is narrowed with
  `t_min += 0.084` (`0x467468`) and `t_max -= 0.02` (`0x46745C`).

## 3. Noise generator `0x423FF4`

Disassembled directly from the dump. This is Delphi `Random`
(`0x402BC8`) plus Box–Muller:

```
seed = seed * 0x08088405 + 1  (mod 2^32)
rnd  = seed * 2^-32           // [0x46A068]
u    = 2*rnd - 1
r    = u1² + u2², retry while r >= 1
g    = sqrt(-2*ln(r)/r) * u1
out  = g*sigma + mean
```

The call at `0x466E2B` passes `sigma = 0.006` (80-bit constant) and
`mean = 0`; each sample is `2H/V - 0.004` (`[0x467444] = 0.004`)
plus this noise.

`DelphiRandom`, `SECTION_NOISE_SIGMA` and `SECTION_TIME_SHIFT` in
`src/grid.rs` preserve this result. The exact ordering of the 300-sample
buffer is still ambiguous, so the GUI currently draws envelope traces from
`t=2H/V` rather than pretending to reproduce the full seismogram.

## 4. DFM facts applied to the GUI

* `clMoneyGreen` = `$C0DCC0` → RGB `(192,220,192)`.
* Menu captions: `Структура ` (trailing space), `Cтереть таблицу` (Latin C).
* Shortcuts: `F2=113`, `F3=114`, `F4=115`, `Del=46`, `Alt+Del=8238`.
* `TabOrder`: Edit2=0, StringGrid=1, Edit1=2, Edit3=3, Memo1=4,
  Edit4=1, Edit5=2. The GUI expands the grid into four stops and adds a
  focus proxy for Memo1.
* Pick labels from `0x468830..0x4688AB`: `t01`, `t02`, … with three decimals.
* Map↔Image1 transforms (`0x466198`, `0x4661F0`):
  `sx = trunc(x/8000 * 799)`, `sy = 399 - trunc(y/4000 * 399)`;
  inverse `x = sx/799*8000`, `y = (399-sy)/399*4000`.
  The profile overlay previously ignored the Y flip and disagreed with the
  texture rasterization; both now use the same transform.
* `Image1.OnMouseDown=begin_prf` (`0x468140`) — in egui the equivalent is
  `drag_started()`; `clicked()` is release-only and never fires for a decided
  drag.

## 5. Correctness caveat

The labels at `0x46805C` in the older callgraph report are unreliable: that
body is byte-identical to `0x469514 EntryHandleClick`. The real
`Image1.OnMouseDown` handler is `0x468140`; DFM event bindings were treated as
ground truth. FPU `dword ptr` operands were re-decoded as `f32`, and 80-bit
constants as extended precision, instead of trusting the report's `dbl=`
annotations.
