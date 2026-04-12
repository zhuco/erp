#!/usr/bin/env bash
set -euo pipefail

# 兼容包装层：官方入口是根级 package.json scripts。
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

pnpm accept:backend
