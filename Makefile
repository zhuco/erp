# ERP System Makefile
# 兼容包装层：官方入口是根级 package.json scripts。

.PHONY: help dev build lint typecheck test-ui accept-web accept-backend accept-all progress-check

help:
	@echo "ERP System - Available targets:"
	@echo ""
	@echo "  dev             - Build and run the native Rust UI"
	@echo "  build           - Build the Rust UI"
	@echo "  lint            - Run Rust UI lint checks"
	@echo "  typecheck       - Check native Rust UI targets (accept-web checks Web)"
	@echo "  test-ui         - Run Rust UI tests"
	@echo "  accept-web      - Run Rust Web UI acceptance checks"
	@echo "  accept-backend  - Run backend acceptance checks"
	@echo "  accept-all      - Run backend and Web UI acceptance checks"
	@echo "  progress-check  - Run progress check and report"

dev:
	@pnpm run dev

build:
	@pnpm run build

lint:
	@pnpm run lint

typecheck:
	@pnpm run typecheck

test-ui:
	@pnpm run test:ui

accept-web:
	@pnpm run accept:web

accept-backend:
	@pnpm run accept:backend

accept-all:
	@pnpm run accept:all

progress-check:
	@bash scripts/progress_check.sh
