# 网店商品下载对比订单审核 UI 元素级复刻设计

**日期**: 2026-04-18

## 目标

将“网店商品下载”页面重构为在元素层级、工作台节奏和区域联动上尽量贴近订单审核页的工作台页面，同时保持商品下载、匹配、建档这套业务语义不变。

这次改造的目标不是“外层壳子看起来像订单审核”，而是让用户进入页面后的视觉重心、操作路径、焦点切换方式、详情跟随关系，都尽量与订单审核工作台一致。

## 范围

### 包含

- 复用 `OrderReviewWorkbenchTemplate`
- 复用 `OrderReviewWorkbenchToolbarTemplate`
- 将网店商品下载页重构为左侧面板、中间主表、右侧详情、底部面板四区工作台
- 将左侧面板改造成更接近订单审核左侧工作台的双 Tab 结构
- 将主表交互改造成更接近订单审核主区的“激活行 + 选择集”双轨模式
- 将右侧详情面板改造成更接近订单审核右侧详情模板的结构化信息区
- 将底部面板改造成更接近订单审核底栏的工作台式承载区
- 保留下载平台 SKU、自动匹配、手动匹配、创建 ERP 档案、删除匹配等现有业务动作
- 保留现有弹窗式录入动作，但让其挂接在新的工作台节奏上

### 不包含

- 不修改 `shopProducts` 相关 API 协议和后端接口
- 不将商品页面抽象成新的全局工作台模板
- 不把商品页硬改成订单业务语义
- 不趁机改造其他商品治理页面
- 不扩展成复杂的右侧表单编辑器

## 当前问题

当前 [ShopProductDownloadPage.tsx](F:\erp\apps\erp-web\src\pages\shopProducts\ShopProductDownloadPage.tsx) 已经复用了工作台外壳，但仍然属于“业务页拼装”状态，而不是“订单审核式工作台”状态：

- 左侧仍然是 `Tabs + Card + Statistic` 的自定义组合，筛选区和概览区更像信息卡片，而不是工作台导航区。
- 主表仍然偏普通查询页模式，虽然已有选中行和激活行的雏形，但页面整体没有完全建立订单审核式的操作节奏。
- 右侧详情仍然是 `Card + Descriptions` 的独立面板，没有复用订单审核右侧模板的结构化信息槽位。
- 底部区域仍然是普通 `Tabs` 加表格/说明卡，不像订单审核底栏那样承担“当前工作区反馈”的角色。
- 当前页面的“选中一批”和“正在查看一条”没有被清晰拆分成两个稳定概念，用户较难形成订单审核式的工作流心智。

## 设计结论

本次采用“深度复刻”方案：

- 业务语义保持商品下载、匹配、建档不变
- UI 元素层级、工作台区域职责和联动方式尽量贴近订单审核页
- 页面继续作为当前业务入口和状态编排层
- 左、中、右、下四区改造成与订单审核更同构的本地组件

这意味着页面不是只做视觉对齐，而是做“工作台行为对齐”：

- 左侧负责驱动主表范围和当前工作状态
- 中间负责主操作和行焦点切换
- 右侧严格跟随当前激活记录
- 底部承载任务、失败反馈和批量动作上下文

## 页面骨架设计

页面继续使用 [ShopProductDownloadPage.tsx](F:\erp\apps\erp-web\src\pages\shopProducts\ShopProductDownloadPage.tsx) 作为入口，但角色从“页面内容拼装器”调整为“订单审核工作台适配器”。

它负责：

- 查询参数、分页、任务分页等页面级状态
- 主表激活行与批量选择集状态
- 下载、自动匹配、手动匹配、创建 ERP 档案、删除匹配等动作入口
- 弹窗开关与提交 loading
- 数据刷新、消息提示和工作台四区装配

最外层继续通过 [OrderReviewWorkbenchTemplate.tsx](F:\erp\apps\erp-web\src\components\workbench\templates\OrderReviewWorkbenchTemplate.tsx) 组织四区布局：

- 左侧为筛选/概览工作台面板
- 中间为主表工作区
- 右侧为当前激活商品详情
- 底部为任务、失败反馈和批量上下文

## 工具栏设计

工具栏继续基于 [OrderReviewWorkbenchToolbarTemplate.tsx](F:\erp\apps\erp-web\src\components\workbench\templates\OrderReviewWorkbenchToolbarTemplate.tsx)，但节奏要更接近订单审核页。

### 布局原则

- 左侧为核心动作组，不再只是“标题 + 一串按钮”
- 中间保留搜索输入与查询触发
- 右侧继续保留导出、打印、帮助等统一工具位
- 整体按钮密度、搜索区宽度和信息层级尽量贴近订单审核

### 元素映射

- 页面标题保留“网店商品下载”语义
- 选中数量作为轻量状态提示，贴近订单审核工具栏里的工作台信息节奏
- 下载平台 SKU、创建 ERP 档案、自动匹配、手动匹配、删除匹配构成主操作组
- 搜索框继续承载平台 SKU、商家 SKU、商品标题搜索
- 导出、打印、帮助保持统一工作台占位

## 左侧面板设计

左侧面板从当前 [ShopProductWorkbenchLeftPanel.tsx](F:\erp\apps\erp-web\src\pages\shopProducts\ShopProductWorkbenchLeftPanel.tsx) 的 `Tabs + Card + Statistic` 组合，调整为更接近 [OrderReviewLeftPanel.tsx](F:\erp\apps\erp-web\src\pages\orders\review-native\OrderReviewLeftPanel.tsx) 的工作台双 Tab 结构。

### Tab 1：筛选

职责是驱动主表范围，而不是展示独立信息卡片。

包含：

- 平台
- 店铺
- 商品状态
- 匹配状态
- 映射状态
- 平台更新时间范围
- 应用筛选按钮
- 重置筛选按钮

设计要求：

- 字段布局密度、对齐方式和操作按钮节奏尽量向订单审核筛选区靠拢
- 提交后重置主表分页到第一页
- 重置时同步清理工具栏关键字搜索

### Tab 2：概览

职责是让左侧承担工作台状态视图，而不是简单统计面板。

包含：

- 总量
- 已匹配
- 未匹配
- 冲突
- 已选中
- 当前服饰映射模板提示
- 轻量流程提示

设计要求：

- 统计信息的组织方式和视觉节奏尽量贴近订单审核左侧概览
- 模板提示和流程说明下沉为辅助信息，避免抢占主要交互

## 主表设计

主表继续由 [ShopProductWorkbenchMainTable.tsx](F:\erp\apps\erp-web\src\pages\shopProducts\ShopProductWorkbenchMainTable.tsx) 承载，但从普通查询表行为推进到更接近订单审核工作台主区。

### 保留字段

- 平台
- 店铺
- 平台 SKU
- 平台 SKU 名称
- 商家 SKU
- ERP SKU
- 匹配状态
- 冲突原因
- 价格
- 平台更新时间

### 行为设计

- 选择集和激活行是两套独立状态
- 勾选框只维护批量选择集
- 行点击只更新激活行
- 右侧详情严格跟随激活行
- 底部面板承载当前工作区反馈，而不是简单跟随选中集变化

### 深度同构要求

- 行高、列头密度、活动行高亮、表尾统计节奏尽量向订单审核对齐
- 主表区域的使用感要更接近“工作台主区”，而不是一般列表页
- 如 `variant="query"` 无法完全承载目标交互，优先在商品页局部补齐行为，不破坏订单审核现有逻辑

## 右侧详情设计

右侧详情从当前 [ShopProductWorkbenchDetailPanel.tsx](F:\erp\apps\erp-web\src\pages\shopProducts\ShopProductWorkbenchDetailPanel.tsx) 的 `Card + Descriptions` 改造成更接近 [OrderReviewDetailPanel.tsx](F:\erp\apps\erp-web\src\pages\orders\review-native\OrderReviewDetailPanel.tsx) 的结构化模板。

### 设计原则

- 保持商品业务语义
- 复用订单审核右侧模板的信息槽位节奏
- 不生硬照搬物流/仓库编辑语义
- 让右侧像“工作台详情区”，而不是“说明卡片”

### 字段映射思路

- `orderId` 类槽位映射为平台 SKU ID 或记录 ID
- `platformOrderId` 类槽位映射为平台商品 ID 或商家 SKU
- `platform` 槽位映射为平台
- `shop` 槽位映射为店铺
- 其余主要信息映射为商品标题、规格、价格、库存、匹配来源、映射状态、冲突原因、ERP SKU、ERP SKU 名称、平台更新时间

### 状态提示

右侧底部保留轻量状态提示，用于表达：

- 当前记录未匹配，建议执行自动匹配或手动匹配
- 当前记录已匹配，可继续创建 ERP 档案或复核结果
- 当前记录存在冲突，应优先处理失败原因

### 空状态

当没有激活行时，显示统一的空状态提示，引导用户先在主表中选择一条记录查看详情。

## 底部面板设计

底部面板从当前 [ShopProductWorkbenchBottomPanel.tsx](F:\erp\apps\erp-web\src\pages\shopProducts\ShopProductWorkbenchBottomPanel.tsx) 的普通 `Tabs`，改造成更接近 [OrderReviewBottomPanel.tsx](F:\erp\apps\erp-web\src\pages\orders\review-native\OrderReviewBottomPanel.tsx) 的底栏结构。

### Tab 1：任务中心

职责是承载当前工作区的后处理结果。

包含：

- 任务类型
- 状态
- 成功数
- 失败数
- 分页

设计要求：

- 呈现方式更像订单审核底部工作区
- 不再只是把普通 Ant Design 表格塞进 Tab

### Tab 2：匹配失败

职责是长期可见地承载最近自动匹配失败反馈。

包含：

- 平台 SKU
- 商家 SKU
- 失败原因

设计要求：

- 从简单列表整理成更有明细感的失败面板
- 降低对临时弹窗的依赖，弹窗只做即时提醒

### Tab 3：批量操作

职责是呈现当前批量动作上下文，而不是说明文案卡片。

包含：

- 当前已选条数
- 当前作用范围
- 批量动作与主表/右侧联动提示

设计要求：

- 用户可以明确区分“当前正在看哪一条”和“当前准备处理哪一批”

## 状态与联动设计

### 页面级状态

- `filters`
- `pagination`
- `taskPagination`
- `selectedRowKeys`
- `selectedRows`
- `activeSkuRecordId`
- `autoMatchFailures`
- 各类弹窗开关
- 各类动作 loading

### 联动规则

- 主表单击：只更新 `activeSkuRecordId`
- 主表勾选：只更新 `selectedRowKeys` 和 `selectedRows`
- 右侧详情：只跟随 `activeSkuRecordId`
- 底部面板：以任务反馈、失败反馈和批量上下文为主，不作为右侧详情的重复展示
- 批量动作：默认作用于 `selectedRowKeys`，部分动作允许切换为 `filtered` 范围

### 刷新回退规则

- 若原激活行仍存在于当前结果中，保持激活
- 若原激活行不存在但当前页仍有数据，则回退到当前页首条
- 若当前页无数据，则右侧和底部进入空状态或弱提示状态

## 错误与空状态

- 主表查询失败由主表区域承载重试
- 右侧无激活行时显示空状态
- 任务中心为空时显示“暂无处理任务”
- 匹配失败为空时显示“最近一次自动匹配无失败项”
- 业务动作失败继续使用 `message.error`

## 影响文件

- `apps/erp-web/src/pages/shopProducts/ShopProductDownloadPage.tsx`
- `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchLeftPanel.tsx`
- `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchMainTable.tsx`
- `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchDetailPanel.tsx`
- `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchBottomPanel.tsx`
- `apps/erp-web/src/pages/shopProducts/shopProductWorkbenchModel.ts`
- `apps/erp-web/src/pages/shopProducts/shopProductDownloadPage.css`
- `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchPanels.test.ts`
- `apps/erp-web/src/pages/shopProducts/ShopProductWorkbenchMainTable.template.test.ts`
- `apps/erp-web/src/pages/shopProducts/shopProductWorkbenchModel.test.ts`

## 测试策略

至少覆盖以下行为：

- 左侧筛选与概览 Tab 的静态渲染
- 主表激活行与批量选择集分离逻辑
- 激活行刷新后的回退逻辑
- 右侧详情跟随激活行变化
- 底部任务/失败/批量面板的静态装配与关键状态展示
- 现有商品匹配治理链路回归

优先复用和补充现有测试：

- `ShopProductWorkbenchPanels.test.ts`
- `ShopProductWorkbenchMainTable.template.test.ts`
- `shopProductWorkbenchModel.test.ts`
- `shop-product-match-governance.spec.ts`

## 风险与控制

### 风险

- 选择集和激活行重新混在一起，导致右侧详情或批量动作错位
- 右侧详情为了复刻模板而出现错误语义映射
- 底部任务区改造后降低了任务反馈可见性
- 页面重构范围较大，容易引入 UI 回归

### 控制策略

- 优先锁定激活行与选择集的行为边界
- 先做本地组件同构，不引入新的全局抽象层
- 保留现有业务接口和动作入口，只调整元素承载与联动关系
- 用现有页面级测试和 E2E 回归保护主要链路
