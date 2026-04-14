# 前端菜单与路由迁移方案（2026-04-13）

## 1. 目标

本次调整把 ERP Web 前端从“功能页面不断追加、菜单归属混乱”的状态，收敛为适合工业级电商 ERP 的两层信息架构：

- 一级菜单：稳定业务域
- 二级菜单：直接页面入口
- 路由：统一使用 canonical path
- 兼容策略：旧路径继续可访问，但只负责重定向到新路径

本次不做页面能力删减，重点解决以下问题：

- 菜单按历史开发顺序堆叠，缺少业务域边界
- 渠道接入、OMS、WMS、TMS、财务、系统治理混放
- 顶部模块导航依赖旧菜单白名单，无法跟随 IA 演进
- 页面内跳转与 E2E 仍依赖旧路径

## 2. 新的一级菜单

1. 工作台
2. 渠道与店铺
3. 销售 OMS
4. 商品与主数据
5. 采购与供应商
6. 计划与补货
7. 仓储 WMS
8. 物流 TMS
9. 客户与售后
10. 财务结算
11. 营销与价格
12. 经营分析
13. 平台与集成
14. 系统治理

## 3. 新的二级菜单

### 3.1 工作台

| 二级菜单 | 路由 |
| --- | --- |
| 工作台 | `/dashboard` |

### 3.2 渠道与店铺

| 二级菜单 | 路由 |
| --- | --- |
| 渠道目录 | `/channels` |
| 店铺接入 | `/channels/shops` |
| 网店订单查询 | `/channels/shop-orders` |
| 商品匹配 | `/channels/product-match` |
| 渠道映射 | `/channels/mappings` |
| 渠道同步任务 | `/channels/sync-jobs` |

### 3.3 销售 OMS

| 二级菜单 | 路由 |
| --- | --- |
| 订单查询 | `/orders` |
| 新建订单 | `/sales/new-order` |
| 导入订单 | `/sales/import-order` |
| 订单审核 | `/orders/review` |
| 异常中心 | `/orders/exceptions` |
| 异常 SLA | `/orders/exception-sla` |
| 拆单合单 | `/orders/split-merge` |
| 审单规则 | `/orders/review-rules` |
| 销售结算 | `/orders/settlement` |
| 销售对账 | `/orders/reconciliations` |
| 发货/包裹 | `/shipments` |
| 回传失败 | `/shipments/push-failures` |
| OMS 包裹 | `/oms/shipments` |
| 物流信息上传 | `/sales/logistics-upload` |
| 销售策略设置 | `/sales/strategy-settings` |
| 销售设置 | `/sales/settings` |

### 3.4 商品与主数据

| 二级菜单 | 路由 |
| --- | --- |
| 商品管理 | `/products` |
| 商品条码 | `/products/barcodes` |
| 商品单位 | `/products/units` |
| 包装规格 | `/products/packaging-specs` |
| 组合装管理 | `/kits` |
| 渠道可售与上架 | `/products/channel-availability` |
| 商品变更控制 | `/products/change-control` |
| 商品生命周期 | `/products/lifecycle` |
| 商品相差 | `/products/diff` |
| 标签目录 | `/tags` |
| 订单标签 | `/tags/orders` |
| 货品标签 | `/tags/skus` |

### 3.5 采购与供应商

| 二级菜单 | 路由 |
| --- | --- |
| 供应商目录 | `/procurement/suppliers` |
| 采购申请 | `/procurement/requests` |
| 采购计划 | `/procurement/plans` |
| 寻源与选品 | `/procurement/sourcing` |
| 采购订单 | `/procurement/orders` |
| 采购到货 | `/procurement/receipts` |
| 采购建议 | `/procurement/suggestions` |
| 采购退货 | `/procurement/returns` |
| 质检单 | `/procurement/inspections` |
| 供应商对账 | `/procurement/reconciliations` |
| 供应商绩效 | `/procurement/performance` |

### 3.6 计划与补货

| 二级菜单 | 路由 |
| --- | --- |
| 销售预测 | `/planning/forecast` |
| 安全库存与补货策略 | `/planning/replenishment-rules` |
| 采购补货计划 | `/planning/purchase-plans` |
| 调拨建议 | `/planning/transfer-suggestions` |
| 促销备货 | `/planning/promo-stocking` |
| 计划执行反馈 | `/planning/plan-execution` |

### 3.7 仓储 WMS

| 二级菜单 | 路由 |
| --- | --- |
| 仓库目录 | `/wms/warehouses` |
| 库区目录 | `/wms/warehouse-areas` |
| 货位目录 | `/wms/warehouse-locations` |
| 库位库存 | `/wms/location-balances` |
| 容器目录 | `/wms/containers` |
| 寄方目录 | `/wms/sender-addresses` |
| 流程配置 | `/wms/workflow-configs` |
| 收货入库 | `/wms/inbounds` |
| 收货查询 | `/wms/inbounds/search` |
| 收货登记 | `/wms/inbounds/receive-entry` |
| 退货入库 | `/wms/return-inbounds` |
| 拒收入库查询 | `/wms/inbound-rejects` |
| 出库单 | `/wms/outbounds` |
| 波次管理 | `/wms/waves` |
| 拣货任务 | `/wms/pick-tasks` |
| 打包复核 | `/wms/pack` |
| 按单发货 | `/wms/ship` |
| 出库确认 | `/wms/outbound-confirm` |
| 劳务任务 | `/wms/labor` |
| 总库存查询 | `/inventory` |
| 分仓库存 | `/inventory/warehouse` |
| 库存成本 | `/inventory/costs` |
| 库存台账 | `/inventory/ledgers` |
| 库存锁定 | `/inventory/locks` |
| 库存同步 | `/inventory/sync-jobs` |
| 盘点管理 | `/inventory/stocktakes` |
| 盘点任务 | `/inventory/stocktake-tasks` |
| 调拨管理 | `/wms/stock-transfers` |
| 库存冻结 | `/wms/inventory-freezes` |
| 报损报溢 | `/wms/stock-adjustments` |
| 移库管理 | `/wms/stock-moves` |
| 库存预警看板 | `/inventory/insights/alerts` |
| 发货预处理策略 | `/wms/shipment-preprocess-strategies` |
| 拣货推荐策略 | `/wms/pick-recommend-strategies` |
| 盘点策略/建议 | `/wms/stock-count-strategies` |
| 库位优化 | `/wms/slotting` |
| 补货任务 | `/wms/replenishment` |
| 月台管理 | `/wms/dock-yard` |

### 3.8 物流 TMS

| 二级菜单 | 路由 |
| --- | --- |
| 承运商目录 | `/logistics/couriers` |
| 物流产品 | `/logistics/products` |
| 物流价格 | `/logistics/prices` |
| 接口配置 | `/logistics/interfaces` |
| 物流追踪 | `/logistics/tracking` |
| 物流异常 | `/logistics/exceptions` |
| 物流结算 | `/logistics/settlements` |
| 仓配优化 | `/logistics/warehouse-optimization` |
| 配送安装 | `/logistics/installations` |

### 3.9 客户与售后

| 二级菜单 | 路由 |
| --- | --- |
| 客户档案 | `/customers` |
| 客户分类 | `/customers/categories` |
| 会员等级 | `/customers/member-levels` |
| 客户标签 | `/customers/tags` |
| 积分与储值 | `/customers/loyalty` |
| 客户分群 | `/customers/segments` |
| 生命周期运营 | `/customers/lifecycle` |
| 客服工单 | `/customers/service-tickets` |
| 售后单 | `/after-sales` |
| 退款拦截 | `/after-sales/intercepts` |
| 换货补发 | `/after-sales/exchanges` |
| 退货质检 | `/after-sales/inspections` |
| 售后 SLA | `/after-sales/sla` |
| 售后批量导入 | `/after-sales/bulk-edit` |
| 平台售后 | `/after-sale-cases` |
| 退款处理 | `/refund-docs` |
| 售后设置 | `/after-sales/settings` |

### 3.10 财务结算

| 二级菜单 | 路由 |
| --- | --- |
| 应收管理 | `/finance/ar` |
| 应付管理 | `/finance/ap` |
| 对账中心 | `/finance/reconcile` |
| 开票中心 | `/finance/invoices` |
| 应付单据 | `/finance/ap-docs` |
| 费用分摊 | `/finance/expenses` |
| 成本核算 | `/finance/costing` |
| 利润报表 | `/finance/profit` |
| 代销结算 | `/finance/consignment` |
| 税务管理 | `/finance/tax` |

### 3.11 营销与价格

| 二级菜单 | 路由 |
| --- | --- |
| 渠道价表 | `/marketing/prices` |
| 促销活动 | `/marketing/promotions` |
| 赠品策略 | `/marketing/gifts` |
| 优惠券 | `/marketing/coupons` |
| 促销费用 | `/marketing/costs` |
| 活动复盘 | `/marketing/postmortems` |

### 3.12 经营分析

| 二级菜单 | 路由 |
| --- | --- |
| 发货统计 | `/stats/shipping` |
| 销售统计 | `/stats/sales` |
| 销售日报表 | `/stats/sales-daily` |
| 销售趋势分析 | `/stats/sales-trends` |
| 滞销分析 | `/stats/slow-moving` |
| 畅销分析 | `/stats/best-sellers` |
| 销售周期 | `/stats/sales-cycle` |
| 经营分析报表 | `/reports` |
| 导出任务 | `/exports` |

### 3.13 平台与集成

| 二级菜单 | 路由 |
| --- | --- |
| 平台接入总览 | `/integrations/center` |
| 集成同步任务 | `/integrations/sync-jobs` |
| 打印设备 | `/integrations/print-devices` |
| 打印映射 | `/integrations/print-mappings` |
| 打印任务 | `/print-jobs` |
| 打印模板 | `/print-templates` |

### 3.14 系统治理

| 二级菜单 | 路由 |
| --- | --- |
| 系统设置 | `/system/settings` |
| 单据编号/流程配置 | `/system/workflows` |
| 自定义字段 | `/system/custom-fields` |
| 登录账户 | `/system/users` |
| 角色管理 | `/system/roles` |
| 权限目录 | `/system/permissions` |
| 数据权限 | `/system/data-scopes` |
| 审计日志 | `/system/audit-logs` |
| 操作日志 | `/system/operation-logs` |
| 审计报告 | `/system/audit-reports` |
| 可观测性面板 | `/system/observability` |
| 失败队列 | `/failure-queue` |

## 4. canonical route 迁移表

| 旧路径 | 新路径 | 归属变化 |
| --- | --- | --- |
| `/mappings` | `/channels/mappings` | 从散落入口并入渠道与店铺 |
| `/sales/shops` | `/channels/shops` | 从销售菜单迁到渠道接入 |
| `/sales/product-match` | `/channels/product-match` | 从销售菜单迁到渠道商品处理 |
| `/sales/shop-orders` | `/channels/shop-orders` | 从销售菜单迁到渠道网店订单 |
| `/sales/sync-job-runs` | `/channels/sync-jobs` | 从销售菜单迁到渠道同步任务 |
| `/system/integrations` | `/integrations/center` | 从系统配置迁到平台与集成 |
| `/system/sync-jobs` | `/integrations/sync-jobs` | 从系统配置迁到平台与集成 |
| `/system/print-devices` | `/integrations/print-devices` | 从系统打印设置迁到平台与集成 |
| `/system/print-mappings` | `/integrations/print-mappings` | 从系统打印设置迁到平台与集成 |
| `/suppliers` | `/procurement/suppliers` | 从目录迁到采购与供应商 |
| `/warehouses` | `/wms/warehouses` | 从目录迁到仓储 WMS |
| `/couriers` | `/logistics/couriers` | 从目录迁到物流 TMS |

## 5. 兼容策略

### 5.1 路由兼容

- 保留旧路径访问能力
- 旧路径不再承载真实页面，只做 `Navigate replace`
- 重定向保留 `query string` 和 `hash`

示例：

- `/mappings?platform=taobao#filter` 会跳转到 `/channels/mappings?platform=taobao#filter`
- `/system/integrations` 会跳转到 `/integrations/center`

### 5.2 页面内跳转兼容

已将核心页面内跳转切到新 canonical path：

- 订单详情页“去匹配”跳转到 `/channels/mappings`
- 店铺接入页“网店日志”跳转到 `/channels/sync-jobs`

### 5.3 测试兼容

E2E 已切到新 canonical path，并保留对旧地址兼容行为的验证空间。

## 6. 已落地实现

本次已完成以下前端改造：

- 重建 `appMenuItems`，形成 14 个一级菜单
- 将顶部 `ModuleHeader` 改为直接根据菜单树生成模块导航
- 去除 `WorkbenchLayout` 对旧模块名称白名单的依赖
- 引入 route alias 映射表，统一旧路由到新路由的迁移规则
- 在 `router.tsx` 中补 canonical route 和 legacy redirect
- 更新订单详情页、店铺接入页的内部跳转
- 更新关键 E2E 用例到新路由结构

## 7. 后续建议

1. 增加“菜单权限矩阵”文档，把菜单项、权限码、功能开关统一到一张表。
2. 按业务域继续整理页面标题、面包屑、快捷入口和收藏能力，避免菜单迁完但页面头信息仍旧混乱。
3. 下一轮优先梳理销售 OMS、仓储 WMS、客户与售后三大模块的页面标题规范与列表页入口顺序。
