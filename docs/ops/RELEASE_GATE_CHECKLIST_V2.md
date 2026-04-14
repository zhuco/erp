# Release Gate Checklist（执行版 / V2）

## 0. 执行记录（窗口10）

- 执行时间：2026-04-14（Asia/Shanghai）
- 执行窗口：系统治理与门禁（共享文件收口窗口）
- 结论：`NO-GO`（环境门禁阻断），其余构建与 E2E 通过

| 序号 | 验证命令 | 结果 | 关键证据 |
| --- | --- | --- | --- |
| 1 | `pnpm run repo:verify` | PASS | `[openapi-gate] OpenAPI gate verification passed` |
| 2 | `pnpm run test:backend:release-gate` | FAIL / NO-GO | `[release-gate] DATABASE_URL/TEST_DATABASE_URL 未配置。发布门禁禁止在无数据库配置下执行（NO-GO）。` |
| 3 | `pnpm --dir apps/erp-web run generate:api` | PASS | `openapi-typescript ... -> apps/erp-web/src/api/generated/schema.ts` |
| 4 | `pnpm --dir apps/erp-web build` | PASS | `✓ built in 12.23s`（存在 chunk 告警，不阻断） |
| 5 | `pnpm run test:e2e:release-gate` | PASS | `4 passed (18.7s)` |

## 1. 契约一致性报告（OpenAPI / 路由 / 前端类型）

### 1.1 OpenAPI 门禁脚本核验（通过）
- 校验脚本：`scripts/verify_openapi_gate_v2.mjs`
- 核验维度：
  - 权限声明 `x-permissions`
  - 标准状态码 `200/400/401/403`
  - 写接口持久化声明 `x-persistence-required: true` 与 `x-persistence-tables`
- 覆盖重点：
  - 商品匹配主链路（`/shop-products/*`）
  - 系统治理链路（`/system/workflows`、`/system/integrations/shops`）
  - 观测与统计链路（`/stats/health`、`/stats/{stats_type}`）

### 1.2 前后端契约联动（通过）
- 前端类型生成：`pnpm --dir apps/erp-web run generate:api` 成功。
- 说明：OpenAPI 可被前端 `openapi-typescript` 正常消费，契约未出现结构性断裂。

### 1.3 路由与权限隔离证据（通过）
- 菜单权限声明见：`apps/erp-web/src/app/layout/menu.tsx`
  - 系统治理页使用 `SYS.SETTINGS.READ` / `SYS.USER.READ` / `SYS.ROLE.READ` / `SYS.PERMISSION.READ` / `SYS.AUDIT.READ` / `SYS.DLQ.READ`。
- 路由守卫证据见：`apps/erp-web/src/app/router.tsx`
  - 系统页、审计页、观测页均按权限守卫挂载。

## 2. 发布门禁清单（本次实测）

## 2.1 契约与后端门禁
- [x] `pnpm run repo:verify`
  - 通过标准：输出包含 `[openapi-gate] OpenAPI gate verification passed`
- [ ] `pnpm run test:backend:release-gate`
  - 通过标准：关键后端测试全部通过，且**无 `skipping ... DATABASE_URL` / `pool timed out`**
  - 阻断规则：出现关键 DB skip 直接 **NO-GO**
  - 本次结果：环境变量未配置，命中 NO-GO

## 2.2 前端构建与类型
- [x] `pnpm --dir apps/erp-web run generate:api`
  - 通过标准：`schema.ts` 生成成功
- [x] `pnpm --dir apps/erp-web build`
  - 通过标准：构建成功且无 fatal error

## 2.3 发布门禁 E2E
- [x] `pnpm run test:e2e:release-gate`
  - 覆盖：
    - `system-governance.spec.ts`
    - `task8-stats-observability-sso.spec.ts`
    - `release-gate-main-chain.spec.ts`
    - `release-gate-cross-chain.spec.ts`
  - 通过标准：0 failed

## 2.4 数据库与审计硬门禁（企业级）
- [ ] 环境变量校验：`DATABASE_URL` 或 `TEST_DATABASE_URL` 至少配置一个
  - 阻断规则：均未配置直接 **NO-GO**
- [ ] 核心操作有落库证据（至少命中一类关键表）
  - `order_action_log`
  - `order_status_history`
  - `operation_logs`
  - `audit_logs`
- [ ] 商品匹配链路写操作有持久化证据（示例）
  - `shop_product_tasks`
  - `shop_products`
  - `shop_product_skus`
  - `channel_sku_mappings`

## 3. 上线结论

- [ ] Go：全部检查通过
- [x] No-Go：任一项失败，或关键 DB 测试被 skip
- 本次 NO-GO 根因：
  - 后端发布门禁在环境前置检查阶段阻断（`DATABASE_URL/TEST_DATABASE_URL` 缺失）。

## 4. 一键全量门禁

- [ ] `pnpm run release:gate:full`
  - 通过标准：整体退出码为 0
