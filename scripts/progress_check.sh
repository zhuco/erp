#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ROOT_DIR}/tmp/progress-reports"
TIMESTAMP="$(date +"%Y%m%d_%H%M%S")"
REPORT_FILE="${REPORT_DIR}/progress_check_${TIMESTAMP}.md"
FAILED=0

mkdir -p "${REPORT_DIR}"
cat >"${REPORT_FILE}" <<EOF
# Progress Check Report
- Generated at: $(date -Iseconds)
- Roadmap: docs/ROADMAP.md
- UI: Rust egui native + WASM
- Legacy Web source metrics: retired and excluded
EOF

append_section() {
  printf '\n## %s\n' "$1" >>"${REPORT_FILE}"
}

run_check() {
  local title="$1"
  local display_cmd="$2"
  shift 2
  append_section "${title}"
  echo "- Command: \`${display_cmd}\`" >>"${REPORT_FILE}"
  local output status
  output="$("$@" 2>&1)"
  status=$?
  if [[ ${status} -eq 0 ]]; then
    echo "- Status: PASS" >>"${REPORT_FILE}"
  else
    echo "- Status: FAIL" >>"${REPORT_FILE}"
    echo "- Output (tail 40 lines):" >>"${REPORT_FILE}"
    echo '```' >>"${REPORT_FILE}"
    echo "${output}" | tail -n 40 >>"${REPORT_FILE}"
    echo '```' >>"${REPORT_FILE}"
    FAILED=1
  fi
}

record_unknown() {
  append_section "$1"
  echo "- Status: UNKNOWN" >>"${REPORT_FILE}"
  echo "- Reason: $2" >>"${REPORT_FILE}"
}

if ! command -v pnpm >/dev/null 2>&1; then
  record_unknown "Repository policy" "pnpm unavailable; gate not run"
  record_unknown "Rust UI native checks" "pnpm unavailable; gates not run"
  record_unknown "Rust UI WASM acceptance" "pnpm unavailable; gate not run"
  record_unknown "Backend acceptance" "pnpm unavailable; gate not run"
  FAILED=1
else
  run_check "Repository policy" "pnpm run repo:verify" pnpm -C "${ROOT_DIR}" run repo:verify
  run_check "Rust UI native typecheck" "pnpm run typecheck" pnpm -C "${ROOT_DIR}" run typecheck
  run_check "Rust UI native Clippy" "pnpm run lint" pnpm -C "${ROOT_DIR}" run lint
  run_check "Rust UI native tests" "pnpm run test:ui" pnpm -C "${ROOT_DIR}" run test:ui
  run_check "Rust UI WASM acceptance" "pnpm run accept:web" pnpm -C "${ROOT_DIR}" run accept:web
  run_check "Backend acceptance" "pnpm run accept:backend" pnpm -C "${ROOT_DIR}" run accept:backend
fi

append_section "Summary"
if [[ ${FAILED} -eq 0 ]]; then
  echo "- Status: PASS" >>"${REPORT_FILE}"
else
  echo "- Status: FAIL_OR_UNKNOWN" >>"${REPORT_FILE}"
fi

echo "Report written to ${REPORT_FILE}"
[[ ${FAILED} -eq 0 ]]
