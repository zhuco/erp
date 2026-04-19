# 订单审核页工业化性能约束

本文档定义订单审核页（`/orders/review`）的本地运行性能门槛，目标是保持工业级 ERP 的高频操作响应。

## 目标分层

1. 视觉响应（Visual Feedback）
- 定义：点击左侧过滤节点到节点选中态更新。
- 目标：P95 <= 280ms。

2. 数据稳定（Data Settled）
- 定义：点击左侧过滤节点到主表平台列完成过滤一致。
- 目标：P95 <= 650ms（本地 mock）。

3. Tab 切换
- 定义：左侧 `过滤/筛选` 切换到可交互状态。
- 目标：P95 <= 300ms。

## 自动化约束

已提供性能预算用例：
- `apps/erp-web/e2e/order-review-performance-budget.spec.ts`

## 请求策略约束（第二阶段）

1. 左侧树节点点击后，先走本地即时过滤，必须优先反馈选中态与主表可见结果。
2. 左侧树过滤不直接驱动服务端过滤请求，服务端查询仅由搜索词、筛选表单、分页等条件触发。
3. 主表默认关闭行动画，避免交互阶段额外动画开销拖慢 P95。

本地运行：

```bash
pnpm --dir apps/erp-web test:e2e:perf
```

## 可调预算（环境变量）

1. `PW_PERF_VISUAL_BUDGET_MS`（默认 280）
2. `PW_PERF_SETTLE_BUDGET_MS`（默认 650）
3. `PW_PERF_TAB_BUDGET_MS`（默认 300）

示例：

```bash
$env:PW_PERF_VISUAL_BUDGET_MS=240
$env:PW_PERF_SETTLE_BUDGET_MS=550
$env:PW_PERF_TAB_BUDGET_MS=260
pnpm --dir apps/erp-web test:e2e:perf
```

## 建议的工业化流程

1. 每次改动订单审核页交互后，本地必须跑 `test:e2e:perf`。
2. 发布前在 CI 执行同一用例，预算超线则阻断合入。
3. 当预算连续 2 周明显低于阈值，可下调阈值，持续收紧。
