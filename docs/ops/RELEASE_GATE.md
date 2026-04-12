# Release Gate（工业 ERP/WMS）

## 目标

上线前用统一门禁命令验证以下能力：
- 系统治理链路（流程发布/生效）
- 统计与可观测链路（分析引擎与 SSO 健康态）
- 关键业务主链路（平台接入、OMS、WMS、售后、计划执行）

## 一键命令

仓库根目录执行（推荐一键）：

```bash
pnpm run release:gate
```

该命令包含：
- `pnpm run repo:verify`
- `cargo test -p erp-api --test stats_clickhouse_sso_smoke -- --nocapture`
- `pnpm --dir apps/erp-web run generate:api`
- `pnpm --dir apps/erp-web build`
- `pnpm run test:e2e:release-gate`

也可只执行 E2E 门禁：

```bash
pnpm run test:e2e:release-gate
```

如需包含 Task1~Task9 关键后端集成测试，使用全量门禁：

```bash
pnpm run release:gate:full
```

该命令会额外执行：

```bash
pnpm run test:backend:release-gate
```

该命令会串行执行以下 E2E：
- `e2e/system-governance.spec.ts`
- `e2e/task8-stats-observability-sso.spec.ts`
- `e2e/release-gate-main-chain.spec.ts`
- `e2e/release-gate-cross-chain.spec.ts`

## 配套验证

建议在门禁前后补充：

```bash
cargo test -p erp-api --test stats_clickhouse_sso_smoke -- --nocapture
pnpm --dir apps/erp-web run generate:api
pnpm --dir apps/erp-web build
```

## 通过标准

1. 所有 release-gate E2E 通过（0 failed）
2. 后端 smoke 测试通过（`stats_clickhouse_sso_smoke`）
3. 前端 build 成功
4. OpenAPI schema 生成成功

## 失败处理

1. 优先看失败用例对应页面/接口是否为新变更触发
2. 若为权限/鉴权失败，先核对 `x-permissions` 与后端 `ensure_permission` 是否一致
3. 若为页面定位失败，优先补稳定 `data-testid`，避免 brittle selector
4. 修复后必须重新执行完整 release-gate 命令，不允许只跑单测后宣称通过
