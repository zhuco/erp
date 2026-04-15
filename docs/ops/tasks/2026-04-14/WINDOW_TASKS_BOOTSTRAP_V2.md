# 2026-04-14 并行窗口首批任务卡（全中文描述版）

> 说明：任务卡 front matter 键名保持英文（供脚本解析），键值与正文均为中文，方便直接分发。

## 字段中文说明
- `name`: 任务名称
- `goal`: 任务目标
- `background`: 业务背景
- `owner/window/domain/slot`: 责任窗口与槽位
- `allowed_files/disallowed_files`: 允许与禁止修改路径
- `shared_lock_targets`: 共享锁目标（命中锁文件时必填）
- `test_commands`: 最小必跑测试
- `submission_format`: 回执字段格式

## 启动顺序（建议）
1. 全窗口先启动 T1。
2. 同窗口 T2 在 T1 字段契约稳定后切换。
3. T3 仅在 T1/T2 有候选提交后启动收口。

## W1 · OMS 订单引入与建单
- T1（进行中）: [W1-T1 订单引入幂等与字段校验加固](docs/ops/tasks/2026-04-14/w1-t1-oms-intake-validation-hardening.md)
- T2（待切换）: [W1-T2 查询组件与建单 API 参数对齐](docs/ops/tasks/2026-04-14/w1-t2-query-and-api-parameter-alignment.md)
- T3（待验证）: [W1-T3 订单引入链路回归收口](docs/ops/tasks/2026-04-14/w1-t3-intake-chain-regression-pack.md)

### W1 可直接发送提示词
```text
你是窗口 W1，只负责 OMS 订单引入与建单。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w1-t1-oms-intake-validation-hardening.md
T2: docs/ops/tasks/2026-04-14/w1-t2-query-and-api-parameter-alignment.md
T3: docs/ops/tasks/2026-04-14/w1-t3-intake-chain-regression-pack.md
```

## W2 · OMS 审核与异常处置
- T1（进行中）: [W2-T1 审核动作守卫与状态约束加固](docs/ops/tasks/2026-04-14/w2-t1-review-action-guardrails.md)
- T2（待切换）: [W2-T2 异常工作台分流与 SLA 字段补齐](docs/ops/tasks/2026-04-14/w2-t2-exception-workbench-sla-routing.md)
- T3（待验证）: [W2-T3 审核与异常联调回归收口](docs/ops/tasks/2026-04-14/w2-t3-review-exception-regression-pack.md)

### W2 可直接发送提示词
```text
你是窗口 W2，只负责 OMS 审核与异常处置。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w2-t1-review-action-guardrails.md
T2: docs/ops/tasks/2026-04-14/w2-t2-exception-workbench-sla-routing.md
T3: docs/ops/tasks/2026-04-14/w2-t3-review-exception-regression-pack.md
```

## W3 · OMS 结算与对账联动
- T1（进行中）: [W3-T1 结算状态机与单据幂等约束](docs/ops/tasks/2026-04-14/w3-t1-settlement-state-machine-guard.md)
- T2（待切换）: [W3-T2 财务字段口径与对账映射对齐](docs/ops/tasks/2026-04-14/w3-t2-finance-reconcile-field-alignment.md)
- T3（待验证）: [W3-T3 结算对账回归与审计收口](docs/ops/tasks/2026-04-14/w3-t3-settlement-audit-regression-pack.md)

### W3 可直接发送提示词
```text
你是窗口 W3，只负责 OMS 结算与对账联动。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w3-t1-settlement-state-machine-guard.md
T2: docs/ops/tasks/2026-04-14/w3-t2-finance-reconcile-field-alignment.md
T3: docs/ops/tasks/2026-04-14/w3-t3-settlement-audit-regression-pack.md
```

## W4 · 售后逆向（退换补发）
- T1（进行中）: [W4-T1 售后退换补发状态流转加固](docs/ops/tasks/2026-04-14/w4-t1-after-sales-reverse-state-flow.md)
- T2（待切换）: [W4-T2 售后页面操作约束与可视化补齐](docs/ops/tasks/2026-04-14/w4-t2-after-sales-ui-operation-consistency.md)
- T3（待验证）: [W4-T3 售后主链路回归收口](docs/ops/tasks/2026-04-14/w4-t3-after-sales-regression-wrap-up.md)

### W4 可直接发送提示词
```text
你是窗口 W4，只负责 售后逆向（退换补发）。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w4-t1-after-sales-reverse-state-flow.md
T2: docs/ops/tasks/2026-04-14/w4-t2-after-sales-ui-operation-consistency.md
T3: docs/ops/tasks/2026-04-14/w4-t3-after-sales-regression-wrap-up.md
```

## W5 · WMS 出库波次执行
- T1（进行中）: [W5-T1 出库波次执行主链路加固](docs/ops/tasks/2026-04-14/w5-t1-wms-wave-execution-core.md)
- T2（待切换）: [W5-T2 波次作业看板与动作幂等保护](docs/ops/tasks/2026-04-14/w5-t2-wms-wave-ui-action-idempotency.md)
- T3（待验证）: [W5-T3 波次履约批次回归收口](docs/ops/tasks/2026-04-14/w5-t3-wms-wave-regression-pack.md)

### W5 可直接发送提示词
```text
你是窗口 W5，只负责 WMS 出库波次执行。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w5-t1-wms-wave-execution-core.md
T2: docs/ops/tasks/2026-04-14/w5-t2-wms-wave-ui-action-idempotency.md
T3: docs/ops/tasks/2026-04-14/w5-t3-wms-wave-regression-pack.md
```

## W6 · 库存与调拨盘点
- T1（进行中）: [W6-T1 库存调拨与冻结规则加固](docs/ops/tasks/2026-04-14/w6-t1-inventory-transfer-freeze-rules.md)
- T2（待切换）: [W6-T2 盘点差异闭环与调拨联动](docs/ops/tasks/2026-04-14/w6-t2-stock-count-difference-loop.md)
- T3（待验证）: [W6-T3 库存治理回归与风险收口](docs/ops/tasks/2026-04-14/w6-t3-inventory-governance-regression-pack.md)

### W6 可直接发送提示词
```text
你是窗口 W6，只负责 库存与调拨盘点。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w6-t1-inventory-transfer-freeze-rules.md
T2: docs/ops/tasks/2026-04-14/w6-t2-stock-count-difference-loop.md
T3: docs/ops/tasks/2026-04-14/w6-t3-inventory-governance-regression-pack.md
```

## W7 · 采购与入库
- T1（进行中）: [W7-T1 采购到货入库主链路打通](docs/ops/tasks/2026-04-14/w7-t1-procurement-inbound-main-chain.md)
- T2（待切换）: [W7-T2 入库异常拒收与补录流程加固](docs/ops/tasks/2026-04-14/w7-t2-inbound-exception-receive-flow.md)
- T3（待验证）: [W7-T3 采购入库回归与权限收口](docs/ops/tasks/2026-04-14/w7-t3-procurement-inbound-regression-pack.md)

### W7 可直接发送提示词
```text
你是窗口 W7，只负责 采购与入库。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w7-t1-procurement-inbound-main-chain.md
T2: docs/ops/tasks/2026-04-14/w7-t2-inbound-exception-receive-flow.md
T3: docs/ops/tasks/2026-04-14/w7-t3-procurement-inbound-regression-pack.md
```

## W8 · 商品 / SKU / 主数据
- T1（进行中）: [W8-T1 商品主数据变更流程加固](docs/ops/tasks/2026-04-14/w8-t1-product-master-data-change-flow.md)
- T2（待切换）: [W8-T2 SKU 条码与单位换算对齐](docs/ops/tasks/2026-04-14/w8-t2-sku-barcode-unit-alignment.md)
- T3（待验证）: [W8-T3 商品与 SKU 链路回归收口](docs/ops/tasks/2026-04-14/w8-t3-product-sku-regression-pack.md)

### W8 可直接发送提示词
```text
你是窗口 W8，只负责 商品 / SKU / 主数据。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w8-t1-product-master-data-change-flow.md
T2: docs/ops/tasks/2026-04-14/w8-t2-sku-barcode-unit-alignment.md
T3: docs/ops/tasks/2026-04-14/w8-t3-product-sku-regression-pack.md
```

## W9 · 渠道店铺映射与同步
- T1（进行中）: [W9-T1 渠道店铺映射规则与冲突检测](docs/ops/tasks/2026-04-14/w9-t1-channel-shop-mapping-rules.md)
- T2（待切换）: [W9-T2 渠道同步重试与错误分类](docs/ops/tasks/2026-04-14/w9-t2-channel-adapter-sync-retry.md)
- T3（待验证）: [W9-T3 渠道映射同步回归收口](docs/ops/tasks/2026-04-14/w9-t3-channel-mapping-regression-pack.md)

### W9 可直接发送提示词
```text
你是窗口 W9，只负责 渠道店铺映射与同步。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w9-t1-channel-shop-mapping-rules.md
T2: docs/ops/tasks/2026-04-14/w9-t2-channel-adapter-sync-retry.md
T3: docs/ops/tasks/2026-04-14/w9-t3-channel-mapping-regression-pack.md
```

## W10 · 系统治理 / 可观测 / 发布门禁
- T1（进行中）: [W10-T1 权限与审计基线巡检](docs/ops/tasks/2026-04-14/w10-t1-rbac-audit-baseline-check.md)
- T2（待切换）: [W10-T2 OpenAPI 门禁脚本与校验加固](docs/ops/tasks/2026-04-14/w10-t2-openapi-gate-hardening.md)
- T3（待验证）: [W10-T3 批次门禁 findings 与合并顺序建议](docs/ops/tasks/2026-04-14/w10-t3-merge-gate-findings-and-ordering.md)

### W10 可直接发送提示词
```text
你是窗口 W10，只负责 系统治理 / 可观测 / 发布门禁。
你可并行处理 3 个子任务，但每个子任务必须独立提交。
严格要求：
1) 仅修改任务卡 allowed_files。
2) 提交前必须运行：node scripts/ops/validate_task_boundary.mjs --task <TASK_CARD> --mode all
3) 执行任务卡内最小测试集。
4) 回执格式：task / commit / changed_files / test_results / residual_risks。
禁止跨域顺手改文件，禁止触碰共享锁文件（除非主控授锁）。
本窗口首批任务卡：
T1: docs/ops/tasks/2026-04-14/w10-t1-rbac-audit-baseline-check.md
T2: docs/ops/tasks/2026-04-14/w10-t2-openapi-gate-hardening.md
T3: docs/ops/tasks/2026-04-14/w10-t3-merge-gate-findings-and-ordering.md
```

## 主控窗口可直接发送提示词
```text
你是并行主控。请按 10 窗口责任矩阵派发子任务，并严格执行共享锁机制。
必须做到：
1) 所有子任务先有任务卡再实现。
2) 所有提交先跑边界校验。
3) 触及 OpenAPI 的任务必须执行 openapi:bundle + repo:verify:openapi。
4) 输出当前批次可合并顺序与阻塞项，不做越界补丁实现。
```
