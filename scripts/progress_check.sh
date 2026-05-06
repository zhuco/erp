#!/usr/bin/env bash
set -uo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${ROOT_DIR}/tmp/progress-reports"
TIMESTAMP="$(date +"%Y%m%d_%H%M%S")"
REPORT_FILE="${REPORT_DIR}/progress_check_${TIMESTAMP}.md"

mkdir -p "${REPORT_DIR}"

FAILED=0

write_header() {
  cat <<EOF >"${REPORT_FILE}"
# Progress Check Report
- Generated at: $(date -Iseconds)
- Repo: ${ROOT_DIR}
EOF
}

append_section() {
  local title="$1"
  echo "" >>"${REPORT_FILE}"
  echo "## ${title}" >>"${REPORT_FILE}"
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

  echo "-> ${title}: ${status}"
}

record_skip() {
  local title="$1"
  local reason="$2"
  append_section "${title}"
  echo "- Status: SKIPPED" >>"${REPORT_FILE}"
  echo "- Reason: ${reason}" >>"${REPORT_FILE}"
}

check_openapi() {
  local file="${ROOT_DIR}/openapi/erp_api_v1.yaml"
  if [[ ! -f "${file}" ]]; then
    echo "OpenAPI file missing: ${file}"
    return 1
  fi

  local -a paths=(
    "/healthz:"
    "/auth/login:"
    "/reports/dashboard:"
    "/v1/tenants/{tenant_id}/orders:"
    "/v1/tenants/{tenant_id}/inventory:"
  )

  for path in "${paths[@]}"; do
    if ! rg -F -q "${path}" "${file}"; then
      echo "Missing OpenAPI path: ${path}"
      return 1
    fi
  done
}

check_migrations() {
  local -a tables=(
    "tenants"
    "shops"
    "orders"
    "order_items"
    "outbox_events"
    "inventory_balances"
  )
  local missing=()

  for table in "${tables[@]}"; do
    if ! rg -F -q "create table if not exists ${table}" "${ROOT_DIR}/migrations"; then
      missing+=("${table}")
    fi
  done

  if [[ ${#missing[@]} -gt 0 ]]; then
    echo "Missing tables in migrations: ${missing[*]}"
    return 1
  fi
}

check_rg_scan() {
  local -a scans=(
    "migrations:tenant_id"
    "crates:tenant_id"
    "openapi:x-permissions"
  )

  local scope pattern
  for entry in "${scans[@]}"; do
    scope="${entry%%:*}"
    pattern="${entry#*:}"
    if ! rg -F -q "${pattern}" "${ROOT_DIR}/${scope}"; then
      echo "Missing pattern '${pattern}' in ${scope}"
      return 1
    fi
  done
}

check_frontend() {
  if [[ -x "${ROOT_DIR}/scripts/accept_web.sh" ]]; then
    bash "${ROOT_DIR}/scripts/accept_web.sh"
    return $?
  fi

  if rg -q "^accept-web" "${ROOT_DIR}/Makefile"; then
    make -C "${ROOT_DIR}" accept-web
    return $?
  fi

  if ! command -v pnpm >/dev/null 2>&1; then
    echo "pnpm not found; install with: npm install -g pnpm"
    return 1
  fi

  pnpm --filter erp-web build
}

check_smoke() {
  local webhook_url="${PROGRESS_SMOKE_WEBHOOK_URL:-}"
  if [[ -z "${webhook_url}" ]]; then
    echo "PROGRESS_SMOKE_WEBHOOK_URL is required when PROGRESS_SMOKE=1"
    return 1
  fi

  curl -fsS -XPOST "${webhook_url}" \
    -H "Content-Type: application/json" \
    -d '{"platform_order_id":"SMOKE_001","created_at":"2026-01-06T10:00:00Z","total_cent":1000}' \
    >/dev/null
}

write_header

run_check "Tooling: ripgrep" "rg --version" rg --version
run_check "Backend: cargo fmt" "cargo fmt --all" cargo fmt --all
run_check "Backend: cargo clippy" "cargo clippy --all-targets --all-features -- -D warnings" \
  cargo clippy --all-targets --all-features -- -D warnings
run_check "Backend: cargo test" "cargo test --all" cargo test --all
run_check "Backend: cargo build" "cargo build --all" cargo build --all
run_check "OpenAPI coverage" "rg -F -q <paths> openapi/erp_api_v1.yaml" check_openapi
run_check "Migrations: key tables" "rg -F -q create table if not exists <table> migrations/" check_migrations
run_check "RG scan: tenant/outbox/permissions" "rg -F -q <pattern> <scope>" check_rg_scan
run_check "Frontend: acceptance" "scripts/accept_web.sh (or make accept-web)" check_frontend

if [[ "${PROGRESS_SMOKE:-}" == "1" || "${PROGRESS_SMOKE:-}" == "true" ]]; then
  run_check "Smoke test: webhook ingestion" "curl -XPOST ${PROGRESS_SMOKE_WEBHOOK_URL}" check_smoke
else
  record_skip "Smoke test: webhook ingestion" "Set PROGRESS_SMOKE=1 and PROGRESS_SMOKE_WEBHOOK_URL"
fi

append_section "Summary"
if [[ ${FAILED} -eq 0 ]]; then
  echo "- Status: PASS" >>"${REPORT_FILE}"
else
  echo "- Status: FAIL" >>"${REPORT_FILE}"
fi

echo "Report written to ${REPORT_FILE}"

if [[ ${FAILED} -ne 0 ]]; then
  exit 1
fi
