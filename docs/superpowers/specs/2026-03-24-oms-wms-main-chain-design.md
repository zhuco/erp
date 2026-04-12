# OMS-WMS 主链路一期设计

- 日期：2026-03-24
- 状态：设计已确认，待规格审阅
- 适用范围：`F:\erp`
- 一期目标主链路：`订单审核 -> 订单复核 -> 提交发货 -> 波次生成 -> 拣货 / 复核 / 打包 -> 出库交接`

## 1. 背景

当前项目已经具备 OMS 与 WMS 主链路的页面和接口骨架，正式页面至少包括：

- 订单审核页：`apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- 审单规则页：`apps/erp-web/src/pages/orders/OrderReviewRulesPage.tsx`
- 波次页：`apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- 拣货页：`apps/erp-web/src/pages/wms/WmsPickTasksPage.tsx`
- 打包复核页：`apps/erp-web/src/pages/wms/WmsPackPage.tsx`
- 发货页：`apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- 扫码出库确认页：`apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`

同时，项目已经存在对应的 API 入口：

- 订单主流程：`apps/erp-web/src/api/orders.ts`
- 审单规则：`apps/erp-web/src/api/orderReviewRules.ts`
- WMS 出库主链：`apps/erp-web/src/api/wms.ts`
- 仓配策略：`apps/erp-web/src/api/warehouseStrategies.ts`
- 仓库流程配置：`apps/erp-web/src/api/workflowConfigs.ts`
- 发货与回传补充接口：`apps/erp-web/src/api/shipments.ts`、`apps/erp-web/src/api/omsShipments.ts`

现状问题不在于“完全没有功能”，而在于以下结构性缺口：

1. 订单审核、订单复核、提交发货、实际交接发运在语义上仍有混用。
2. 异常处理仍然偏向主列表筛选视图，缺少真正的异常池闭环。
3. 波次、拣货、打包、交接虽然已有页面，但职责边界不清，容易继续堆叠为“大而全工作台”。
4. WMS 末端缺少正式交接闭环，当前“发货”动作仍偏弱语义。
5. 调试页与 demo 页偏多，影响系统收口与回归基座统一。

本设计的目标不是推翻现有页面，而是在保留现有骨架的前提下，重定义职责边界、统一状态机与阻断规则，让系统从“可演示”升级为“可控、可追责、可扩展”的高端电商 ERP 主履约链路。

## 2. 设计目标

### 2.1 一期业务目标

1. 打通 OMS 到 WMS 的主履约链路。
2. 明确“审核、复核、下发、交接”四个不同业务节点。
3. 建立订单异常池闭环，而不是继续依赖主列表筛选处理异常。
4. 建立从待波次池到交接完成的仓内执行主链。
5. 保证所有关键动作可阻断、可回退、可审计、可回归验证。

### 2.2 工程目标

1. 保留现有正式页面作为演进基座，避免全量推翻重做。
2. 删除不需要的调试页和 demo 页，保留唯一 grid 回归基座。
3. 将共享 grid 交互约束继续沉淀为跨页面统一规范。
4. 为后续实施计划提供足够清晰、无歧义的页面边界、状态定义和数据落点。

## 3. 非目标

本设计不包含以下范围：

1. 售后、采购、财务、物流资费与利润回写的二期联动。
2. 全量重构 OMS、WMS 或权限体系。
3. 一期内引入复杂路径优化、播种墙、自动装箱算法等高级优化能力。
4. 一期内替换现有 grid 主体技术路线。
5. 一期内完成所有历史调试逻辑与所有历史文档的全面清洗。

## 4. 方案比较与推荐

### 4.1 方案 A：在现有页面上继续堆功能

做法：

- 继续在订单审核页、波次页、拣货页、打包页、发货页上直接增加按钮、弹窗和条件逻辑。

优点：

- 改动最少，短期交付最快。

缺点：

- 页面职责继续发散。
- 异常池、交接批次、阻断规则会被塞进已有页面，后续维护成本持续变高。
- 容易继续形成“功能看起来都有，但语义与流程越来越混”的局面。

结论：

- 不推荐作为高端 ERP 主链方案。

### 4.2 方案 B：OMS 中台 + WMS 中台完整重拆

做法：

- 将 OMS 复核中台、WMS 执行中台全部拆成新页面、新状态机与新导航体系。

优点：

- 架构最清晰，长期扩展性最好。

缺点：

- 本轮范围过大，页面、权限、mock、测试、状态迁移都要重做。
- 超出一期主链收敛目标。

结论：

- 可作为长期路线，不适合本轮直接落地。

### 4.3 方案 C：主链路优先的“双中台轻重构”

做法：

- 保留现有正式页作为骨架。
- 重定义各页面职责边界。
- 只新增一期必须新增的支撑页。
- 统一状态机、阻断规则、权限和审计语义。

优点：

- 与当前项目阶段最匹配。
- 能最大化复用现有代码和 API。
- 能在不推翻现有页面的前提下建立真正可用的主履约链。

缺点：

- 需要先收敛边界，再实施，不适合边想边堆按钮。

结论：

- 推荐采用。

## 5. 一期范围

### 5.1 一期正式页

保留并重定义职责的正式页：

- `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`
- `apps/erp-web/src/pages/wms/WmsWavesPage.tsx`
- `apps/erp-web/src/pages/wms/WmsPickTasksPage.tsx`
- `apps/erp-web/src/pages/wms/WmsPackPage.tsx`
- `apps/erp-web/src/pages/wms/WmsShipPage.tsx`
- `apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`

### 5.2 一期必须新增页

1. 订单异常池页
2. 出库交接页

说明：

- `WmsShipPage` 可以先以重构方式演进为出库交接页；如果实施时发现旧页面负担过重，可新增新页并保留旧路由跳转兼容。

### 5.3 二期暂缓项

- 售后与出库主链深度联动
- 财务结算回写
- 物流资费优化
- 自动装箱
- 拣货路径优化可视化
- 播种墙 / 分区波次高级模式

## 6. 调试页治理

### 6.1 删除项

建议删除以下页面与其路由、菜单、测试依赖：

- `apps/erp-web/src/pages/system/ErpDataGridDebugPage.tsx`
- `apps/erp-web/src/pages/system/ErpDataGridReviewSandboxPage.tsx`
- `apps/erp-web/src/pages/demo/WorkbenchListDemoPage.tsx`

需要同步清理：

- `apps/erp-web/src/app/router.tsx`
- `apps/erp-web/src/app/layout/menu.tsx`
- `apps/erp-web/e2e/erp-data-grid-debug.spec.ts`
- 相关 CSS 引用和 import

### 6.2 保留项

保留唯一 grid 级回归基座：

- `apps/erp-web/src/pages/system/ErpDataGridNextDebugPage.tsx`

理由：

- mock 数据独立
- 无业务 API 强耦合
- 已有专属 E2E
- 可作为统一 grid 回归入口

### 6.3 正式业务页中的调试残留

`apps/erp-web/src/pages/orders/OrderReviewPage.tsx` 中的调试残留命名和调试面板应清理，但不属于独立调试页删除范围。

## 7. 统一业务语义

### 7.1 订单审核

含义：

- 规则引擎审单与人工初审，决定订单是否允许进入后续复核与履约。

责任域：

- OMS

### 7.2 订单复核

含义：

- 对异常单、风控单、地址异常、黑名单、高客单、无效商品等订单做人工复核闭环。

责任域：

- OMS 异常池

### 7.3 提交发货

含义：

- 从 OMS 正式下发到 WMS，生成履约需求、尝试预占库存、生成出库单。
- 该动作不代表货物已经发走。

责任域：

- OMS -> WMS 交界

### 7.4 出库交接

含义：

- 包裹完成拣货、复核、打包、称重后，正式交接给承运商或出车批次。
- 只有在此节点成功后，订单才能进入真实已发货。

责任域：

- WMS / TMS 交界

## 8. 一期状态模型

一期采用多状态轴，而不是将所有业务意义压缩到一个字段中。

### 8.1 审核状态 `audit_status`

- `PENDING_AUDIT`
- `AUTO_PASSED`
- `REVIEW_REQUIRED`
- `UNDER_REVIEW`
- `REVIEW_PASSED`
- `REVIEW_REJECTED`
- `ON_HOLD`

说明：

- `AUTO_PASSED` 表示系统自动审核通过，业务语义等价于“已允许进入履约下发”。
- `REVIEW_PASSED` 表示经人工复核后审核通过。
- 一期不强制把两者合并为同一个状态值，但在阻断规则与页面动作上，二者都视为“已审核通过”。

### 8.2 异常状态 `exception_status`

- `NONE`
- `OPEN`
- `ASSIGNED`
- `PROCESSING`
- `ESCALATED`
- `RESOLVED`
- `CLOSED`

### 8.3 履约主状态 `fulfillment_status`

- `NOT_SUBMITTED`
- `WAVE_PENDING`
- `WAVED`
- `PICKING`
- `PACKING`
- `HANDOVER_PENDING`
- `HANDOVERED`
- `SHIPPED`
- `CANCELLED`
- `ROLLBACK_PENDING`

说明：

- `fulfillment_status` 是一期的主状态汇总字段，由后端在关键业务事件发生时显式写入。
- `wave_status / pick_status / pack_status / handover_status` 是执行细节状态，不替代 `fulfillment_status`。

### 8.4 预占状态 `reserve_status`

- `NOT_RESERVED`
- `RESERVING`
- `RESERVED`
- `RESERVE_FAILED`
- `RELEASED`

### 8.5 仓内执行子状态

`wave_status`

- `NONE`
- `CREATED`
- `RELEASED`
- `CANCELLED`

`pick_status`

- `NONE`
- `PICKING`
- `PICKED`
- `SHORT_PICK`
- `EXCEPTION`

`pack_status`

- `NONE`
- `PACKING`
- `CHECKED`
- `WEIGHED`
- `PACKED`
- `EXCEPTION`

`handover_status`

- `NONE`
- `READY`
- `HANDOVERED`
- `PUSHED`
- `FAILED`

### 8.6 主状态写入策略

为避免实施阶段在状态落库与前端展示上出现分叉，一期固定采用“主状态显式写入，子状态并行记录”的策略。

最小写入规则如下：

| 业务事件 | 主状态写入 `fulfillment_status` | 子状态变化 |
| --- | --- | --- |
| 提交发货且预占成功 | `WAVE_PENDING` | `reserve_status -> RESERVED` |
| 提交发货但预占失败 | 保持 `NOT_SUBMITTED` | `reserve_status -> RESERVE_FAILED` |
| 波次创建 / 下发成功 | `WAVED` | `wave_status -> CREATED / RELEASED` |
| 任一拣货任务开始 | `PICKING` | `pick_status -> PICKING` |
| 任一打包 / 复核 / 称重动作开始 | `PACKING` | `pack_status -> PACKING / CHECKED / WEIGHED` |
| 全部包裹封箱完成，进入待交接 | `HANDOVER_PENDING` | `pack_status -> PACKED` |
| 交接确认成功 | `HANDOVERED` | `handover_status -> HANDOVERED` |
| 物流推送成功或承运商揽收确认成功 | `SHIPPED` | `handover_status -> PUSHED` |

说明：

- 一期不采用“运行时由子状态实时派生主状态”的策略，避免接口和前端同时承担状态计算责任。
- 一期允许 `handover_status = HANDOVERED` 时，`fulfillment_status` 先处于 `HANDOVERED`，后续在物流确认后再写入 `SHIPPED`。

### 8.7 页面字段映射口径

为避免实施阶段在“改前端展示”还是“补后端字段”之间摇摆，一期统一采用以下映射：

| 页面展示字段 | 一期口径 | 是否需要新增后端字段 | 数据归属 |
| --- | --- | --- | --- |
| 审核状态 | 直接映射 `audit_status` | 否 | 订单汇总 |
| 复核状态 | 派生展示字段，不单独落状态轴 | 否 | 订单汇总派生 |
| 异常状态 | 直接映射 `exception_status` | 否 | 订单汇总 |
| 预占状态 | 直接映射 `reserve_status` | 是 | 订单汇总投影 |
| 下发状态 | 派生自 `fulfillment_status` | 否 | 订单汇总派生 |
| 波次状态 | 直接映射 `wave_status` | 否 | 波次 / 出库单 |
| 拣货状态 | 直接映射 `pick_status` | 否 | 拣货任务 / 出库单 |
| 打包状态 | 直接映射 `pack_status` | 否 | 包裹 / 出库单 |
| 交接状态 | 直接映射 `handover_status` | 否 | 包裹 / 交接批次 |

派生字段定义：

- `复核状态`
  - `无需复核`：`audit_status = AUTO_PASSED`
  - `待复核`：`audit_status in REVIEW_REQUIRED / UNDER_REVIEW`
  - `复核通过`：`audit_status = REVIEW_PASSED`
  - `复核驳回`：`audit_status = REVIEW_REJECTED`
  - `已挂起`：`audit_status = ON_HOLD`
- `下发状态`
  - `未下发`：`fulfillment_status = NOT_SUBMITTED`
  - `已下发待波次`：`fulfillment_status = WAVE_PENDING`
  - `已进入波次及仓内执行`：`fulfillment_status in WAVED / PICKING / PACKING / HANDOVER_PENDING / HANDOVERED / SHIPPED`
  - `已取消 / 待回滚`：`fulfillment_status in CANCELLED / ROLLBACK_PENDING`

## 9. 主链路标准流转

### 9.1 订单进入审核

1. 新订单进入 `PENDING_AUDIT`
2. 未命中规则：`audit_status -> AUTO_PASSED`
3. 命中规则：`audit_status -> REVIEW_REQUIRED`，同时 `exception_status -> OPEN`

补充说明：

- `AUTO_PASSED` 订单可直接进入“提交发货”前置校验，不需要再进入人工复核。

### 9.2 主审核页处理

1. 正常单允许审核通过或直接处于 `AUTO_PASSED`
2. 异常单不允许在主审核页中“假通过”
3. 异常单必须进入异常池闭环

### 9.3 异常池复核

1. 通过：`audit_status -> REVIEW_PASSED`，`exception_status -> RESOLVED`
2. 驳回：`audit_status -> REVIEW_REJECTED`
3. 挂起：`audit_status -> ON_HOLD`
4. 转派 / 升级：`exception_status -> ASSIGNED / ESCALATED`

### 9.3.1 异常池动作最小状态迁移表

一期异常池仅处理 `fulfillment_status = NOT_SUBMITTED` 的订单。

说明：

- 已提交发货并已生成履约单据的订单，不在一期异常池回退范围内。
- 一期支持仓内异常记录，但这些异常只记录在 WMS 执行异常域，不支持回退到 OMS 待审核，也不支持撤销已生成履约单据。

| 动作 | 允许前置状态 | 状态变化 | 阻断结果 |
| --- | --- | --- | --- |
| 领取 | `audit_status = REVIEW_REQUIRED` 且 `exception_status in OPEN / ASSIGNED` | `audit_status -> UNDER_REVIEW`；`exception_status -> PROCESSING` | 保持阻断 |
| 分派 / 转派 | `exception_status in OPEN / ASSIGNED / PROCESSING / ESCALATED` | `exception_status -> ASSIGNED`；`audit_status` 保持 `REVIEW_REQUIRED / UNDER_REVIEW` | 保持阻断 |
| 升级 | `exception_status in ASSIGNED / PROCESSING` | `exception_status -> ESCALATED`；`audit_status -> UNDER_REVIEW` | 保持阻断 |
| 复核通过 | `audit_status in REVIEW_REQUIRED / UNDER_REVIEW / ON_HOLD` | `audit_status -> REVIEW_PASSED`；`exception_status -> RESOLVED` | 解除阻断 |
| 退回待补充 | `audit_status in REVIEW_REQUIRED / UNDER_REVIEW / ON_HOLD` | `audit_status -> ON_HOLD`；`exception_status -> OPEN` | 保持阻断 |
| 驳回关闭 | `audit_status in REVIEW_REQUIRED / UNDER_REVIEW / ON_HOLD` | `audit_status -> REVIEW_REJECTED`；`exception_status -> CLOSED` | 保持阻断，且不得提交发货 |
| 回退到待审核 | `audit_status in REVIEW_REQUIRED / UNDER_REVIEW / ON_HOLD / REVIEW_REJECTED` | `audit_status -> PENDING_AUDIT`；`exception_status -> CLOSED` | 当前异常工单关闭，后续若再次命中规则则重新开单 |

### 9.4 提交发货

前置条件：

- `audit_status in AUTO_PASSED / REVIEW_PASSED`
- `exception_status in NONE / RESOLVED / CLOSED`
- 库存预检通过

动作：

- 创建出库单
- 执行预占库存
- 生成履约任务

结果：

- 成功：`reserve_status -> RESERVED`；`fulfillment_status -> WAVE_PENDING`
- 失败：`reserve_status -> RESERVE_FAILED`；`fulfillment_status` 保持 `NOT_SUBMITTED`

### 9.5 波次生成

1. 波次创建成功：`wave_status -> CREATED`
2. 波次下发成功：`wave_status -> RELEASED`
3. 同时履约状态推进：`fulfillment_status -> WAVED`

### 9.6 拣货

1. 开始拣货：`pick_status -> PICKING`
2. 拣货完成：`pick_status -> PICKED`
3. 短拣 / 差异：`pick_status -> SHORT_PICK / EXCEPTION`
4. 首个拣货动作落地后：`fulfillment_status -> PICKING`

### 9.7 复核 / 打包 / 称重

1. 打包开始：`pack_status -> PACKING`
2. 复核通过：`pack_status -> CHECKED`
3. 称重完成：`pack_status -> WEIGHED`
4. 封箱完成：`pack_status -> PACKED`
5. 任一打包工作开始后：`fulfillment_status -> PACKING`
6. 全部包裹封箱完成且未交接：`fulfillment_status -> HANDOVER_PENDING`

### 9.8 出库交接

1. 形成交接批次、交接单、交接人、承运商信息
2. 交接成功：`handover_status -> HANDOVERED`
3. 订单履约推进：`fulfillment_status -> HANDOVERED`
4. 若物流推送或承运商确认成功：`fulfillment_status -> SHIPPED`

## 10. 阻断规则

以下规则在一期内必须严格执行：

1. 只有 `audit_status in AUTO_PASSED / REVIEW_PASSED` 的订单允许提交发货。
2. `OPEN / ASSIGNED / PROCESSING / ESCALATED` 的异常单禁止进入波次。
3. `reserve_status != RESERVED` 的订单禁止建波次。
4. 未完成拣货的出库单禁止打包。
5. 复核不通过的包裹禁止称重和交接。
6. 未称重完成的包裹禁止进入交接批次。
7. `HANDOVERED` 之前不得对订单显示真实已发货。
8. 波次取消一期仅允许在 `wave_status = CREATED` 且尚未开始拣货时执行；取消后单据回退到 `WAVE_PENDING`。
9. 短拣、复核差异、称重异常都必须落异常记录，不能只弹提示。
10. 一期不支持已提交发货订单回退到 OMS 待审核，也不支持撤销已生成履约单据。

## 11. 页面职责边界

### 11.1 订单审核页

页面文件：

- `apps/erp-web/src/pages/orders/OrderReviewPage.tsx`

职责：

- 待审核
- 待复核完成后可下发
- 已审核待提交发货

允许动作：

- 审核通过
- 挂起
- 强制审核
- 拆单
- 合单
- 改地址 / 改仓 / 改物流
- 打标签 / 备注
- 提交发货

不承担：

- 异常池分派与升级
- 仓内执行
- 交接发运确认

必须补充：

- 顶部看板：待审核数、异常待复核数、待提交发货数、挂起数、超时未处理数
- 提交发货确认层：订单数、商品件数、目标仓库、预占结果、阻断数、将生成出库单数
- 主表关键列：
  - 审核状态
  - 复核状态
  - 异常状态
  - 命中规则数
  - 预占状态
  - 下发状态
  - 目标仓库
  - 预计波次策略
  - 最后处理人
  - SLA 剩余时间

字段口径说明：

- 上述“复核状态 / 预占状态 / 下发状态”统一参考 `8.7 页面字段映射口径`。

### 11.2 订单异常池页

建议新增路由：

- `/orders/exceptions`

职责：

- 接住所有 `REVIEW_REQUIRED / OPEN / PROCESSING / ESCALATED` 的单
- 支持分派、领取、升级、退回、关闭、复核通过
- 支持按异常类型与责任人做 SLA 管理

一期边界：

- 仅处理 `fulfillment_status = NOT_SUBMITTED` 的异常订单。
- 已进入 WMS 履约阶段的异常，放入仓内异常处理，不在本页回退到待审核。

必须支持：

- 异常类型快捷视图
- 异常工单主表
- 详情区展示异常原因、命中规则、处理记录、责任人、升级记录

必须动作：

- 领取
- 分派
- 转派
- 升级
- 复核通过
- 退回待补充
- 驳回关闭
- 回退到待审核

必须字段：

- 异常类型
- 异常等级
- 当前责任人
- 首次命中时间
- 最近处理时间
- 超时等级
- 处理结果
- 是否阻断发货

### 11.2.1 SLA 最小口径

为防止一期范围失控，SLA 只采用最小可验收口径，不引入班次、责任人绩效、动态日历计算。

一期规则：

1. SLA 来源为“异常类型 -> 固定分钟阈值”的静态映射。
2. 责任人只参与筛选和看板归属，不参与 SLA 计算口径。
3. 一期只要求支持三种展示结果：
   - `NORMAL`
   - `DUE_SOON`
   - `OVERDUE`
4. 页面字段定义：
   - `sla_deadline_at`：按异常类型阈值计算出的截止时间
   - `sla_status`：当前 SLA 状态
   - `sla_remaining_minutes`：剩余分钟数，可小于 0
5. 看板口径：
   - `超时未处理数`：`sla_status = OVERDUE` 且 `exception_status in OPEN / ASSIGNED / PROCESSING / ESCALATED`

建议默认阈值：

- 高优先级异常：15 分钟
- 中优先级异常：30 分钟
- 低优先级异常：120 分钟

### 11.3 波次页

页面文件：

- `apps/erp-web/src/pages/wms/WmsWavesPage.tsx`

建议演进为双态页面：

- `待波次订单池`
- `已生成波次`

`待波次订单池` 负责：

- 查看已提交发货、待建波次的订单 / 出库单
- 批量建波次
- 自动试算波次
- 查看阻断原因

`已生成波次` 负责：

- 查看波次
- 查看波次详情
- 下发波次
- 取消波次
- 打印拣货单

一期波次规则字段分两类：

- 现有能力复用或轻改：
  - 仓库
  - 承运商
  - 订单数上限
  - 商品件数上限
  - 截止时间
- 一期必须新增：
  - 店铺 / 平台
  - 时效等级
  - 是否允许混承运商
  - 是否允许跨区拣货
  - 优先级

波次详情至少展示：

- 波次号
- 仓库
- 出库单数
- 订单数
- 件数
- 预估拣货区
- 承运商
- 当前状态
- 异常单数
- 释放时间
- 下发人

波次取消边界：

- 一期仅允许取消 `wave_status = CREATED` 且尚未开始拣货的波次。
- `RELEASED` 后不支持取消，避免一期引入任务撤回与补偿复杂度。

### 11.4 拣货页

页面文件：

- `apps/erp-web/src/pages/wms/WmsPickTasksPage.tsx`

职责：

- 拣货任务领取
- 拣货开始
- 拣货确认
- 短拣登记
- 容器绑定
- 改派

必须字段：

- 波次号
- 任务号
- 库区
- 库位
- SKU
- 应拣数量
- 已拣数量
- 短拣数量
- 容器
- 拣货人
- 任务状态

必须动作：

- 领取任务
- 开始拣货
- 确认拣货
- 登记短拣
- 转异常
- 改派
- 绑定容器
- 完成任务

必须补充：

- 短拣原因
- 缺货原因
- 补货建议
- 任务优先级
- 最晚完成时间
- 改派原因

说明：

- 一期支持仓内异常记录，但不支持把仓内异常回退为 OMS 审核异常。

### 11.5 打包 / 复核 / 称重页

页面文件：

- `apps/erp-web/src/pages/wms/WmsPackPage.tsx`

职责：

- 打包
- 复核
- 称重
- 包裹生成
- 包材记录

推荐工作站布局：

- 左侧：待处理队列
- 中间：扫描与操作区
- 右侧：包裹明细、商品明细、差异记录、称重结果

必须动作：

- 开始打包
- 复核通过
- 复核异常
- 称重
- 新增包裹
- 拆包 / 合包
- 确认封箱

必须字段：

- 出库单号
- 波次号
- 包裹号
- SKU / 应发数量 / 已打包数量
- 复核结果
- 重量
- 包材
- 操作人
- 打包时间
- 异常原因

说明：

- 一期支持复核差异、称重异常的仓内异常记录，但不支持撤销已生成履约单据。

### 11.6 出库交接页

建议演进自：

- `apps/erp-web/src/pages/wms/WmsShipPage.tsx`

长期建议路由：

- `/wms/handover`

职责：

- 创建交接批次
- 包裹 / 出库单加入交接批次
- 确认交接
- 打印交接清单
- 物流回传重试

必须展示：

- 承运商
- 交接批次号
- 包裹数
- 运单数
- 司机 / 揽收人
- 笼车 / 车次 / 批次号
- 交接时间
- 交接人
- 回执状态

必须动作：

- 创建交接批次
- 加入交接批次
- 移出交接批次
- 确认交接
- 回滚交接
- 打印交接清单
- 推送物流状态
- 重试回传

### 11.7 扫码出库确认页

页面文件：

- `apps/erp-web/src/pages/wms/WmsOutboundConfirmPage.tsx`

职责限定为终端快操作台：

- 扫单号
- 扫波次号
- 扫包裹号
- 快速确认交接结果
- 快速回显异常

不承担：

- 规则维护
- 批量策略
- 审计主视图
- 波次编排

## 12. 统一 Grid 交互约束

以下共享约束必须继续沉淀为跨页面统一交互规范，适用于订单审核、订单列表、异常池、待波次池等主表：

1. `选中` 仅表示当前活动行。
2. `钩选` 才表示批量操作集合。
3. `Ctrl / Meta` 行点击自动钩选。
4. `Shift` 以当前活动行 / 上一次选中行为锚点，对区间逐行反选钩选状态。
5. `Space` 切换当前活动行钩选状态。
6. 左上角支持当前可见行全选 / 反选。
7. 不允许不同页面再各自定义另一套多选语义。

## 13. 权限、开关与数据落点约束

### 13.1 权限方向

一期设计按动作粒度收口，建议至少向以下权限模型靠拢：

- `orders.review.read`
- `orders.review.write`
- `orders.exception.assign`
- `orders.exception.resolve`
- `orders.fulfillment.submit`
- `wms.wave.read`
- `wms.wave.write`
- `wms.pick.execute`
- `wms.pack.execute`
- `wms.handover.execute`
- `wms.handover.rollback`
- `shipping.push.retry`

说明：

- 一期不要求一次性重构完全部权限代码，但页面动作与设计必须向这一粒度收口。

### 13.2 Feature Flag 方向

一期真正保留并落地的开关建议为：

- `wms.enable_wave`
- `wms.enable_outbound_inspection`
- `wms.enable_pack_performance`
- `wms.enable_shipment_cancel`
- `flows.pick_recommend`

规则：

1. 未落地到页面和接口逻辑的 flag 不应只停留在枚举定义。
2. flag 关闭时要同时体现为页面可见能力、按钮可用态和接口阻断，而不只是隐藏入口。

### 13.3 一期数据落点与接口清单

为保证实施计划能直接拆数据库、接口与前端任务，一期采用以下最小落点：

#### 13.3.1 数据落点

- 订单汇总 / 订单投影：
  - `audit_status`
  - `exception_status`
  - `fulfillment_status`
  - `reserve_status`
- 异常工单：
  - `exception_type`
  - `exception_level`
  - `owner_id`
  - `sla_deadline_at`
  - `sla_status`
  - `result`
- 波次：
  - `wave_status`
  - 规则摘要字段
- WMS 执行：
  - `pick_status`
  - `pack_status`
  - `handover_status`
- 审计日志：
  - `state_before`
  - `state_after`

说明：

- `reserve_status` 一期落在订单汇总投影上，作为 OMS 主表和待波次池的展示字段。
- 预占明细和库存预占来源仍以履约单据 / 预占记录为准，但前台主表使用订单维度聚合口径。

#### 13.3.2 一期最小接口清单

OMS：

- 订单审核列表 / 详情：返回 `audit_status / exception_status / fulfillment_status / reserve_status`
- 异常池列表 / 详情 / 领取 / 分派 / 升级 / 通过 / 挂起 / 驳回 / 回退
- 提交发货：返回出库单结果、预占结果、失败原因

WMS：

- 待波次池列表
- 波次创建 / 下发 / 取消 / 详情
- 拣货确认 / 短拣登记
- 打包 / 复核 / 称重
- 交接批次创建 / 加入 / 确认 / 回滚

打印与回传：

- 波次打印
- 交接清单打印
- 物流推送重试

## 14. Mock、审计与验证约束

### 14.1 Mock 约束

一期必须补齐主链路 mock 演练能力：

- 审单规则 mock 持久化
- 异常池 mock 数据
- 提交发货后的待波次流转
- 波次 -> 拣货 -> 打包 -> 交接 的 mock 状态推进
- 短拣、差异、交接失败、回滚等异常场景

说明：

- 这里的“回滚”仅指交接失败重试、交接撤销等仓内执行层回滚。
- 不包含已下发订单回退到 OMS 审核态或撤销履约单据。

### 14.2 审计约束

以下动作必须落审计记录：

- 审核通过
- 强审
- 挂起
- 异常分派 / 升级 / 关闭
- 提交发货
- 波次创建 / 取消 / 下发
- 拣货确认 / 短拣
- 复核异常
- 称重确认
- 交接确认 / 回滚
- 物流回传重试

### 14.2.1 审计状态快照口径

由于一期采用多状态轴模型，审计中的“前状态 / 后状态”统一定义为状态快照，而不是单一字段。

建议快照结构：

- `audit_status`
- `exception_status`
- `fulfillment_status`
- `reserve_status`
- `wave_status`
- `pick_status`
- `pack_status`
- `handover_status`

规则：

1. 所有主链关键动作都必须记录 `state_before` 与 `state_after` 快照。
2. 若某次动作只影响其中一个状态轴，其余状态轴仍需按当前值一并写入快照。
3. 这样可以保证后续审计追责、状态回放和异常定位具备统一口径。

每条审计至少应携带：

- 单据类型
- 单据 ID
- 动作
- 前状态快照
- 后状态快照
- 操作人
- 操作时间
- 原因 / 备注
- 来源页面 / 终端

### 14.3 体验与性能约束

1. 主表必须基于统一 grid 交互约束。
2. 批量动作必须基于钩选集合，而不是当前活动行。
3. 批量操作结果必须给出成功数、失败数、阻断数摘要。
4. 核心页应支持键盘优先操作。
5. OMS / WMS 主工作台颜色风格必须与系统菜单主题一致。

## 15. 分阶段落地顺序

### 阶段 1：结构收口

- 删除不需要的调试页 / demo 页
- 清理对应路由、菜单、旧 E2E
- 清理正式业务页中的调试残留

### 阶段 2：OMS 侧收口

- 重构订单审核页职责
- 新增订单异常池页
- 打通审核、异常流转、提交发货

### 阶段 3：波次中台

- 重构波次页为待波次池 + 波次页双态结构
- 补齐波次规则维护
- 打通提交发货到波次生成闭环

### 阶段 4：仓内执行

- 增强拣货页
- 增强打包 / 复核 / 称重页
- 打通短拣、差异、包裹维度记录

### 阶段 5：交接闭环

- 将发货页升级为出库交接页
- 扫码确认页收口为终端快操作台
- 打通交接批次、交接清单、回滚与回传

### 阶段 6：回归与验收

- 单元测试：状态机、阻断规则、共享选择约束
- E2E：审核 -> 异常池 -> 提交发货 -> 波次 -> 拣货 -> 打包 -> 交接

## 16. 一期验收标准

### 16.1 业务验收

1. 异常单必须进入独立异常池闭环。
2. 只有 `AUTO_PASSED` 或 `REVIEW_PASSED` 的订单才能提交发货。
3. 提交发货成功后，订单必须进入待波次池，并带出 `reserve_status = RESERVED`。
4. 只有预占成功单据才能生成波次。
5. 短拣、复核差异、称重异常都必须形成仓内异常记录。
6. 只有交接成功后，订单才显示真实已发货。

### 16.2 页面验收

1. 各正式页职责边界清晰，无明显跨域动作堆叠。
2. OMS 与 WMS 页面均遵守统一 grid 交互约束。
3. 调试页收口完成，仅保留 `ErpDataGridNext` 调试页。

### 16.3 工程验收

1. mock 模式可跑通主链路与关键异常分支。
2. 主链路关键状态迁移有单测或集成校验。
3. 主链路至少具备端到端回归用例。

## 17. 风险与注意事项

1. 本次规格交付基于当前工作区运行；由于当前工作区未检测到 git 元数据，本轮只能以规格文件形式交付，后续实施计划需先确认真实仓库路径或补丁提交流程。
2. 现有部分页面已较重，实施时若发现重构成本过高，应优先新增薄页面并做兼容跳转，而不是继续在旧页面硬堆。
3. 一期必须先稳主链语义，避免同时引入过多二期能力导致重新失焦。
4. 若权限重构无法一步到位，应优先保证页面动作阻断和后端接口阻断一致。

## 18. 结论

一期采用“主链路优先的双中台轻重构”方案：

- OMS 侧形成：订单审核页 + 订单异常池
- WMS 侧形成：待波次池 / 波次页 + 拣货页 + 打包复核页 + 出库交接页
- 终端侧保留：扫码快操作台

该方案能够最大化复用当前页面和 API 底座，同时建立高端电商 ERP 主链路所必需的状态边界、阻断规则、交接闭环和审计能力，为下一步实施计划提供稳定基础。
