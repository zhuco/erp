# ERP System Makefile
# 兼容包装层：官方入口是根级 package.json scripts；此文件仅做兼容调用。

.PHONY: help accept-web accept-backend accept-all web-lint web-typecheck web-build web-test web-test-e2e progress-check

# Default target
help:
	@echo "ERP System - Available targets:"
	@echo ""
	@echo "  Web UI:"
	@echo "    accept-web      - Run all Web UI production gates"
	@echo "    accept-backend  - Run all backend production gates"
	@echo "    accept-all      - Run backend + web production gates"
	@echo "    web-lint        - Run ESLint"
	@echo "    web-typecheck   - Run TypeScript type check"
	@echo "    web-build       - Build production bundle"
	@echo "    web-test        - Run Web unit tests only"
	@echo "    web-test-e2e    - Run E2E tests"
	@echo ""
	@echo "  Ops:"
	@echo "    progress-check  - Run progress check and report"
	@echo ""

# Web UI acceptance (one-click)
accept-web:
	@echo "Running Web UI acceptance checks..."
	@pnpm accept:web

accept-backend:
	@echo "Running backend acceptance checks..."
	@pnpm accept:backend

accept-all:
	@echo "Running full acceptance checks..."
	@pnpm accept:all

progress-check:
	@bash scripts/progress_check.sh

# Individual targets
web-lint:
	@pnpm lint

web-typecheck:
	@pnpm typecheck

web-build:
	@pnpm build

web-test:
	@pnpm --filter erp-web test:unit

web-test-e2e:
	@pnpm test:e2e
