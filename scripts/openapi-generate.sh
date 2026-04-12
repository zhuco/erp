#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SPEC_PATH="${ROOT_DIR}/openapi/erp_api_v1.yaml"
OUTPUT_PATH="${ROOT_DIR}/apps/erp-web/src/api/generated/schema.ts"

pnpm --filter erp-web exec openapi-typescript "$SPEC_PATH" -o "$OUTPUT_PATH"
