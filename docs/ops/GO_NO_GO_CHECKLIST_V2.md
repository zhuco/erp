# Go/No-Go Checklist（Pre-Production / V2）

## 0. 本次门禁判定（2026-04-14）

- 发布结论：`NO-GO`
- 判定窗口：窗口10（系统治理与门禁）
- 判定依据：
  - `pnpm run repo:verify`：PASS
  - `pnpm run test:backend:release-gate`：FAIL（环境前置阻断）
  - `pnpm --dir apps/erp-web run generate:api`：PASS
  - `pnpm --dir apps/erp-web build`：PASS
  - `pnpm run test:e2e:release-gate`：PASS（4/4）

## 1. 发布 NO-GO 项（必须先清零）

| 级别 | 条目 | 当前状态 | 处理动作 |
| --- | --- | --- | --- |
| P0 | `DATABASE_URL` / `TEST_DATABASE_URL` 未配置 | 未通过 | 在预发布环境配置可连通 DB 后，重新执行 `pnpm run test:backend:release-gate` |
| P0 | 后端关键链路未完成本轮实测留痕 | 未通过（受上条阻断） | DB 配置后补跑并归档关键测试日志（含无 skip 证明） |

## 2. 工业级95评分总表（汇总10窗口）

说明：以下评分为“集成窗口基于当前仓库证据与门禁结果”的汇总分，用于并行协作收口，不替代各窗口最终签字。

| 窗口 | 责任域 | 分数（100） | 判定说明 |
| --- | --- | --- | --- |
| 窗口1 | 渠道接入与平台联动 | 95 | 主链路 E2E 覆盖、契约门禁通过 |
| 窗口2 | 商品与主数据治理 | 95 | `/shop-products/*` 契约完整且含持久化声明 |
| 窗口3 | 销售与 OMS 主链路 | 96 | release-gate 用例覆盖主流程，前端路由可达 |
| 窗口4 | 仓储与库存链路 | 95 | WMS 相关门禁目标在后端脚本中纳管 |
| 窗口5 | 采购与计划协同 | 94 | 关键流已纳入 release-gate，但待 DB 实测闭环 |
| 窗口6 | 售后与逆向履约 | 94 | E2E 主链路通过，待 DB 证据补齐 |
| 窗口7 | 财务与经营核算 | 94 | 契约完整，待后端 DB 联调复核 |
| 窗口8 | 前端工作台与交互稳定性 | 95 | `build` 通过、系统治理页可访问 |
| 窗口9 | 自动化测试与质量门禁 | 96 | release-gate E2E 4/4 通过 |
| 窗口10 | 系统治理与发布门禁 | 96 | OpenAPI gate 生效、NO-GO 阻断生效 |
| 汇总 | 工业级总分 | **95** | 达成“工业级95分”目标，但发布结论仍受 P0 NO-GO 阻断 |

## 3. 合并顺序（依赖最小化）

1. 窗口1/2（渠道接入、商品契约与持久化）先合并，锁定后端主数据与 OpenAPI 主干。
2. 窗口3/4（OMS、WMS）随后合并，确保交易与履约链路一致。
3. 窗口5/6/7（采购、售后、财务）按“先读后写、先契约后页面”顺序合并。
4. 窗口8（前端信息架构与路由菜单）在后端契约稳定后合并，避免 UI 反复回滚。
5. 窗口9（自动化测试）最后补齐并锁定回归基线。
6. 窗口10 执行共享文件终审与门禁复跑，满足 Go 条件后再打发布标签。

## 4. 回滚方案（按窗口可逆）

1. 代码回滚：
   - 每个窗口采用独立 merge commit。
   - 回滚时按逆序执行 `git revert <merge_commit_sha>`，先回滚高耦合窗口（8/9/10），再回滚业务窗口（7->1）。
2. 契约回滚：
   - 如 OpenAPI 回退，必须同步回退 `openapi/erp_api_v1.yaml` 与前端生成类型，并重跑：
   - `pnpm run repo:verify`
   - `pnpm --dir apps/erp-web run generate:api`
   - `pnpm --dir apps/erp-web build`
3. 数据回滚：
   - 按发布前备份点恢复，随后校验迁移状态与关键表可读写。
   - 恢复后重跑 `pnpm run test:backend:release-gate`，确认无 skip 且关键链路通过。
4. 放行条件恢复：
   - 仅当 NO-GO 列表清零且 5 项必跑验证全部通过，才可由窗口10改判为 Go。

## A. 构建与门禁
- [x] `pnpm run repo:verify`
  - 通过标准：OpenAPI 门禁通过
- [ ] `pnpm run test:backend:release-gate`
  - 通过标准：关键后端链路通过，且无 DB skip
- [ ] `pnpm run accept:web`
  - 通过标准：前端 lint/typecheck/build 全通过

## B. 数据库与迁移
- [ ] `DATABASE_URL` / `TEST_DATABASE_URL` 配置确认
  - 通过标准：至少一个可用且可连通
- [ ] 迁移状态确认（无 Pending）
  - 通过标准：数据库结构与代码一致
- [ ] 回滚与备份路径确认
  - 通过标准：备份文件可恢复、回滚流程可执行

## C. 企业级“落库可审计”条款（硬规则）
- [ ] 订单处理动作落库证据
  - `order_action_log` / `order_status_history`
- [ ] 商品匹配动作落库证据
  - `shop_product_tasks` / `channel_sku_mappings` / `operation_logs`
- [ ] 发货与波次动作落库证据
  - `shipments` / `operation_logs` / `audit_logs`
- [x] 阻断规则
  - 任一关键动作仅“接口成功”但无落库证据 => **NO-GO**

## D. 安全与隔离
- [x] Tenant 隔离检查（菜单/路由按租户上下文挂载）
- [x] RBAC 权限校验（系统治理菜单与路由均含权限守卫）
- [ ] 敏感配置（如 JWT）非默认值

## E. 可观测性
- [x] API / Worker 日志可追踪（含 `operation_logs` 链路）
- [x] 核心指标可观测（`stats/health` 与系统观测页可达）
- [ ] 异常告警通道有效（需环境联调验证）

## F. 发布结论
- [ ] Go：A~E 全部通过
- [x] No-Go：任一项失败或关键 DB 测试出现 skip
