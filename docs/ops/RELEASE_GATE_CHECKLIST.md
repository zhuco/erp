# Release Gate Checklist（执行版）

## 1. 契约与后端

- [ ] `pnpm run repo:verify`
  - 通过标准：输出包含 `OpenAPI 门禁校验通过`
- [ ] `cargo test -p erp-api --test stats_clickhouse_sso_smoke -- --nocapture`
  - 通过标准：6/6 通过
- [ ] `pnpm run test:backend:release-gate`
  - 通过标准：Task1~Task9 关键后端集成测试全部通过（允许 DB 依赖用例按现有逻辑 skip）

## 2. 前端构建与类型

- [ ] `pnpm --dir apps/erp-web run generate:api`
  - 通过标准：`schema.ts` 生成成功
- [ ] `pnpm --dir apps/erp-web build`
  - 通过标准：构建成功、无 fatal error

## 3. 发布门禁 E2E

- [ ] `pnpm run test:e2e:release-gate`
  - 覆盖：
    - `system-governance.spec.ts`
    - `task8-stats-observability-sso.spec.ts`
    - `release-gate-main-chain.spec.ts`
    - `release-gate-cross-chain.spec.ts`
  - 通过标准：全部通过（0 failed）

## 4. 上线结论

- [ ] Go：以上三部分全部通过
- [ ] No-Go：任一部分失败，必须修复后重新跑完整门禁

## 5. 一键全量门禁

- [ ] `pnpm run release:gate:full`
  - 通过标准：命令整体退出码为 0
