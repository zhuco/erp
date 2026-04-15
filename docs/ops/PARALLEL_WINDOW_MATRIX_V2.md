# 10 窗口并行任务矩阵（V2）

更新时间：2026-04-14  
适配基线提交：`b50c787`（合并至 `86795f1`）

## 1. 全局规则

1. 固定 10 个业务窗口，不额外设“开发窗口外编外角色”。
2. 每窗口固定 3 个子任务槽位：`T1`（进行中）、`T2`（待切换）、`T3`（待验证）。
3. 一个子任务一个提交，禁止混提。
4. 子任务提交前必须执行：
   - `node scripts/ops/validate_task_boundary.mjs --task <task-card> --mode all`
5. 涉及 OpenAPI 改动时必须附加执行：
   - `pnpm run openapi:bundle`
   - `pnpm run repo:verify:openapi`
6. 每日收工前生成评分卡：
   - `node scripts/ops/generate_parallel_scorecard.mjs --input <json> --output <md>`

## 2. 共享锁文件（V2）

共享锁策略定义在：
- `docs/ops/config/PARALLEL_SHARED_LOCKS_V2.json`

共享锁状态登记在：
- `docs/ops/state/PARALLEL_SHARED_LOCK_REGISTRY.json`

常用命令：

```bash
# 查看锁状态
node scripts/ops/register_shared_lock.mjs --mode status

# 申请锁（可重复 --pattern）
node scripts/ops/register_shared_lock.mjs --mode acquire \
  --task docs/ops/tasks/2026-04-14/example.md \
  --owner W10 \
  --pattern openapi/src/openapi.yaml

# 释放锁
node scripts/ops/register_shared_lock.mjs --mode release \
  --task docs/ops/tasks/2026-04-14/example.md \
  --owner W10
```

## 3. 10 窗口职责矩阵

| 窗口 | 主责 | 允许路径（关键） | 禁止路径（关键） |
| --- | --- | --- | --- |
| W1 | OMS 订单引入与建单 | `crates/erp-api/src/orders/intake.rs` `apps/erp-web/src/pages/orders/components/*Query*` `apps/erp-web/src/api/orders.ts` | `orders/review_control.rs` `orders/settlement.rs` |
| W2 | OMS 审核与异常处置 | `crates/erp-api/src/orders/review_control.rs` `crates/erp-api/src/orders/exception_workbench.rs` `apps/erp-web/src/pages/orders/review-native/**` `apps/erp-web/src/pages/orders/exceptions-native/**` | `orders/intake.rs` `orders/settlement.rs` |
| W3 | OMS 结算与对账联动 | `crates/erp-api/src/orders/settlement.rs` `crates/erp-api/src/finance_management.rs` `crates/erp-repo/src/finance*.rs` `apps/erp-web/src/pages/orders/OrderSettlementPage.tsx` | `orders/review_control.rs` |
| W4 | 售后逆向（退换补发） | `crates/erp-repo/src/after_sales*.rs` `crates/erp-api/tests/after_sales*.rs` `apps/erp-web/src/pages/afterSales/**` | `finance_*.rs` |
| W5 | WMS 出库波次执行 | `crates/erp-repo/src/wms*.rs` `crates/erp-api/tests/wms_*.rs` `apps/erp-web/src/pages/wms/**` | 库存财务成本模块 |
| W6 | 库存与调拨盘点 | `crates/erp-repo/src/inventory*.rs` `crates/erp-repo/src/stock_*.rs` `apps/erp-web/src/pages/inventory/**` | `wms` 出库作业细节 |
| W7 | 采购与入库 | `crates/erp-repo/src/procurement*.rs` `crates/erp-repo/src/inbounds*.rs` `apps/erp-web/src/pages/procurement/**` `apps/erp-web/src/pages/inbounds/**` | OMS 审核模块 |
| W8 | 商品/SKU/主数据 | `crates/erp-repo/src/product*.rs` `crates/erp-repo/src/sku*.rs` `apps/erp-web/src/pages/products/**` `apps/erp-web/src/pages/skus/**` | 渠道映射主链路 |
| W9 | 渠道店铺映射与同步 | `crates/erp-repo/src/channels.rs` `shops.rs` `mapping*.rs` `shop_products.rs` `crates/adapter-*/**` `apps/erp-web/src/pages/channels/**` | OpenAPI 锁文件 |
| W10 | 系统治理/可观测/发布门禁 | `crates/erp-repo/src/{rbac,permissions,audit_logs,operation_logs,reports}.rs` `apps/erp-web/src/pages/system/**` `scripts/openapi/**` | 业务域实现文件（W1-W9） |

机器可读矩阵：
- `docs/ops/config/PARALLEL_WINDOW_MATRIX_V2.json`

## 4. Codex 提示词包（直接可用）

### 4.1 主控窗口提示词

```text
你是并行主控。请按 10 窗口责任矩阵派发子任务，并严格执行共享锁机制。
必须做到：
1) 所有子任务先有任务卡再实现。
2) 所有提交先跑边界校验。
3) 触及 OpenAPI 的任务必须执行 openapi:bundle + repo:verify:openapi。
4) 输出当前批次可合并顺序与阻塞项，不做越界补丁实现。
```

### 4.2 业务窗口通用提示词

```text
你是窗口 {Wn}，只负责 {DOMAIN}。
你可并行处理 3 个子任务，但每个子任务独立提交。
严格遵守：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：
   node scripts/ops/validate_task_boundary.mjs --task {TASK_CARD} --mode all
3) 运行任务卡规定的最小测试。
4) 按格式交付：
   task | commit | changed_files | test_results | residual_risks
禁止顺手修改跨域文件，禁止触碰共享锁文件（除非主控授锁）。
```

### 4.3 W10（系统门禁窗口）附加提示词

```text
你负责并行质量门禁，不做业务实现。
请审查每个任务的：
1) 是否越界改动
2) 是否缺失必跑测试
3) 是否触发共享锁冲突
4) OpenAPI 改动是否经过 bundle 与 gate
只输出 findings + 合并建议顺序。
```
