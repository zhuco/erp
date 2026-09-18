# PERF-02 measurement, 2026-09-10

Release builds from the controlled ERP build entrypoint; 10,000 rows × 100
columns, two left and two right fixed columns, 1280 × 800 at scale 1. Five warm-up
frames followed by three ten-second bidirectional scroll rounds. No API requests.

| Runtime | Frame interval P95 | Result |
| --- | ---: | --- |
| Edge 152.0.4191.66, headless WebGPU | 18.4 ms | Timing budget passed |
| Windows, NVIDIA Vulkan | 92.4562 ms | Failed 32 ms budget |
| Windows, NVIDIA DX12 | 94.0317 ms | Failed 32 ms budget |

Web raw evidence: `web-latest.json`, 1,700 unique-frame samples, round counts
566/567/567, 30.0153 seconds, all focused. Screenshots `web-8s.png` and
`web-17s.png` and `web-26s.png` were reviewed: serials and fixed columns remain visible and sampled
data has no gaps. This is sampled visual evidence, not proof of every frame.
The three screenshot captures are included in the measured workload.

Native evidence: `native-vulkan.json`/`native-vulkan-metadata.json` and
`native-latest.json`/`native-metadata.json` (DX12). DX12 collected 363 frames in
30.0336618 seconds; first UI-pass CPU P95 was 4.8403 ms. This is not GPU time or
presentation latency. Its sampled lifetime peak working-set counter was
188,334,080 bytes. Vulkan and DX12 both failed; the cause is not established.
Changing the default graphics backend is not justified by these measurements.

Hardware: Intel Core i7-8700K, 6 cores/12 logical processors; GTX 1070 Ti, NVIDIA
driver 560.94. Windows also exposes a Todesk Virtual Display Adapter. Its presence
alone does not prove that it caused the native result. Browser adapter name was
unavailable; the report identifies BrowserWebGpu. Browser memory records six
individual process lifetime peaks, not a simultaneous combined memory peak.

The earlier 1920 × 1009 native report and fixed-viewport pre-batching report are
retained as `native-initial-1920.json` and `native-before-batching.json`. Reusing a
persisted window size made the first run unsuitable for direct comparison.
Sharing clipping for fitting cell text did not establish a native timing benefit.

Measured native executable SHA256:
`DE5452ABDC390F8A7E93953A0A3C9C39DB902CF56D80D92B16B9EA6D5FDAB13F`

Measured WASM SHA256:
`A9AA15D6911726F7CDC0497869327E9E41FFC53195A20181E5B69F10F9774C31`

The Web collector's first attempt could not serialize its result because the
Playwright run-code sandbox does not expose Node Buffer. Encoding now runs in the
browser with TextEncoder/btoa; the subsequent real collection succeeded. The
independent validator recomputes P95 from samples and rejects wrong scenarios,
missing rounds, unfocused samples and inconsistent raw/parsed reports. Seven
deterministic validator tests passed; the saved Web report also passed this gate.

PERF-01/03/04/05, native interactive input, Chinese IME, screen readers and physical
printing are not certified by PERF-02. The complete component goal remains open.

## 0.58 native presentation comparison (2026-09-10)

The same release executable measured the unchanged PERF-02 grid twice, with no
build running during either measurement. Both used NVIDIA GTX 1070 Ti / Vulkan,
focused viewport samples, five warm-up frames and three 10-second rounds.

- AutoVsync baseline: P95 85.3637 ms, 808 samples; previous complete eframe CPU
  mean 6.416 ms, maximum 19.331 ms. The independent 32 ms gate correctly failed.
- AutoNoVsync: P95 7.7168 ms, 4952 samples; previous complete CPU mean 5.381 ms,
  maximum 11.462 ms. Independent validation passed, rounds 1442/1769/1741.
- Evidence: native-058-vsync.json and native-058-no-vsync.json, each with its
  corresponding -metadata.json (executable hash, machine and memory evidence).
  native-pre-058.json preserves the earlier latest report.

This implicates presentation waiting rather than the DataGrid UI pass as the
main bottleneck on this machine; it does not identify a specific driver defect.
Following the comparison, desktop and benchmark defaults were aligned to
AutoNoVsync, retaining an explicit PERF-only auto-vsync override. The subsequent
build includes the independent main/subgrid popup ID fix; the timing reports
remain attributed to their recorded executable, not silently to a newer hash.
wgpu's Immediate-first fallback can tear, so visual review remains outstanding.
PERF-01/03/04/05, physical IME, screen readers and printing remain separate.

## 0.62 HTTP page fixture (2026-09-10)

PageLab uses actual HTTP, shared RemotePages and DataGrid against the isolated
million-row logical fixture. Each endpoint request generates only 100 rows.
Both native and Web completed 35 pairs: the old request was delayed by 250 ms,
the newer response was sent first, and all 35 old responses were rejected as stale.
Independent server sentOrder checks verified every pair. Each client received
448,310 bytes, ended with one cached 100-row block and zero active HTTP tasks.

After five warm-up pairs, 30 samples measured HTTP completion to grid drawing
and a processed synthetic ArrowDown event. Independently sorted P95 values:
native 2.8932 ms; Web 31.3 ms; both below the 300 ms component budget.
Evidence: page-native.json, page-web.json, their -server.json files, page-web.log,
and page.metadata.json with executable/WASM SHA256. Web viewport was 1280x800.
The browser session was closed after collection. Its favicon 404 and Windows
powerPreference warning were not grid errors.

This is a synthetic-server component result, not ERP database latency, physical
input or GPU presentation. Transport and deliberately injected server delay are
outside the reported completion-to-ready interval. Full RSS/GPU lifecycle and
resident-data update scenarios remain separately unverified.

## 0.63 resident update scenario (2026-09-10)

UpdateLab now materializes 10,000 local rows and renders that dataset through
DataGrid virtualization. The public update queue and 100-row remote cache remain
bounded; only changed rows are copied into the resident model. The benchmark does
not enlarge production cache limits. Every version transition uses the existing
CAS path; final validation checks identity, version and Revision for all 10,000
rows (first ten at revision 600, all remaining rows unchanged at revision 0).

Both native and Web completed 60 seconds at 100 scheduled events/second, with
6,000 samples, no error and zero queued items/bytes. Independently sorted draw
latency P95: native 0.9600 ms; Web 17.0 ms, below the 200 ms budget. Evidence:
update-resident-native.json, update-resident-web.json, Web log and
update-resident.metadata.json. Native two-second process samples recorded peak
working set 131,813,376 bytes and peak private bytes 436,092,928 bytes; these are
sampled process counters, not allocator/GPU attribution or a leak test.

Native/Web Clippy passed (2.94s/2.22s), native release 42.20s, Web release compile
28.54s and packaging exited successfully. No extra regression suite was needed
for this benchmark-only dataset change. Owned native process exited and browser
session erp-updates-063 was closed. Old 100-row native report is update-060.json.

This measures local scheduled updates to UI drawing. Intermediate versions may
be coalesced within a frame; it does not prove every version was presented by the
GPU. Network, physical input and full lifecycle PERF-04 remain separate.
