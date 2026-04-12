# ERP流程文档（待开发完善）
> 面向电商ERP的核心流程文档，覆盖单据状态、异常处理、退回上级与操作影响。

## 文档清单
- `00-总体运行原理与主数据准备.md`
- `01-订单处理流程.md`
- `02-出入库流程.md`
- `03-仓储管理流程.md`
- `04-商品管理流程.md`
- `05-结算流程.md`
- `06-采购流程.md`
- `07-售后服务流程.md`
- `08-接口任务监控与自检.md`
- `09-权限审批与审计.md`
- `10-后端上线验收清单.md`
- `11-待开发清单.md`

## ERP运行逻辑总览
1. 主数据驱动：商品、仓库、供应商、客户为业务单据提供基础主数据。
2. 单据驱动：订单、采购、出入库、售后、结算是主业务链路的核心对象。
3. 状态机驱动：每张单据采用主状态+子状态，状态变化触发库存、财务、消息。
4. 任务与日志：审核、波次、拣货、对账等形成任务与日志，支持追溯与绩效统计。
5. 异常升级：异常记录→退回上级→复核/审批→回退或补偿，保证流程可控。

## 使用方式
- 每份流程文档包含：目标范围、关键角色、单据与状态、标准流程、异常处理、影响清单、待开发项。
- 系统实现可对照 `docs/spec` 与 `openapi` 的API与状态机文档补齐细节。

## 附录A：全局状态字典（建议）
### A1. 单据通用状态
| 状态 | 含义 | 主要动作/影响 |
| --- | --- | --- |
| 草稿 | 未提交审核 | 可编辑，不占用库存与资金 |
| 待审核 | 已提交待审批 | 生成审核任务，可设置预占策略 |
| 审核中 | 进入人工审核 | 冻结关键字段，记录审核日志 |
| 审核驳回 | 不满足规则 | 退回修改或关闭，释放预占 |
| 已审核 | 审核通过 | 允许分配/执行作业 |
| 执行中 | 作业进行中 | 触发库存锁定、任务与日志 |
| 待确认 | 等待外部回执 | 等待签收/回款/验收 |
| 已完成 | 业务结案 | 进入结算与报表统计 |
| 已关闭 | 作废/取消 | 终止流程并记录原因 |

### A2. 库存通用状态
| 状态 | 含义 | 触发场景 |
| --- | --- | --- |
| 可用 | 可售可分配 | 入库上架完成 |
| 预占 | 订单占用 | 订单审核/分配前 |
| 锁定 | 待出库 | 出库单已创建 |
| 冻结 | 暂不可用 | 盘点/异常处理 |
| 待检 | 等待质检 | 收货后未检 |
| 不良 | 不可销售 | 质检不合格 |
| 在途 | 调拨运输 | 调拨已发出 |

### A3. 状态流转原则
- 每次状态变更必须有触发动作、时间与责任人记录。
- 可回退状态需标记原因与审批人，避免“无记录回滚”。
- 关键动作影响库存、财务或绩效时，需产生联动日志。

## 附录B：角色与权限矩阵（建议）
### B1. 权限动作定义
- 建单/编辑：创建与修改业务单据
- 审核/退回：审批、驳回、退回上级处理
- 执行作业：拣货、上架、对账、退款等执行动作
- 作废/关闭：终止单据流程
- 查看/导出：查询、导出与报表查看

### B2. 通用权限矩阵
| 角色 | 建单/编辑 | 审核/退回 | 执行作业 | 作废/关闭 | 查看/导出 |
| --- | --- | --- | --- | --- | --- |
| 运营/客服 | ✅ | — | — | — | ✅ |
| 审核主管 | ✅ | ✅ | — | ✅ | ✅ |
| 仓库作业员 | — | — | ✅ | — | ✅ |
| 仓库主管 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 采购专员 | ✅ | — | ✅ | — | ✅ |
| 财务对账 | ✅ | ✅（结算） | ✅（对账/回款） | ✅ | ✅ |
| 系统管理员 | ✅ | ✅ | ✅ | ✅ | ✅ |

> 注：实际权限以岗位、数据范围与审批流配置为准。

## 附录C：异常分级与SLA（建议）
| 级别 | 典型场景 | 业务影响 | 首次响应 | 解决目标 | 升级路径 |
| --- | --- | --- | --- | --- | --- |
| P0 | 全站无法下单/发货 | 业务停摆 | 30分钟 | 4小时 | 技术负责人+业务负责人 |
| P1 | 核心链路阻塞 | 大面积延迟 | 2小时 | 1个工作日 | 模块负责人+主管 |
| P2 | 局部异常可绕过 | 效率下降 | 1个工作日 | 3个工作日 | 业务主管 |
| P3 | 低频体验问题 | 轻微影响 | 2个工作日 | 2周 | 产品负责人 |

## 附录D：MVP验收清单（建议）
- 订单：拉单/录入、审核通过/驳回、分仓、出库、发货、签收、完成。
- 库存：预占/锁定/扣减可追溯，库存台账可查询。
- 出入库：入库（收货/质检/上架）与出库（拣货/复核/打包）流程闭环。
- 采购：申请→订单→收货→入库，供应商信息可维护。
- 结算：账单导入、对账、差异记录、结算单与回款登记。
- 售后：退款/退货/换货至少一种路径可跑通。
- 权限：至少具备运营、仓库、采购、财务、主管的基础权限区分。
- 日志：关键状态变更、审核与作废动作有审计记录。

## 附录E：状态字典对照表（订单/仓储/采购/售后）
> 以“主状态”为主，子状态可在各模块按需扩展。

### E1. 订单状态对照表
| 单据 | 状态 | 含义 | 进入条件 | 主要影响 |
| --- | --- | --- | --- | --- |
| 销售订单 | 新建/草稿 | 订单已创建 | 拉单/手工录入 | 可编辑，不产生履约 |
| 销售订单 | 待审核 | 等待审核 | 提交审核 | 生成审核任务，可能预占库存 |
| 销售订单 | 审核驳回 | 不满足规则 | 审核拒绝 | 退回修改，释放预占 |
| 销售订单 | 异常挂起 | 需人工处理 | 命中异常规则 | 暂停流程，待放行/关闭 |
| 销售订单 | 待分配 | 等待分仓 | 审核通过 | 生成分仓/拆合单 |
| 销售订单 | 待出库 | 已生成出库单 | 分配完成 | 库存锁定 |
| 销售订单 | 出库中 | 拣货/打包 | 拣货任务下发 | 扣减可用库存 |
| 销售订单 | 已发货 | 物流已回传 | 出库完成 | 触发物流跟踪与结算 |
| 销售订单 | 已签收 | 物流签收 | 签收回传 | 进入结算/售后窗口 |
| 销售订单 | 已完成 | 订单结案 | 签收无售后 | 进入统计与归档 |
| 销售订单 | 已取消/关闭 | 订单终止 | 平台取消/手工关闭 | 释放库存并记录原因 |

### E2. 仓储状态对照表
| 单据/对象 | 状态 | 含义 | 进入条件 | 主要影响 |
| --- | --- | --- | --- | --- |
| 入库单 | 草稿 | 待提交 | 新建入库单 | 可编辑 |
| 入库单 | 待审核 | 等待审核 | 提交审核 | 生成审核记录 |
| 入库单 | 待收货 | 允许收货 | 审核通过 | 生成收货计划 |
| 入库单 | 收货中 | 收货执行 | 扫描到货 | 记录差异 |
| 入库单 | 待质检 | 等待质检 | 收货完成 | 形成质检任务 |
| 入库单 | 待上架 | 等待上架 | 质检通过 | 生成上架任务 |
| 入库单 | 上架中 | 上架执行 | 上架任务下发 | 占用库位 |
| 入库单 | 已完成 | 入库结案 | 上架完成 | 库存增加 |
| 入库单 | 已关闭 | 作废/取消 | 手工关闭 | 终止流程 |
| 出库单 | 草稿 | 待提交 | 新建出库单 | 可编辑 |
| 出库单 | 待审核 | 等待审核 | 提交审核 | 生成审核记录 |
| 出库单 | 待分配 | 等待波次 | 审核通过 | 生成拣货任务 |
| 出库单 | 待拣货 | 等待拣货 | 分配完成 | 冻结库存 |
| 出库单 | 拣货中 | 拣货执行 | 拣货开始 | 扣减可用库存 |
| 出库单 | 待复核 | 等待复核 | 拣货完成 | 复核差异记录 |
| 出库单 | 待打包 | 等待打包 | 复核通过 | 生成装箱 |
| 出库单 | 待发运 | 等待交运 | 打包完成 | 生成物流单 |
| 出库单 | 已出库 | 出库完成 | 交接物流 | 库存实扣 |
| 出库单 | 已关闭 | 取消/作废 | 手工关闭 | 终止流程 |
| 盘点单 | 草稿 | 待提交 | 新建盘点单 | 可编辑 |
| 盘点单 | 待盘点 | 等待盘点 | 审批通过 | 冻结库位 |
| 盘点单 | 盘点中 | 盘点执行 | 任务下发 | 记录差异 |
| 盘点单 | 待复核 | 等待复核 | 盘点完成 | 复核差异 |
| 盘点单 | 已完成 | 盘点结案 | 复核通过 | 生成调整单 |
| 盘点单 | 已关闭 | 取消盘点 | 手工关闭 | 终止流程 |
| 库存状态 | 可用/预占/锁定/冻结/待检/不良/在途 | 库存占用状态 | 作业触发 | 影响可售与分配 |

### E3. 采购状态对照表
| 单据 | 状态 | 含义 | 进入条件 | 主要影响 |
| --- | --- | --- | --- | --- |
| 采购申请 | 草稿 | 待提交 | 新建申请 | 可编辑 |
| 采购申请 | 待审批 | 等待审批 | 提交审批 | 生成审批任务 |
| 采购申请 | 已批准 | 允许下单 | 审批通过 | 可转采购订单 |
| 采购申请 | 已驳回 | 不通过 | 审批驳回 | 退回修改 |
| 采购申请 | 已关闭 | 终止申请 | 手工关闭 | 终止流程 |
| 采购订单 | 草稿 | 待提交 | 新建订单 | 可编辑 |
| 采购订单 | 待审批 | 等待审批 | 提交审批 | 生成审批任务 |
| 采购订单 | 已下单 | 已通知供应商 | 审批通过 | 形成到货计划 |
| 采购订单 | 部分到货 | 未完全收货 | 首次收货 | 记录缺口 |
| 采购订单 | 已到货 | 收货完成 | 全部收货 | 进入结案 |
| 采购订单 | 已关闭 | 取消/终止 | 手工关闭 | 终止流程 |
| 收货/质检 | 待收货 | 等待到货 | PO已下单 | 形成收货任务 |
| 收货/质检 | 收货中 | 收货执行 | 到货扫描 | 记录差异 |
| 收货/质检 | 待质检 | 等待质检 | 收货完成 | 形成质检任务 |
| 收货/质检 | 质检通过 | 可入库 | 质检通过 | 入库与成本入账 |
| 收货/质检 | 质检不通过 | 不合格 | 质检失败 | 退货或让步接收 |

### E4. 售后状态对照表
| 单据 | 状态 | 含义 | 进入条件 | 主要影响 |
| --- | --- | --- | --- | --- |
| 售后申请 | 已提交 | 客户提交 | 申请创建 | 生成工单 |
| 售后申请 | 待审核 | 等待审核 | 提交申请 | 生成审核任务 |
| 售后申请 | 已受理 | 受理通过 | 审核通过 | 进入退款/退货 |
| 售后申请 | 已驳回 | 不符合规则 | 审核拒绝 | 关闭或重提 |
| 售后申请 | 已关闭 | 售后结束 | 完成/取消 | 归档 |
| 退货单 | 待寄回 | 等待客户寄回 | 退货受理 | 进入运输 |
| 退货单 | 在途 | 退货运输 | 物流回传 | 待收货 |
| 退货单 | 待收货 | 等待仓库收货 | 物流到仓 | 收货完成 |
| 退货单 | 质检中 | 质检处理 | 收货完成 | 质检结果 |
| 退货单 | 质检通过 | 可入库 | 质检通过 | 库存回补 |
| 退货单 | 质检不通过 | 不可入库 | 质检失败 | 争议处理 |
| 退款单 | 待审核 | 等待审批 | 申请通过 | 审批 |
| 退款单 | 待退款 | 等待打款 | 审批通过 | 资金退回 |
| 退款单 | 已退款 | 退款完成 | 退款成功 | 结案 |
| 退款单 | 退款失败 | 退款异常 | 打款失败 | 重试/人工 |
| 换货/补发 | 待出库 | 等待出库 | 受理通过 | 生成出库 |
| 换货/补发 | 出库中 | 拣货打包 | 出库执行 | 物流发运 |
| 换货/补发 | 已发出 | 已发货 | 发运完成 | 签收确认 |
| 换货/补发 | 已完成 | 售后结案 | 签收完成 | 归档 |

## 附录F：状态码 + UI文案 + API字段名（建议）
> 建议使用全大写下划线作为状态码，UI文案可按业务语言调整。

### F1. 订单状态码（销售订单）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| order_status | DRAFT | 草稿 | 订单已创建未提交 |
| order_status | PENDING_REVIEW | 待审核 | 等待自动/人工审核 |
| order_status | REVIEW_REJECTED | 审核驳回 | 退回修改或关闭 |
| order_status | EXCEPTION_HOLD | 异常挂起 | 需要人工处理 |
| order_status | PENDING_ALLOCATE | 待分配 | 等待分仓/拆合单 |
| order_status | PENDING_OUTBOUND | 待出库 | 已生成出库单 |
| order_status | PICKING | 出库中 | 拣货/打包执行中 |
| order_status | SHIPPED | 已发货 | 物流已回传 |
| order_status | SIGNED | 已签收 | 物流签收完成 |
| order_status | COMPLETED | 已完成 | 订单结案 |
| order_status | CANCELLED | 已取消/关闭 | 终止订单 |

### F2. 仓储状态码
#### 入库单（inbound_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| inbound_status | DRAFT | 草稿 | 新建入库单 |
| inbound_status | PENDING_REVIEW | 待审核 | 提交审核 |
| inbound_status | PENDING_RECEIPT | 待收货 | 允许收货 |
| inbound_status | RECEIVING | 收货中 | 收货执行 |
| inbound_status | QC_PENDING | 待质检 | 收货完成 |
| inbound_status | PENDING_PUTAWAY | 待上架 | 质检通过 |
| inbound_status | PUTAWAYING | 上架中 | 上架执行 |
| inbound_status | COMPLETED | 已完成 | 入库结案 |
| inbound_status | CLOSED | 已关闭 | 取消/作废 |

#### 出库单（outbound_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| outbound_status | DRAFT | 草稿 | 新建出库单 |
| outbound_status | PENDING_REVIEW | 待审核 | 提交审核 |
| outbound_status | PENDING_ALLOCATE | 待分配 | 等待波次/分配 |
| outbound_status | PENDING_PICK | 待拣货 | 分配完成 |
| outbound_status | PICKING | 拣货中 | 拣货执行 |
| outbound_status | PENDING_VERIFY | 待复核 | 拣货完成 |
| outbound_status | PENDING_PACK | 待打包 | 复核通过 |
| outbound_status | PENDING_SHIP | 待发运 | 打包完成 |
| outbound_status | SHIPPED | 已出库 | 交接物流 |
| outbound_status | CLOSED | 已关闭 | 取消/作废 |

#### 库存状态（inventory_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| inventory_status | AVAILABLE | 可用 | 可售可分配 |
| inventory_status | RESERVED | 预占 | 订单占用 |
| inventory_status | LOCKED | 锁定 | 待出库 |
| inventory_status | FROZEN | 冻结 | 盘点/异常 |
| inventory_status | QC_PENDING | 待检 | 质检中 |
| inventory_status | DEFECTIVE | 不良 | 不可销售 |
| inventory_status | IN_TRANSIT | 在途 | 调拨运输中 |

#### 盘点单（stocktake_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| stocktake_status | DRAFT | 草稿 | 新建盘点单 |
| stocktake_status | PENDING_COUNT | 待盘点 | 审批通过 |
| stocktake_status | COUNTING | 盘点中 | 任务下发 |
| stocktake_status | PENDING_REVIEW | 待复核 | 盘点完成 |
| stocktake_status | COMPLETED | 已完成 | 复核通过 |
| stocktake_status | CLOSED | 已关闭 | 取消盘点 |

### F3. 采购状态码
#### 采购申请（purchase_request_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| purchase_request_status | DRAFT | 草稿 | 新建申请 |
| purchase_request_status | PENDING_APPROVAL | 待审批 | 提交审批 |
| purchase_request_status | APPROVED | 已批准 | 可转采购订单 |
| purchase_request_status | REJECTED | 已驳回 | 退回修改 |
| purchase_request_status | CLOSED | 已关闭 | 终止申请 |

#### 采购订单（purchase_order_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| purchase_order_status | DRAFT | 草稿 | 新建订单 |
| purchase_order_status | PENDING_APPROVAL | 待审批 | 提交审批 |
| purchase_order_status | PLACED | 已下单 | 已通知供应商 |
| purchase_order_status | PARTIALLY_RECEIVED | 部分到货 | 首次收货 |
| purchase_order_status | RECEIVED | 已到货 | 全部收货 |
| purchase_order_status | CLOSED | 已关闭 | 取消/终止 |

### F4. 售后状态码
#### 售后申请（after_sale_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| after_sale_status | SUBMITTED | 已提交 | 客户提交申请 |
| after_sale_status | PENDING_REVIEW | 待审核 | 等待审核 |
| after_sale_status | ACCEPTED | 已受理 | 进入处理流程 |
| after_sale_status | REJECTED | 已驳回 | 不符合规则 |
| after_sale_status | CLOSED | 已关闭 | 售后结案 |

#### 退货单（return_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| return_status | WAITING_RETURN | 待寄回 | 等待客户寄回 |
| return_status | IN_TRANSIT | 在途 | 退货运输中 |
| return_status | WAITING_RECEIPT | 待收货 | 物流到仓 |
| return_status | QC_CHECKING | 质检中 | 质检处理 |
| return_status | QC_PASSED | 质检通过 | 可入库 |
| return_status | QC_FAILED | 质检不通过 | 争议处理 |

#### 退款单（refund_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| refund_status | PENDING_APPROVAL | 待审核 | 等待审批 |
| refund_status | PENDING_REFUND | 待退款 | 等待打款 |
| refund_status | REFUNDED | 已退款 | 退款完成 |
| refund_status | REFUND_FAILED | 退款失败 | 重试/人工 |

#### 换货/补发（exchange_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| exchange_status | WAITING_OUTBOUND | 待出库 | 等待出库 |
| exchange_status | OUTBOUNDING | 出库中 | 拣货打包 |
| exchange_status | SHIPPED | 已发出 | 物流发运 |
| exchange_status | COMPLETED | 已完成 | 售后结案 |

## 附录G：子状态字段（建议）
> 主状态描述“阶段”，子状态描述“阶段内细节”，便于统计与异常处理。

### G1. 订单子状态
#### 审核子状态（review_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| review_status | NOT_REVIEWED | 未审核 | 未进入审核 |
| review_status | AUTO_APPROVED | 自动通过 | 规则自动放行 |
| review_status | MANUAL_REVIEW | 人工审核中 | 进入人工审核 |
| review_status | APPROVED | 审核通过 | 审核完成 |
| review_status | REJECTED | 审核驳回 | 审核拒绝 |
| review_status | ROLLED_BACK | 退回重审 | 退回上级 |

#### 履约子状态（fulfillment_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| fulfillment_status | NOT_ALLOCATED | 未分配 | 未分仓/未生成出库 |
| fulfillment_status | ALLOCATED | 已分配 | 已生成出库单 |
| fulfillment_status | WAVE_CREATED | 波次已生成 | 波次任务创建 |
| fulfillment_status | PICKING | 拣货中 | 拣货作业执行 |
| fulfillment_status | PACKING | 打包中 | 打包作业执行 |
| fulfillment_status | READY_TO_SHIP | 待发运 | 等待交接物流 |
| fulfillment_status | SHIPPED | 已发货 | 物流已回传 |
| fulfillment_status | SIGNED | 已签收 | 签收完成 |

#### 异常子状态（exception_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| exception_status | NONE | 无异常 | 正常 |
| exception_status | RISK | 风控异常 | 命中风控规则 |
| exception_status | ADDRESS_ERROR | 地址异常 | 地址不完整/不可达 |
| exception_status | OUT_OF_STOCK | 缺货 | 库存不足 |
| exception_status | PAYMENT_ISSUE | 支付异常 | 支付失败/争议 |
| exception_status | CUSTOMER_BLACKLIST | 黑名单 | 命中黑名单 |
| exception_status | MANUAL_HOLD | 人工挂起 | 人工暂停 |
| exception_status | RELEASED | 已放行 | 异常放行 |
| exception_status | CLOSED | 异常关闭 | 异常终止 |

#### 支付子状态（payment_status，可选）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| payment_status | UNPAID | 未支付 | 待支付 |
| payment_status | PAID | 已支付 | 支付完成 |
| payment_status | PART_REFUNDED | 部分退款 | 部分退款完成 |
| payment_status | REFUNDED | 已退款 | 全额退款 |
| payment_status | CHARGEBACK | 拒付/争议 | 平台拒付 |

### G2. 仓储子状态
#### 入库收货子状态（receipt_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| receipt_status | NOT_RECEIVED | 未收货 | 尚未到货 |
| receipt_status | PART_RECEIVED | 部分收货 | 部分到货 |
| receipt_status | RECEIVED | 已收货 | 收货完成 |

#### 入库质检子状态（inbound_qc_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| inbound_qc_status | NOT_REQUIRED | 免检 | 无需质检 |
| inbound_qc_status | PENDING | 待质检 | 等待质检 |
| inbound_qc_status | PASSED | 质检通过 | 可入库 |
| inbound_qc_status | FAILED | 质检不通过 | 需处理 |

#### 入库上架子状态（putaway_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| putaway_status | NOT_PUTAWAY | 未上架 | 未开始上架 |
| putaway_status | PART_PUTAWAY | 部分上架 | 部分上架完成 |
| putaway_status | PUTAWAYED | 已上架 | 上架完成 |

#### 出库拣货子状态（pick_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| pick_status | NOT_PICKED | 未拣货 | 未开始拣货 |
| pick_status | PICKING | 拣货中 | 拣货执行中 |
| pick_status | PICKED | 已拣货 | 拣货完成 |
| pick_status | PICK_EXCEPTION | 拣货异常 | 差异/缺货 |

#### 出库复核子状态（verify_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| verify_status | NOT_VERIFIED | 未复核 | 待复核 |
| verify_status | VERIFIED | 已复核 | 复核通过 |
| verify_status | VERIFY_EXCEPTION | 复核异常 | 差异待处理 |

#### 出库打包子状态（pack_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| pack_status | NOT_PACKED | 未打包 | 未开始打包 |
| pack_status | PACKING | 打包中 | 打包执行中 |
| pack_status | PACKED | 已打包 | 打包完成 |

#### 出库发运子状态（ship_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| ship_status | NOT_SHIPPED | 未发运 | 待交接物流 |
| ship_status | SHIPPING | 发运中 | 交接中 |
| ship_status | SHIPPED | 已发运 | 已交接物流 |
| ship_status | SHIP_FAILED | 发运失败 | 需重试 |

### G3. 采购子状态
#### 采购审批子状态（approval_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| approval_status | PENDING_APPROVAL | 待审批 | 等待审批 |
| approval_status | APPROVED | 已批准 | 审批通过 |
| approval_status | REJECTED | 已驳回 | 审批拒绝 |

#### 供应商确认子状态（supplier_confirm_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| supplier_confirm_status | NOT_CONFIRMED | 待确认 | 等待供应商确认 |
| supplier_confirm_status | CONFIRMED | 已确认 | 确认交期 |
| supplier_confirm_status | REJECTED | 已拒绝 | 供应商拒单 |

#### 采购收货子状态（receiving_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| receiving_status | NOT_RECEIVED | 未收货 | 尚未到货 |
| receiving_status | PART_RECEIVED | 部分收货 | 部分到货 |
| receiving_status | RECEIVED | 已收货 | 收货完成 |

#### 采购质检子状态（purchase_qc_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| purchase_qc_status | PENDING | 待质检 | 等待质检 |
| purchase_qc_status | PASSED | 质检通过 | 可入库 |
| purchase_qc_status | FAILED | 质检不通过 | 退货/让步接收 |

#### 采购结算子状态（settlement_status）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| settlement_status | UNSETTLED | 未结算 | 待结算 |
| settlement_status | SETTLING | 结算中 | 对账/开票 |
| settlement_status | SETTLED | 已结算 | 结算完成 |

### G4. 售后子状态
#### 赔付子状态（compensation_status，可选）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| compensation_status | NOT_REQUIRED | 无赔付 | 无需赔付 |
| compensation_status | PENDING_APPROVAL | 待审核 | 等待审批 |
| compensation_status | APPROVED | 已批准 | 赔付通过 |
| compensation_status | PAID | 已赔付 | 赔付完成 |
| compensation_status | REJECTED | 已驳回 | 赔付拒绝 |

#### 举证子状态（evidence_status，可选）
| API字段名 | 状态码 | UI显示文案 | 说明 |
| --- | --- | --- | --- |
| evidence_status | NOT_REQUIRED | 无需举证 | 无举证要求 |
| evidence_status | REQUIRED | 需举证 | 等待举证 |
| evidence_status | SUBMITTED | 已提交 | 已提交材料 |
| evidence_status | VERIFIED | 通过 | 举证通过 |
| evidence_status | REJECTED | 驳回 | 举证不通过 |
