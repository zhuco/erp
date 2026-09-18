# Grid rendering measurements

PERF-02 uses the real DataGrid in an isolated application mode: 10,000 generated
rows, 100 columns, two left and two right pins, five warm-up frames and three
10-second bidirectional scroll rounds. It never authenticates or queries business
services. A unique egui frame number prevents extra layout passes from counting
as extra frames. First-pass CPU time and inter-frame time are separate fields.

Build only through the registered entrypoints, using the existing fixed cache:

```powershell
pwsh -File scripts/Invoke-ErpBuild.ps1 -Action build -Scope grid -Release
pwsh -File scripts/Build-ErpWeb.ps1 -Release
```

Finish builds before measuring, so compilation does not compete with rendering.
Launch `build/cargo/release/erp-grid-lab.exe --grid-perf` from the project directory.
In a second terminal, run `pwsh -File scripts/Measure-ErpGrid.ps1 -MeasurementProcessId <PID>`
with that exact process ID. Its window closes after measurement. The collector
validates the executable and argument, samples memory, and never terminates a
process. An initial hidden-launch attempt did not produce frames, so the collector
uses explicit attachment to an already-running renderer. Early memory samples may
precede attachment; the OS lifetime peak working-set counter is retained.

The desktop and benchmark now both default to `AutoNoVsync`, following the 0.58
same-binary comparison. For the prior vsync baseline only, set
`ERP_GRID_PERF_PRESENT_MODE=auto-vsync` before launching `--grid-perf`.
The override affects only the benchmark; it does not change the 32 ms budget.
wgpu 30 chooses Immediate, then Mailbox, then Fifo for AutoNoVsync. Immediate can
tear on some displays, so visual validation remains distinct from timing.

Serve the generated release package with `node qa/acceptance/serve-web.mjs`, then
run `node qa/performance/run-web.mjs`. It uses the pinned Playwright CLI, a separate
browser session and no business API. Browser screenshots sample different points
in the three rounds. The browser closes in `finally`.

Raw samples are retained in `native-latest.json` and `web-latest.json`. Native
hardware/toolchain/hash and memory samples are in `native-metadata.json`. On
Windows, browser CDP process IDs are used to read per-process working-set peaks
before closing the browser. These lifetime per-process maxima must not be summed
and described as simultaneous total peak memory. JavaScript heap size is not total
browser memory. Missing platform measurements must remain explicitly unavailable.

`interval_budget_met` only evaluates the PERF-02 32 ms P95 timing requirement with
at least 30 samples in each round and a full 30-second run. It does not certify
visual gap absence, the other four performance scenarios, system input methods,
screen readers, real backend data, or product release. Screenshots need review;
native visual review remains separate. A debug build must not produce a passing
measurement. Preserve failed raw reports instead of changing the budget.

Both collectors additionally use `validate-report.mjs` / `perf02-validation.mjs`
to validate scenario shape, viewport, focus, sample order and round coverage, and
recompute P95 independently. Validate an existing report without rerunning the
measurement with `node qa/performance/validate-report.mjs <report.json>`. Validator
regressions use deterministic fixtures via
`node --test qa/performance/perf02-validation.test.mjs`.
