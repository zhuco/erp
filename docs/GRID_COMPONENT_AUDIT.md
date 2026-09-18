# 易格组件可用性审计

审计日期：2026-09-10。对象：当前工作区的 `erp-grid-core`、`erp-ui-egui` 及 `apps/erp-ui` 中的组件接入代码。依据：用户已经确认的 ERP 网格范围；不追加 AG Grid 功能。

**修复状态（0.70）：本次审计的4项已处理，具备开始接入已确认 ERP 页面场景的组件基础。**无需重写架构，不再把平台设备验收列为组件前置。下方原始发现保留为修复前快照，不再作为未完成清单。

| 原发现 | 当前修复 | 验证 |
| --- | --- | --- |
| 负数复制/CSV | core `spreadsheet_value` 按类型保留合法数值，TSV/CSV共用文本防护 | 负数复制→公共粘贴规划、CSV定向测试；实际Web下载读回金额-1.25 |
| 类型编辑器 | Date/DateTime日历、Choice下拉、Entity可搜索候选；共享既有提交ticket和草稿 | 日期鼠标选日、对象键盘选择/提交、过期请求/重复ID拒绝；实际Web选日期/状态/SKU后导出读回 |
| 单元格呈现 | `show_with_options`、`GridOptions`、`CellPresentation`、`CellAction` | 图片纹理与可见区裁剪、原始值复制、按钮身份与选区/勾选保护；Ctrl点击仍勾选；实际Web Open定位子表 |
| 内部复用 | `erp_ui::components`统一现有导出、命名视图UI、框选汇总；TableView消费门面 | 公共门面调用测试及双端Clippy/构建 |

完整运行边界和文件哈希见 `qa/acceptance/RESULTS.md` 的0.70记录及 `build-070.json`。Web使用本地Lab目录验证对象选择，不代表具体业务SKU API已接入；后续页面按公共契约提供业务数据和保存即可。

Windows 候选窗、系统剪贴板权限、系统 DPI、打印机、实际读屏软件和远端 CI 属于宿主/交付环境验收，不作为组件功能缺失。组件仍负责输入事件语义、TSV 编解码、布局、导出内容及无障碍节点；下面的复制问题属于组件自身。

## 具体发现（按影响排序）

### 1. P1：区域复制会破坏负数的网格内复制粘贴

证据：[clipboard.rs](../crates/erp-ui-egui/src/clipboard.rs:36)、[editing.rs](../crates/erp-grid-core/src/editing.rs:155)、[edit_planner.rs](../crates/erp-grid-core/src/edit_planner.rs:70)。

`selection_tsv` 对所有以 `= + - @` 开头的值添加单引号，没有依据列的 `ValueKind` 区分已验证数值与文本。`parse_tsv` 只解码 TSV 引号，不移除该单引号；`plan_paste` 随后执行 Integer/Money 校验。

因此金额格 `-1.25` 经区域复制生成 `'-1.25`，粘贴到金额格时会被拒绝。整数负数同样受影响。这是源码链路可以确定的结果，本次未新增运行用例或实际改写业务数据。右键单格复制使用另一条路径，不能代替区域复制验证。

CSV 导出也有相同的不区分字段类型的前缀策略：[export.rs](../apps/erp-ui/src/export.rs:46)。XLSX 有独立的类型判断，不能据此认定 XLSX 同样损坏。

**最小修复边界：**对经过类型校验的 Integer/Money 保留合法数值；文本继续保留公式防护。一次定向覆盖负数量、负金额的复制—粘贴，以及文本公式防护，避免靠全局删除单引号破坏文本。

### 2. P1：日期、选项和可搜索业务对象的单元格编辑尚未完成

证据：[EditKind](../crates/erp-grid-core/src/editing.rs:5) 只有 ReadOnly/Text/OptionalText/Integer/Money/Choice；[editor.rs](../crates/erp-ui-egui/src/editor.rs:73) 对所有可编辑类型统一使用 `TextEdit::singleline`。Choice 仅校验字符串是否在允许集合内，没有下拉选择 UI。没有日期编辑类型或宿主自定义编辑器接入口。

日期日历存在于快速筛选行：[quick_search.rs](../crates/erp-ui-egui/src/quick_search.rs:41)，不等于可编辑单元格的日期选择器。订单验收宿主的 SKU 列仍为只读，只有数量和单价可编辑：[orders.rs](../apps/erp-ui/src/remote_view/orders.rs:986)。现有远端“筛选选项目录”也不能充当返回稳定 SKU 身份的业务对象编辑器。

**影响：**查询列表和名称/整数/金额编辑可以用；采购到货日期、状态选择、货品录入等已确认场景尚不能仅配置公共组件完成。此前把“数值、金额、日期、商品/SKU 编辑器”整体计为完成不准确。

**最小补齐边界：**现有文本编辑器保留，补日期与 Choice 控件、可搜索对象的公共编辑接入口。宿主提供查询、稳定对象 ID 和显示名称、业务校验及保存；网格负责焦点、提交/取消和草稿。复用现有提交 ticket，不引入新的编辑框架。

### 3. P2：缺少业务单元格的自定义绘制/交互入口

证据：[DataGrid::show](../crates/erp-ui-egui/src/lib.rs:541) 的数据回调只能返回 String；[绘制循环](../crates/erp-ui-egui/src/lib.rs:1670) 统一调用文本绘制。公开入口没有向宿主提供单元格 Ui/Rect 的绘制回调或等价类型化扩展。已有行右键动作，不等于行内按钮。

**影响：**商品缩略图、可点击单号、状态标签、行内动作不能通过现有列配置直接实现。Excel 图片导出只作用于导出文件，不能视为网格图片单元格已完成。这属于原 DG-12 承诺，不是新增功能。

**最小补齐边界：**在现有虚拟绘制循环中增加可选单元格呈现/动作扩展，保留默认文本快路径；原始值继续供复制、编辑与导出使用。业务数据查询和执行动作仍归宿主。无需引入第三方完整表格。

### 4. P2：部分通用能力仍留在应用私有模块，新页面接入尚需整理

证据：[应用模块声明](../apps/erp-ui/src/lib.rs:7) 将 `export`、`named_views`、`selection_summary` 声明为私有模块；[接入说明](GRID_INTEGRATION.md:3) 要求普通页面仅依赖两个公共 crate。导出编码、命名视图 UI、框选求和不能由只依赖这两个 crate 的外部页面直接使用。

这些功能**已经有实现**：命名视图数据模型在 core，导出与视图 UI 在应用；同一个 `erp-ui` crate 内的新页面可以复用，并非一律无法接入。缺口是公共组件交付与说明的边界不一致，不是缺少导出算法。

**最小整理边界：**统一一个 ERP 内部共享入口，明确哪些在 core/renderer，哪些为宿主共享适配。已有代码直接复用，禁止每页复制；不要求发布 SDK、新建平台或迁走认证/业务接口。

## 已确认能力与实际适用程度

“已具备”表示核对了当前实现和对应测试/既有操作记录，不等于本次重新运行全部交互。

| 能力 | 结论 | 实现与证据 |
| --- | --- | --- |
| 双轴虚拟化、列宽/拖序/显隐、左右固定、分组表头、结构列、分页 | 已具备，适合 ERP 查询主表 | `erp-ui-egui/lib.rs`、`column_config.rs`、`header_groups.rs`、`pagination.rs`；layout/resize/column 测试及 0.53 绘制记录 |
| 活动行、勾选、框选分离；Ctrl/Shift 勾选/取消；当前页与跨页显式勾选 | 已具备，保留既定 ERP 操作语义 | core `rows.rs`；renderer `tests.rs` 的 modifier、checkbox、shift 区间用例；按稳定 RowId 操作 |
| 全部匹配、排除个别行、范围/失效检查 | 公共状态契约已具备，后端凭证由宿主提供 | core `matched_selection.rs`；`build-068.json` 实际取消计数、保留2项排除、重试恢复27条 |
| 快速搜索、金额双输入、日期筛选、枚举、AND/OR、远端选项 | 已具备契约与现有接线，接口能力由每个宿主声明 | `quick_search.rs`、`query_builder.rs`、core `filter/options/paged_query`；商品远端目录是业务适配，不冒充 SKU 编辑选择器 |
| 远端分页、有界缓存、取消/重试、迟到响应隔离 | 已具备，适合大查询结果按页显示 | core `remote.rs`；现有真实商品查询接线及 PageLab HTTP 证据；禁止客户端全量下载伪装远端分页 |
| 文本/整数/金额编辑、Tab 导航、草稿、保存 ticket、冲突保护 | 已具备基础编辑链路；类型编辑器缺口见第2项 | `editor.rs`、`editor_tests.rs`、core `editing.rs`；已有商品/SKU 保存重开、订单数量/单价和 Unknown 恢复记录 |
| 批量粘贴/填充、预览、移除/撤销重做、部分失败/重试反馈 | 公共规划与反馈已具备；区域复制负数缺陷见第1项 | core `edit_planner/actions`；renderer `ChangePreview/BatchActionPanel`；商品与SKU跨页填充既有实测 |
| 底部子表、默认关闭的行内展开、固定合计、分组和懒加载树 | 已具备基础模型和交互 | `show_with_details`、`SummaryRow/TreeRowVisual`、core `tree/report`；真实仓库/库位加载和库存汇总记录。分组选择范围仍由宿主明确配置 |
| 基础透视、柱/折线图、公式计算 | 已具备基本 ERP 报表积木 | core `ReportAccumulator/FormulaGraph`、renderer `report_chart`；ReportView 为有界数据夹具，真实库存聚合另有后端适配，不代表任意业务透视后端自动具备 |
| CSV、多工作表 XLSX、样式/图片、打印 HTML、任务下载 | 有现成实现，复用边界及 CSV 类型问题见第1/4项 | 应用 `export.rs` 和导出宿主；库存 XLSX 内容读回、中文打印 HTML 及跨页下载记录。大型任务规模和宿主文件交付不等于通用编码器验收 |
| 命名视图、查找定位、状态隔离、主题 | 核心契约已具备；部分配套 UI 在应用 | core `preferences/find`、renderer `GridTheme`、现有 view/find 适配。页面/实例独立 ID 和状态是接入责任，品牌色、最终文案/密度在 UI 阶段确定 |

## 性能和验证证据的分量

- 当前 native/WASM 文件 SHA256 与 `qa/acceptance/build-069.json` 一致。本次未构建、未修改 Rust 源码、未写入业务数据；工作区无 `.git`，没有 Git 差异证据。
- 历史测试并非全部首次通过：`qa/grid-053-tests.log` 包含两个失败；后续 fill 定向与 renderer 日志通过，0.55 订单也有单独修正后的通过日志。本次检查了这些实际输出，不把零匹配或初次失败记成通过。
- 万行×30列首绘制/合成输入：native P95 4.84ms、Web 36.5ms；HTTP 百万条逻辑结果按100条取页：native 2.89ms、Web31.3ms；万行驻留100次/秒更新：native0.96ms、Web17ms。数据来源分别是 `first-load.metadata.json`、`page.metadata.json`、`update-resident.metadata.json`，均绑定各自历史构建，不冒充当前哈希重新跑过。
- 这些证据支持组件的虚拟化和有界处理路线，不能得出“比 AG Grid 更快”。没有同设备、同负载的对照；也不证明数据库总查询延迟或真实店铺所有场景。
- 页面资源释放属于组件生命周期契约与宿主接线的共同责任。已有缓存/取消用例和局部运行证据；完整多页面场景没有被证明。本审计不以此要求另造测试平台或阻塞普通列表接入。

## 本轮决策与结束边界

保留当前 Rust core + egui renderer + 宿主适配路线。四项修复后，查询列表、基础资料编辑、货品对象选择和库存展示可按公共接口接入。具体业务保存/权限仍由宿主承担，不能把通用组件交付等同整套 ERP 上线。

本轮仅完成上述4项，未扩展报表/图表品类或业务页面。后续针对实际接入缺陷维护，不重新开发这些功能。

文档状态另有漂移：`UI_COMPONENT_LIBRARY.md` 仍含“核心契约未实现”“自动约束待实现”的历史标题，`GRID_INTEGRATION.md` 同时保留旧更新烟测与新测量说明；ROADMAP 之前的“只剩平台验收”过于乐观。本审计以源码为准，不能用旧状态追加任务，也不能用旧完成结论跳过上述缺口。
