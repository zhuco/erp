# DataGrid 页面接入

本文只说明当前已公开的网格接口。基础页面依赖 `erp-grid-core` 和
`erp-ui-egui`。同一 ERP 应用内的配套服务统一走 `crate::components`（外部 Rust 消费者可用 `erp_ui::components`），不要复制 Lab 或依赖 `remote_view` 下的私有业务模块。

## 0.70 审计修复后的共享入口

- `erp_ui::components` 统一导出 `NamedViewsUi`、`SelectionSummary`、`Format`、`WorkbookSheet`、`SheetOptions`、`SheetImage`、`encode/encode_workbook/encode_workbook_with_options`、`Delivery`。这是已有实现的 ERP 内部共享门面，不是新的 SDK；认证、保存和文件交付仍由宿主管理。实验台 TableView 已使用该入口。
- `EditKind::Date` 使用 `YYYY-MM-DD`；`DateTime` 使用 `YYYY-MM-DD HH:MM:SS`，时区由列/宿主声明。单元格支持直接输入和日历；`Choice` 支持下拉选择，最多显示100项，更多对象用 Entity。选择修改草稿，Enter/Tab 或既有宿主确认才提交，失败保留草稿和原值。
- `EditKind::Entity` 存储完整、非零 UUID，标签不能充当身份。单元格搜索有250ms防抖，组合输入期间不发查询。`take_entity_search()` 取一次待发送请求；宿主每帧比较 `entity_search()` 与在途请求，身份变化或返回None时取消旧网络任务。
- 宿主按账号/权限查询后调用 `resolve_entity_options(&request, result)`，每次最多100个不同ID，标签每项最多4096字节；搜索过期、跨实例、编辑已取消或重复回包返回false。结果超过100时应由宿主搜索缩小范围，不能全量加载再伪装远端选项。失败有结果提示和明确重试。
- 对象候选支持上下键和鼠标。搜索输入中的第一次Enter选候选，下一次Enter提交；也可选择后Tab连续编辑。返回 `CellEdit.after` 为选中UUID，宿主仍须复核权限和业务规则。公共组件不内置 SKU API。Lab的Related SKU使用20条本地目录，仅演示该契约。
- `DataGrid::show_with_options(ui, raw_value, GridOptions, detail)` 为可选呈现入口，已有 `show/show_with_details` 保持兼容。`GridOptions.present` 是无I/O的轻量回调，返回默认文本、格式化文本、Badge、Image或Action；纹理由宿主加载和有界缓存。
- `CellContext` 包含视图行位置、稳定RowId、ColumnId及原始值。呈现标签不覆盖原始值，复制、编辑及导出仍使用原始值。Action经 `GridOutput.cell_action` 返回单元格的稳定行/列和动作ID，由宿主执行；普通点击不清除框选/勾选、不进入编辑。Ctrl/Shift保留行勾选语义。不要在呈现回调发请求或执行动作。
- `spreadsheet_value` 按字段类型保护文本公式，保留合法整数/金额的负号。区域TSV及CSV共用，负数可复制回网格；XLSX继续走已有类型编码。

真实入口：

- `crates/erp-grid-core/src/lib.rs`：列、身份、查询、选择、编辑和分页状态。
- `crates/erp-grid-core/src/remote.rs`：有界远端页生命周期。
- `crates/erp-grid-core/src/paged_query.rs`：可序列化的服务端查询。
- `crates/erp-ui-egui/src/lib.rs`：`DataGrid`、`GridState`、`GridOutput`。
- `crates/erp-ui-egui/src/pagination.rs`：`show_pagination`。

## 最小接入清单

1. 为每个业务字段分配稳定 `ColumnId`；列标题、宽度、筛选类型、值类型和
   `EditKind` 由宿主配置。业务主键使用稳定且非零的 `RowId`。
2. 宿主持有 `Vec<Column>`、`GridState`、当前行快照和网络任务。
   `DataGrid` 只绘制并返回意图，不访问 HTTP、数据库或认证信息。
   主表、子表及弹窗各用独立且稳定的 egui ID；相同标题不能代替实例身份。
3. 本地数据变化后提供新的行快照。远端数据使用 `RemoteQuery`、
   `PageAddress`、`RemotePages`，不要下载全量后伪装服务端分页。
4. 查询范围、授权身份或列身份变化时增加页面代次并调用
   `GridState::bind_view`。同一查询翻页调用 `rebind_page`；同页身份顺序未变的
   刷新才可调用 `adopt_generation`。
5. `GridOutput` 中的排序、筛选、编辑、粘贴、填充和行动作都是意图。
   宿主校验权限、版本和业务规则后才执行副作用。

## 渲染骨架

```rust
use eframe::egui;
use erp_grid_core::{Column, ColumnId, RowId};
use erp_ui_egui::{DataGrid, GridState};

fn grid(
    ui: &mut egui::Ui,
    grid_id: egui::Id,
    columns: &mut Vec<Column>,
    state: &mut GridState,
    rows: &[MyRow],
) {
    let output = DataGrid {
        id: grid_id,
        columns,
        rows: rows.len(),
        state,
        row_id: Some(&|index| rows[index].id),
    }
    .show(ui, |index, column| rows[index].text(column));

    if output.quick_search_changed { /* 防抖后重查完整范围 */ }
    if output.sorts_changed { /* 取消旧请求并按新排序重查 */ }
    if let (Some(ticket), Some(edit)) = (output.edit_ticket, output.edits.first()) {
        /* 保存 edit；完成后调用 state.resolve_edit(ticket, result) */
    }
}

struct MyRow { id: RowId }
impl MyRow {
    fn text(&self, _column: ColumnId) -> String { String::new() }
}
```

实际页面应从自己的已加载快照返回单元格文本，不能在 `show` 回调中发请求。

## 远端分页骨架

```rust
use erp_grid_core::remote::{
    PageAcceptance, PageAddress, PageStart, RemotePage, RemotePages, RemoteQuery,
};

fn request_page(
    pages: &mut RemotePages,
    query: RemoteQuery,
    address: PageAddress,
) {
    for ticket in pages.bind(query) { abort_transport(ticket); }
    match pages.start(address) {
        Ok(PageStart::Fetch(request)) => start_http(request),
        Ok(PageStart::Cached | PageStart::Pending(_)) => {}
        Err(error) => show_error(error),
    }
}

fn complete(pages: &mut RemotePages, request: &erp_grid_core::remote::PageRequest,
            result: Result<RemotePage, String>) {
    match pages.accept(request, result) {
        PageAcceptance::Stored => {}
        PageAcceptance::Stale => {} // 迟到响应不得发布
        PageAcceptance::Failed(error) => show_error(error),
    }
}
# fn abort_transport(_: u64) {}
# fn start_http(_: erp_grid_core::remote::PageRequest) {}
# fn show_error(_: String) {}
```

`bind`、`clear` 和 `invalidate` 返回需要取消的传输 ticket。即使底层取消失败，
也必须把原 `PageRequest` 交回 `accept`，由代次和查询身份拒绝迟到结果。
页面通过 `pages.page(&address)` 读取已验证页，通过 `pages.error(&address)` 读取错误。

## 编辑与列坐标

`GridOutput.edits` 必须与同次的 `edit_ticket` 配对。保存成功或明确失败后调用
`GridState::resolve_edit`；网络结果未知时不要回报成功，也不要生成新请求身份重发。
离开页面前可用 `edit_draft` 检查草稿、`hold_edit` 阻止回焦自动保存，并用
`prepare_edit_commit` 执行明确的“保存后离开”。宿主验证过的草稿可由
`restore_edit` 恢复，但该调用不会自动提交。

`Cell` 和 `CellRect` 使用半开区间的可见视图坐标；稳定业务身份始终使用
`RowId` 和 `ColumnId`。宿主处理粘贴或填充时，应保存当时的可见列列表，不能在
异步结果返回后使用已重排或隐藏后的列序号。

## 公共批量规划和预览

`erp_grid_core::edit_planner::{IndexedRow, plan_paste, plan_fill}` 负责纯规划。
`IndexedRow` 提供当前查询中的 `ordinal`、稳定 `id` 和按 `ColumnId` 索引的
`values`；跨页只传实际源段、目标段，不加载中间页。两个规划器返回
`Vec<CellEdit>`，不生成业务动作或发送请求。源/目标限制10,000格、文本1MiB；
基础字段校验整批通过后才返回。横向填充允许源和目标共享同一稳定行。

`erp_ui_egui::ChangePreview::new(id, edits, column_labels)` 构造公共预览。
它使用DataGrid展示修改，支持移除项、撤销和重做；`ChangeItemId` 按原输入
分配，移除不会重编号。每个页面/子表必须传独立ID。

调用 `preview.ui(ui)` 得到 `ChangePreviewEvent::Confirm(edits)` 或 `Cancel`。
Confirm仅是意图：宿主校验业务规则、权限及版本并接受提交后，才调用
`mark_submitted()`。同步保存失败可保留原预览重试；异步提交期间禁止修改内容。
宿主通过 `set_row_results` 提供逐行 Applied/Rejected/Unknown，并以
`set_finished` 标记处理完毕；缺少终态或存在Unknown时不能关闭。
`set_before_overrides` 仅展示失败项的最新值，不修改原请求内容或版本。

`apps/erp-ui/src/remote_paste.rs` 与 `remote_view/paste.rs` 现在是上述公共接口
的商品/SKU业务适配，保留动作名、权限、版本CAS、持久化和重试翻译。
普通页面直接使用公共规划器和预览面板，再编写自己的保存适配；不要复制私有模块。

## 已加载远端行的增量更新

`RemotePages::apply_patches(generation, membership_and_order_unchanged, patches)`
仅更新已缓存的live行。订阅创建时捕获`generation()`；每项`RowPatch`含稳定行ID、
原版本、新版本与变化字段。版本为服务端不复用的opaque标识，按原版本相等检查，
不能把时间字符串按字典序当递增序号。宿主负责授权订阅、有序投递及断流补偿。

宿主只有确认记录仍属于相同查询、位置不变时才能传true；增删、排序/筛选字段、
隐藏业务谓词或派生查询变化必须invalidate并重查。冻结快照拒绝更新。
一次最多1000项/1MiB，重叠缓存页的同一行同步更新，任何版本断链、未知字段、
未缓存行或预算错误均保持整批未应用。失败后停止增量并重新同步，不静默丢弃。

成功回执返回变化行和需要取消的请求ticket，迟到回包已在核心层隔离；宿主仍应
中止对应传输并触发绘制。更新不清除勾选、框选或编辑草稿；正在编辑的行由宿主
显示版本冲突，不能覆盖草稿或假称已保存。UpdateLab已接入该入口和逐帧批处理，
实际网络订阅仍由宿主提供；运行口径见下方resident-grid-update及qa/performance记录。

帧投递使用`UpdateQueue::new(pages.generation())`。接收端调用`push(generation, patch)`，
UI每帧最多调用一次`flush(&mut pages, membership_and_order_unchanged)`，处理回执中的
取消ticket。`pending_count/pending_bytes/error`可供诊断；队列最多1000项/1MiB。
任何非过期生产者错误均停止继续投递，重新加载后建立新队列；失败批次保留但不重试。
禁止忽略push错误、用新队列绕过版本缺口，或在队列前接无界网络channel。

应用成功后，对回执中的每个变化行调用`GridState::notify_row_changed(row)`。
同一行任意字段改变都可能使行版本失效；活动且未提交的编辑器保留before/draft，
显示冲突并禁止Enter/Tab和宿主prepare_edit_commit提交。用户明确重新审核最新版本后，
宿主更新自己的CAS基准并调用`rebase_edit`；也可取消草稿。该方法不会把awaiting操作
改为失败或成功，已提交项仍由原ticket/回执处理。宿主尚需将此接线到实际更新流。

早期更新烟测口径（历史，当前以本页后方resident-grid-update为准）：桌面`erp-grid-lab.exe --grid-updates`，Web在启动WASM前设置
body属性`data-grid-updates`（现有index入口支持`?grid-updates`）。它将本地有序生产者接到UpdateQueue、RemotePages和DataGrid，
逻辑10000条、缓存100条、前10行轮流更新，100次/秒持续60秒。桌面输出
`qa/performance/update-latest.json`，Web输出body的`data-grid-update-result`。
样本为计划时刻到UI文本绘制完成，同帧多次变化可以只绘制最终值；不包含网络或GPU呈现。
该入口用于增量链路烟测，不是完整万行驻留、所有版本逐帧展示或PERF-05通过证明。

资源诊断使用`RemotePages::stats()`，获取缓存块、行、页面负载估算字节、在途请求和
错误记录数量。切换查询/离开页面后，宿主记录返回的取消ticket并中止对应传输，
核心统计归零不代表HTTP任务已经停止。该统计不包含分配器保留、运行时或GPU内存；
PERF-04仍需要结合宿主任务数和真实进程内存测量，不能只看缓存行数。

当前HTTP适配提供`HttpClient::active_requests()`，按客户端实例统计仍存活的JSON请求
future，包含取消后的清理阶段。计数在future退出时减少，不能在点击取消时直接减去。
该值不含下载任务、空闲连接池或已完成但尚未消费的响应；不得据此宣称全部网络资源为零。

首次加载测量入口：桌面`--grid-first-load`，Web`?grid-first-load`。300000个文本单元格
预先物化，数据就绪后计时，每样本创建新的GridState，首帧绘制后下一帧注入egui方向键，
确认焦点移动才结束计时；通过eframe raw_input_hook注入并设定虚拟焦点，独立于OS前台焦点。
预热5次、保留30样本，P95预算500ms。数据生成和网络不在
计时范围内，合成事件不等于系统输入验收，计时到UI处理完成而非GPU实际呈现。
桌面结果写`qa/performance/first-load-native.json`，Web写body属性
`data-grid-first-load-result`。运行结果、构建和设备信息须另行记录；入口存在不算通过。

HTTP分段计时：`completed_duration`是请求创建到传输结束，`completed_at`是同客户端
单调时钟上的结束时刻，`delivery_delay`是结束到首次poll的固定间隔。不得与服务端
时间戳相减或跨进程持久化。商品验收宿主只对当前地址被接受的页建立首次绘制计时，
查询重置时清空；加载耗时悬停展示三段值，未知显示“—”。首次绘制不等于GPU呈现，
这些诊断数据仍需结合百万行、有界分页、乱序回复场景才能验收PERF-03。

PERF-03隔离HTTP夹具：`node qa/performance/page-server.mjs`只监听127.0.0.1:15444。
POST `/page`接受schema=1、offset、limit（最多100），可按列0倒序；其他筛选语义明确拒绝。
逻辑100万条，服务器只生成当前页，不生成/下载全部数据。请求头`X-Fixture-Delay-Ms`
可指定0—2000ms延迟以制造乱序。最多2个在途请求，16KiB请求体，最多保留1000条诊断记录。
GET `/metrics`提供请求/返回字节数、在途数及逐次结果。服务无业务数据库和认证验收功能，
只能证明组件对该分页协议的处理，不能作为实际ERP查询性能或生产授权证据。

分页测量入口：服务启动后运行桌面`--grid-pages`或Web`?grid-pages`。
PageLab复用HttpClient、RemotePages和DataGrid。35轮各发两个100行请求，旧请求延迟250ms，
切换代次后发新请求；刻意保留旧传输，验证即使取消未生效，旧响应也不能覆盖新页。
延迟也支持查询参数`delay_ms`；服务端诊断`sentOrder`用于独立核对发送顺序。
前5轮预热，后30轮从HTTP完成到绘制及合成方向键处理计时，P95预算300ms。
桌面写`qa/performance/page-native.json`，Web写body属性`data-grid-page-result`。
协议错误、错误身份、旧页被接受或方向键未移动均使采样失败。结果不包含服务端延迟、
真实ERP查询性能、物理键盘或GPU呈现；百万是逻辑查询总量，并非客户端驻留行数。

更新测量入口仍为桌面`--grid-updates`和Web`?grid-updates`。当前场景为
`resident-grid-update`：本地实际驻留10000行，DataGrid直接按该数据集虚拟绘制；
其中前10个可见行经公共UpdateQueue与100行RemotePages缓存接收每秒100次更新，
仅按回执同步改变的行，不提高生产缓存上限。持续60秒，逐项版本CAS，结束核对
全部10000行身份、版本和Revision字段；前10行版本600，其余9990行版本0。
采样到UI绘制完成，帧内多个更新允许合并显示；不声称每个中间版本都呈现到屏幕。
桌面仍写update-latest.json，Web仍写data-grid-update-result；旧100行场景结果需另存，
不可混用两个scenario计算趋势。网络和GPU呈现不在此本地生产者测量范围内。

打印HTML按最多8列拆成多个表格：首列在每个列组重复，其余字段按当前列顺序各出现一次。
后续列组从新打印页开始，每组保留重复表头；浏览器表格采用固定布局，长文本在格内换行。
该策略不自动猜测业务主键，宿主应将用于识别记录的列放在首列。重复后的单元格与文本
仍计入100000格/8MiB预算，超限明确拒绝，不能靠分组绕过导出上限。

页面离开时，由宿主调用商品适配的`suspend_page_queries`：取消普通主表/SKU页读取、
查找及选项目录请求，失效页缓存，保留查询条件、勾选、编辑草稿和业务操作回执。
返回页面重新加载当前页；该方法不是清空账号或取消已提交业务动作的入口。
冻结选择、导出任务等独立流程须按各自生命周期处理，不能据普通分页取消推断全部任务结束。

冻结选择离开处理：尚未完成的签发取消并给出状态提示；已有凭据与排除集合保留。
若准确计数正在读取，取消请求并清除未确认数量，返回后通过现有“重试计数”恢复，
在数量确认前批量操作仍禁止。取消客户端签发不代表撤销服务端可能已生成的短期快照，
服务端仍按既有到期回收处理；不创建业务执行任务。

导出任务列表离开时停止轮询/当前列表读取，保留筛选和页码；再次打开重新查询。
用户已启动的文件下载继续交付，主应用每帧处理其完成回执并释放句柄，结果提示保留。
这类明确启动的下载不属于遗留查询；登出/账号切换仍沿用账号清理边界。
服务端导出任务、创建操作身份不因关闭列表而取消或重新生成。

下载进度/完成/取消反馈与任务列表加载反馈分开存储；列表刷新不得清除最近一次下载结果。
显式关闭下载窗口仍按控件提示取消当前下载，并显示取消结果；切换页面则继续交付。
