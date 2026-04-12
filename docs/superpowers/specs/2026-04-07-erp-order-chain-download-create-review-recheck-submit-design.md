# 工业级电商 ERP 订单链路设计与落地（下载/新建/审核/复核/提交）

## 1. 目标

将订单主链路标准化为可市场发布的企业级流程：
- 下载订单
- 新建订单
- 审核订单
- 复核订单
- 提交订单

并在 API 层实现最小权限闭环、可追踪审计、可回归测试。

## 2. 链路定义（状态机）

### 2.1 下载订单（Order Download）
- 触发来源：运营人员手动触发（同步任务 `order_sync`）。
- 行为：写入/更新同步任务，生成运行记录，异步由 worker 拉取平台订单。
- 输出：`sync_job` 进入 `PENDING`，run 记录为 `REQUESTED`。

### 2.2 新建订单（Manual Create）
- 触发来源：人工补单/客服建单。
- 行为：按 `shop + platform_order_id` 建立订单投影并写入订单索引与明细。
- 约束：同平台同店铺 `platform_order_id` 不可重复。

### 2.3 审核订单（Review）
- 既有能力：`/review`。
- 核心校验：SKU 映射、物流信息、库存缺货规则。
- 结果：`review_status` 进入 `APPROVED/REJECTED/MANUAL_REVIEW`。

### 2.4 复核订单（Recheck）
- 新增能力：在已审核通过订单上进行提交前复核。
- 核心校验：
  - 订单已 `APPROVED`
  - 不处于冻结标记
  - SKU/物流/缺货规则再次满足
- 结果：不改变主状态，仅写审计和状态日志，形成“二次把关”证据。

### 2.5 提交订单（Submit）
- 新增能力：业务语义化提交接口（底层复用分配/生成发货单逻辑）。
- 前置条件：订单必须 `APPROVED`。
- 行为：调用履约分配逻辑，生成发货单，返回 `ALLOCATED/SHORTAGE` 结果。

## 3. API 清单

### 3.1 新增 API
- `POST /v1/tenants/:tenant_id/orders/download`
- `POST /v1/tenants/:tenant_id/oms/orders/download`
- `POST /v1/tenants/:tenant_id/orders/new`
- `POST /v1/tenants/:tenant_id/oms/orders/new`
- `POST /v1/tenants/:tenant_id/orders/:order_id/recheck`
- `POST /v1/tenants/:tenant_id/oms/orders/:order_id/recheck`
- `POST /v1/tenants/:tenant_id/orders/:order_id/submit`
- `POST /v1/tenants/:tenant_id/oms/orders/:order_id/submit`

### 3.2 兼容增强
- `POST /v1/tenants/:tenant_id/orders` 支持 `POST` 新建（与 `/orders/new` 等价）。
- `POST /v1/tenants/:tenant_id/oms/orders` 支持 `POST` 新建（与 `/oms/orders/new` 等价）。

## 4. 权限矩阵

| 动作 | 权限 | 说明 |
|---|---|---|
| 下载订单 | `orders.download` | 触发订单同步任务 |
| 新建订单 | `orders.create` | 人工建单 |
| 审核订单 | `orders.review` | 一审 |
| 复核订单 | `orders.recheck` | 二次复核 |
| 提交订单 | `orders.submit` + `shipping.write` + `inventory.write` | 提交后进入履约分配 |

权限别名映射策略（为了兼容现有权限体系）：
- `orders.download -> OMS.ORDER.WRITE`
- `orders.create -> OMS.ORDER.WRITE`
- `orders.recheck -> OMS.ORDER.REVIEW`
- `orders.submit -> OMS.ORDER.WRITE`

## 5. 企业级发布要求（本链路）

- 租户与店铺双层隔离校验必须先于业务执行。
- 每个关键节点必须有审计日志（下载触发、建单、复核、提交）。
- 复核与提交分离，提交前必须具备审核通过状态。
- 权限回归测试覆盖“缺少任一关键权限即 403”。

## 6. 本轮落地摘要

已完成：
- 新增下载/新建/复核/提交 API（OMS 与非 OMS 双路径）。
- 新增订单权限别名映射并在接口中启用。
- 新增权限回归测试，验证链路权限封闭性。

建议下一步：
- 增加新建订单与提交订单成功路径集成测试（含真实 DB schema）。
- 在 OpenAPI 文档中补充这组新接口字段约束与错误码。
- 在前端工作台新增链路看板（下载任务 -> 新建 -> 审核 -> 复核 -> 提交）。
