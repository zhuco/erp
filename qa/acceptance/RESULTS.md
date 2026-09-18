# 隔离 API / Web 操作验收

2026-09-10，Windows / Microsoft Edge，原生 Rust UI 编译为 WASM。
本次是隔离种子账号与真实 API/数据库之间的操作，不是店铺生产数据验收。

- API：`127.0.0.1:15441`，显式连接 `127.0.0.1:15438/erp_grid_acceptance`。
- Web：`127.0.0.1:15440`；固定用途浏览器 profile `qa/browser-grid-acceptance`。
- 统一入口完成 Web debug 与 backend build；wasm-bindgen CLI 0.2.128 与 Cargo.lock 一致。
- 官方 Windows CLI 归档 SHA256：`8fd8e2165da16b21ee3f5efd19e7f97d8d27cb7832f54edaef4b18e830283ec0`，下载后核验再解压。

## 已观察结果

1. 浏览器界面填写隔离账号，登录后查询显示 4 商品；独立 `/auth/login`、`/me` 读回账号、权限和仓库范围一致。
2. 双击 QA-P-001 商品名称，输入 `QA Web saved 2026` 并 Enter 提交。数据库直接读回相同名称、更新时间变化。
3. SKU 子表双击 QA-SKU-001 成本，输入 `26.50` 并 Enter 提交。数据库直接读回 `cost_cent=2650`。
4. 刷新浏览器并重新登录，商品名称和子表成本仍显示上述保存值。截图：`output/playwright/web-product-reopen.png`、`web-sku-reopen.png`。
5. 从商品页打开真实库存报表，读取 4 个余额记录、数量合计 195，图表和分组网格呈现成功。
6. 点击报表 Excel，浏览器实际触发下载。保存为 `output/playwright/inventory-browser.xlsx`；读取 XLSX XML 确认含杯具 180、样品 0、箱包 15、合计 195，折叠状态没有遗漏明细。
7. 从商品页打开仓库树，点击 QA-WH-01 的展开箭头后按需加载 A-01-01 库位；截图 `output/playwright/web-warehouse-tree.png`。自动化点击使用 120ms 按下时长，保证 egui 有机会处理独立帧的按下与释放事件。

## 实测问题与边界

- 初次 Web 中文字体缺失，已嵌入附 OFL 许可证的思源黑体；真实 egui 字形测试通过，浏览器目视确认商品、库存、按钮中文正常（`output/playwright/web-chinese-font.png`）。
- SKU 子表已实际保存命名视图 `QA SKU view`，刷新并重新登录后仍可找到（`output/playwright/web-sku-view-saved.png`、`web-sku-view-reopened.png`）；账号/父商品 scope 隔离通过定向测试，未据此冒充多账号浏览器实测。
- 离开保护初次实测发现导航失焦提前提交，现已在导航按下阶段 hold 编辑。修复后输入 `QA discard must not save`，120ms 点击导航：弹窗选择前数据库未变；点击放弃后切页，数据库仍保留原值。截图 `web-discard-leave.png`。
- 再次编辑为 `QA confirmed save and leave`，点击导航后选择保存：数据库读回相同新值，收到保存结果后自动切页（`web-save-leave-draft.png`、`web-save-leave.png`）。这次才算保存后离开的有效证据。
- 有未提交编辑时执行浏览器刷新，实际触发 beforeunload；取消后编辑文字和当前页面保留。保存清稿后可正常刷新。
- 新增与直接契约共 13 项定向测试通过：字体 1、视图 7、离开/批次 3、提交票据 1、失焦 hold 回归 1；native/WASM Clippy 通过。首次编译接线错误及字体测试的纹理释放问题已修复，不重复运行旧数据库测试。
- 0.51发现页面隐藏期间未消费查询结果会延长显示耗时；0.52商品/SKU已改为后台完成耗时并通过测试，本地分片报表另有计时口径待核。该客户端耗时不代表纯数据库执行时间。
- 初次静态启动脚本误以为 bindgen 保留绝对导入；已根据真实生成的 snippets 相对路径修正并实际启动。
- 当前工具缺少 Computer Use 要求的 node_repl，桌面鼠标/键盘操作未验收，不能用构建或 API 结果代替。
- 以上 debug 小数据操作不证明百万行性能、中文 IME、屏幕阅读器、物理打印或真实店铺发布可用。
- 种子复跑会恢复验收库初值；保留当前修改作为保存读回证据，不自动复跑或清库。

## 0.52 可重复门禁（2026-09-10）

- `scripts/Build-ErpWeb.ps1`通过统一入口完成当前WASM构建及固定版本bindgen打包；生成`erp_ui_bg.wasm` SHA256为`966808AE11447E16C31F073597A51FE3E2A21A7024BCCDC2970C794D1CDEB7B7`。
- 新包实际执行`node qa/acceptance/browser-smoke.mjs`成功：隔离账号登录与筛选、编辑后导航未提前保存、明确保存后API读回、通过网格编辑恢复原商品名称。证据`output/playwright/browser-smoke.json`及`smoke-confirmation.png`、`smoke-saved.png`；脚本没有直接改库或重新播种。
- 三个受影响Rust包测试通过（包含后台耗时及百万行可见AccessKit树）；native/WASM Clippy、fmt、仓库基线、511文件AG Grid禁止扫描和OpenAPI门禁通过。两次首次编译接线错误已修复，未将编译失败计为测试通过。
- 原生AccessKit新增依赖及Web语音功能使两端依赖发生变化，因此本批集中建立三个网格包基线；没有重复运行旧后端数据库用例。后端14个集成目标仍保留，改由统一构建入口执行，尚未重跑完整发布门禁。
- Web语音提示不等于ARIA网格；桌面实际读屏、IME、物理打印、规模性能及真实店铺验收仍未完成。CI已改Rust入口但尚未远端运行。

## 0.53 列布局与填充（2026-09-10）

- 核心75、应用168、渲染器56项不同测试通过；初次2个填充夹具错误修正后只重跑填充集合。保留qa/grid-053-tests.log、grid-053-renderer-tests.log、grid-053-fill-tests.log；前者包含原始失败，不单独据其声称全通过。
- 右固定列位置/点击身份、固定宽度预算、旧视图兼容、分组表头高度及不误排序、横向拖柄已由真实egui帧验证。数列推断受字段类型约束、源轴预计算及溢出拒绝已测。
- native/WASM Clippy、fmt与仓库门禁通过，两端构建成功。新Web包实际完成浏览器烟测并通过界面恢复QA商品原名称；smoke-lab.png目视确认主表/货品子表分组表头、序号及搜索行正常显示。运行结果保存在output/playwright/browser-smoke.json。
- 首次浏览器打开因QA静态服务已停止而连接拒绝；确认旧进程及监听不存在后恢复同一回环端口服务，清理此次打开的浏览器会话后重跑成功。未重启API、未播种或清库。失败打开也会关闭已创建浏览器，避免遗留会话。
- 跨页填充、平台输入/读屏、规模性能及真实订单录入代表宿主仍未完成；当前页面演示与debug小数据烟测不代替这些验收。

## 0.54 性能与订单保存前置（2026-09-10）

- 独立release PERF-02测量详见[性能记录](../performance/RESULTS.md)：Web P95 18.4ms通过，桌面Vulkan 92.4562ms与DX12 94.0317ms未通过。原始数据、硬件、三轮截图及不同口径内存全部保留，不将失败指标改为通过。
- 主实验台分片查询只累计处理时间；单元格文字保持溢出裁剪并允许完全容纳的文字共享区域裁剪。各新增定向测试通过，不能据此声称原生性能已改善。
- 公共HTTP客户端新增显式PUT，保留旧GET/POST入口及超时、大小、取消、鉴权边界。真实回环HTTP直接契约4项通过，包含PUT方法/查询/授权/完整JSON及既有错误/取消；native/WASM Clippy和grid fmt通过。
- `oms_orders::order_item_edit_requires_current_version_and_preserves_items_on_conflict` 在独立数据库 `127.0.0.1:15438/erp_order_cas_test` 通过，验证400/409、成功版本递增和并发唯一赢家。与浏览器验收库隔离，不重播种。首次编译遇旧测试的进程环境变量修改不兼容Edition2024，已移除无必要的全局幂等禁用；无key请求沿正常中间件路径。测试目标重建API可执行文件前只停止身份确认后的本任务QA进程。
- 本记录不表示订单货品UI已接线，也不表示完整后端旧格式/Clippy债务已清除；保留式行修改、代表录单宿主与剩余质量场景继续推进。
- 后续保留式`item_edits`已完成并验证：平台SKU/标题/raw/折扣/实付/退款/售后字段保留；qty-only沿用当前单价，price-only保留数量，组件数量按旧BOM比例精确调整且身份/版本/快照不变。非法比例、金额或组件溢出、缺行/重复/空修改/非法数值整批拒绝，版本与货品不变。
- 本批订单两个不同用例最终通过。首轮新用例在旧版本遇异常比例时得到400而非409，修复为锁订单后先比较版本，再规划货品，两个直接消费者复验通过（2 passed、0 failed、21 filtered，0.55s）。随后补充不裁剪line_id的保护，仅重跑`order_item_partial_edits_preserve_platform_facts_and_are_atomic`通过（1 passed、0 failed、22 filtered，0.49s）；带空格的另一个ID不会误改原行。
- 保存日志新增本次item_edits及expected_version；读取最新详情的版本标为readback_version，避免将并发情况下的最新读回误称为该请求提交版本。完整API事务测试不等于录单页面或生产店铺验收。

## 0.55 订单宿主与跨页填充（2026-09-10）

- 两个低成本子任务分别实现订单宿主和私有冻结填充，主线统一接线、审查和构建。新增夹具orders.sql（12笔订单/24行货品）、cross-fill.sql（24条商品）均为限定数据库/端口/身份的add-only插入；原商品/SKU夹具未重新播种。
- 原生库定向测试共31个不同用例通过：订单4、填充17（含渲染器3）、冻结编解码5、新UTC精度1、离开保护2、实际PG排序1、API请求兼容1。cross-fill模块的3个已通过用例在新增UTC测试时被同过滤器再次覆盖，不重复计数。日志qa/grid-055-*.log。
- 首次订单测试编译发现临时夹具借用生命周期错误；更正后订单4项通过。首次native Clippy发现indexed fill的范围索引循环lint，改迭代器后通过。API第一次过滤器匹配0项，不计成功用例；准确过滤selection_bodies_reject_unknown_fields后1项通过。没有重跑旧订单数据库写入集合。
- 最终native/WASM UI Clippy通过，统一fmt执行成功；OpenAPI门禁与AG Grid禁止门禁通过（518文件）。后端API构建成功，隔离API旧进程23804经身份核验后停止，新进程27044绑定127.0.0.1:15441，仍使用erp_grid_acceptance；没有生产迁移、worker或平台发货。SQLx旧future-incompat提示及旧后端格式/Clippy债务仍保留。
- 实际Edge/Rust canvas：登录、订单主表与货品子表加载、数量2→3的PUT200、读取响应数量3。恢复数量2时route.fetch使服务器提交，再abort客户端响应；界面进入Unknown并保留输入。点击原请求重试返回409，读取最新返回数量2后解除Unknown。独立API最终读回版本3/数量2。证据order-055-unknown.png、order-055-retry-conflict.png、order-055-reconciled.png及order-055-readback.json。后续移除重复重试按钮，不改变已测保存流程。
- 订单真实分页：设置10行返回10条，下一页offset10只返回2条，总数12。订单序号起点随后对齐offset；尚未单独截图最终订单第二页序号，不据代码声称已目视验收。首次详情等待使用了不包含尾部问号的URL条件而超时；修正匹配后功能实测正常。
- 首次商品跨页填充读取冻结source后拒绝“源已变化”，没有发业务写入。实际原因：live默认created_at DESC/product_id DESC，旧冻结页ordinal为id ASC；同一updated_at的文本又分别为Z与+00:00。新增可选query_order默认false兼容旧分页，true对齐live排序，UI以完整精度比较等价UTC版本。实际PG测试覆盖时间并列和显式字段并列。
- 修复后当前商品页只加载10条，以QA-FILL-024的Brand为源，填充查询序号11–12（QA-FILL-014/013）。请求实际仅offset0/limit1、offset10/limit2，query_order=true；预览分别展示brand14/13→brand24。确认前API读回原值；明确确认后两个products.update动作200，API读回两个新值，预览显示成功2/失败0。
- 通过第二页单元格编辑恢复brand14、brand13。恢复脚本最初第二次编辑早于第一次保存后的刷新完成，Control+A落在网格选择而非编辑器，未发第二次写入；待刷新完成后只重做该行，恢复成功并清除测试勾选。最终独立API确认两个品牌均为原值。证据fill-055-ready-preview.png、fill-055-saved.png、fill-055-target-page.png、fill-055-restored.png、fill-055-readback.json；qa/grid-055-fill-browser.log含确认前后断言结果。
- 最终WASM及API哈希记录在fill-055-readback.json；本任务浏览器erp-order-smoke已关闭。截图初始淡入帧另保留，验收采用ready/saved/restored稳定画面。
- 边界：订单重启草稿、命名视图和列内快捷筛选未完成；顶部keyword不是列内搜索。SKU没有等价冻结跨页填充。桌面实际输入/读屏、IME、物理打印、原生性能与其余规模场景仍未验收，整个组件目标保持未完成。
`scripts/Invoke-ErpBuild.ps1 -Action build -Scope grid -Platform native -Release`最终通过（39.11s）；WASM release打包通过。产物SHA256见build-055.json。原生链接器输出创建导入库的信息被Rust显示为linker_messages warning，不是链接失败；未启动新桌面包做系统输入验收。

## 0.56 订单列筛选及账号视图（2026-09-10）

- 默认展开订单号Text、平台/OMS状态Enum；平台4值、状态9值均核对erp-types枚举。查询通过既有API keyword/platform/oms_status，后两者CSV多选，不筛当前页。输入变化取消旧任务和结果、250ms自动查询；中文Preedit期间等待Commit。空结果和错误使用list_initialized防止每帧重发。金额列保留，FilterKind::None明确无筛选能力；快捷行不绘制控件，组合条件编辑器不提供该列，非空历史草稿compile报错。
- 订单Views复用现有账户存储，单独orders-schema1/main；商品默认scope不变。视图校验scope、列、筛选类型、页行数和不支持设置后原子应用；不接受静默丢弃排序/表达式。dirty/pending/Unknown时禁止切查询。
- 定向测试14项：orders8、none_2、remote_view::views::tests4。各package未匹配的0项不计数。首轮订单测试提示ImeEvent::Disabled已弃用，去除该分支后native/WASM Clippy均通过（5.37s/4.27s）。WASM release打包33.84s通过，SHA256 4FF737C1DA65D5F48FFEC9BA0504DE5B5D7F802A5A816C96D1D6EA072226FFF1。未重建后端、未重跑旧写入集合。
- 实际Edge/Rust canvas（erp-order-056）：隔离QA账号读取12订单；输入QA-NO-MATCH返回0，随后等待90次双RAF，订单查询计数仍1；输入QA-ORDER-001返回1。保存QA order 001视图到orders-schema1/main key；清空条件回到12条，再Apply恢复QA-ORDER-001一条。持久化值包含列宽/显示/固定、筛选、页行数；其他页面store数量0，没有覆盖其他页面。
- 浏览器脚本两处观察错误如实保留：首次等清空URL完全不含keyword，但API实际发送keyword=，超时不代表查询失败；首次apply观察用URL全局在CLI隔离环境不可用，改已确认URL字符串匹配后通过。证据qa/grid-056-filter-browser.log、view-inspect.log、view-apply-fixed.log、view-storage.log，截图output/playwright/order-056-loaded.png、saved-view.png、view-applied.png。初始淡入截图不作为视觉验收。浏览器已关闭，无业务数据写入。
- 边界：本轮未实测物理中文IME、浏览器重开后重新登录应用视图或桌面输入；持久化scope隔离有纯测试及实际storage证据。订单恢复drafts.rs正在接线，不能计为已交付。当前native执行文件仍为0.55；本轮native证据为Clippy而非新桌面运行。

## 0.57 订单金额与重启恢复（2026-09-10）

- 后端新增total_amount_min_cent/max_cent非负i64分单位，包含边界，min>max拒绝；count/data在分页前共用谓词，旧精确金额筛选兼容。OpenAPI源及bundle一致。UI金额列使用Money双输入和金额类型；非法输入保留且不派发查询，视图应用前同样校验。
- 恢复按server/tenant/actor固定单槽，记录订单/稳定货品行、before/after、版本及Unknown原path/body；无token。复用已有SQLite synchronous Extra与IndexedDB strict事务，StorageTask返回提交结果后才PUT。eframe原生Storage.flush经源码确认只启动后台写入，因此未将其内存readback冒充磁盘确认，也未把Unknown放入会自动删过期数据的普通DraftStore。
- 独立order-recovery-v1 key复用checkpoint存储，无自动TTL删除；清除写入有版本null标记，阻止旧窗口ABA覆盖。恢复槽与现有action checkpoint共享32槽上限，达到上限保留已有数据并报错。账号变化不会重绑定未确认请求；损坏或提交不确定时阻止派发。GridState::restore_edit只建立可审阅编辑器，不自动提交。
- 定向测试20项：API amount_range_tests 1、repo order_filter_tests 1、orders 16、实际SQLite关闭重开/CAS/clear-ABA 1、renderer恢复编辑器1。第一轮orders测试有helper scope被局部变量遮蔽的编译错误，更正后16项通过；0匹配包不计数。后续发现的恢复残留状态缺陷以真实浏览器收尾路径复验。
- API test 2m11s，API debug build 1m44s。旧隔离进程27044经可执行文件/监听端口核对后停止，新进程28720监听127.0.0.1:15441，同一erp_grid_acceptance夹具，没有新seed/生产迁移。只读脚本qa/acceptance/order-amount-readback.ps1通过：1250..1250总12、第一页10/第二页2；max1249/min1251均0；倒置、负数、非数字均400。OpenAPI门禁通过。
- 实际Web金额框：13返回0、12.50返回12、上下都12.50返回12，直接输入自动请求；证据qa/grid-057-amount-browser.log、output/playwright/order-057-detail.png。
- 首轮Web恢复：初始订单数量2/version3。PUT发出时先读取IndexedDB，断言已存在同版本Unknown checkpoint；服务器保存数量3/version4后丢弃响应。刷新重登自动读取原订单、恢复Unknown，无自动PUT（requests日志#23–30仅认证/读取）；明确读取最新后得到数量3/version4，恢复槽清为null/revision3。reload的beforeunload使CLI提前报告modal，后续脚本仍完成；用实际截图与网络记录补证，不只依赖未返回的Result。
- 未发送草稿：输入数量2但不Enter，IndexedDB为Unsent；刷新/重登后输入2仍在，服务器仍3且自动PUT计数0。明确Enter保存数量2/version5。此时最后清理检查失败，发现成功响应accept_detail把本进程刚提交的Unknown checkpoint再次恢复；没有把这次结果计为完整通过。
- 修复：只有open加载的snapshot需要恢复；本进程commit ack标记已应用，成功保存先清Unknown再接受详情，避免恢复旧编辑器。仅重验正常保存收尾。首个自动化尝试键盘早于编辑器就绪，Control+A落入网格且没有PUT；改为F2并等待实际INPUT/TEXTAREA焦点后，数量2→3/version6→2/version7均PUT200，每次恢复槽均null（revision3/6）。证据qa/grid-057-final-save-browser-fixed.log、output/playwright/order-057-final-clean.png。
- 最终独立API读回数量2/version7，保留平台事实，不回退版本。产物哈希见output/playwright/order-057-final-readback.json。最终native/WASM Clippy、fmt、WASM release（28.86s）及native release（42.48s）通过。AG Grid门禁521文件通过。原生linker_messages只是生成导入库的信息；旧后端SQLx future-incompat/全量格式债务仍保留。
- 所有本轮浏览器已关闭。没有进行新桌面包的键鼠、物理IME/读屏或打印验收，原生性能问题也未因此解决。整个组件目标保持未完成。

## 0.58 SKU 跨页填充集成（进行中）

- 当前交付范围已按用户纠正为网格组件；停止扩展 ERP 业务 UI，保留现有最小验收宿主。
- SKU query-ranges 使用单次 repeatable-read/read-only 事务读取源与目标段，共用完整查询和稳定排序；最多两段，每段1000行。UI限制目标1000行/10000格/8MiB响应，源身份与完整UTC版本复核后只生成PasteReview，确认前无业务写入。
- 子任务审查发现取消仅清任务导致cross_fill残留并锁住分页/筛选，已修复为当帧移除；当前查询无效也取消并解除锁定。父页面认证失效沿已有expires路径处理，保留恢复状态。
- grid details过滤器实际11项通过，repo范围边界1项通过。首个API过滤器误用了文件名作为模块名，运行0项，不计通过；应使用函数名sku_ranges_validate_bounds_and_pass_fixed_query_to_repo。
- SKU改动双端Clippy通过；最后取消修复已通过原生Clippy。后续性能诊断字段改动及最终双端集成仍待统一检查。实际SKU跨页浏览器操作尚未验收，不能将接线称为完整交付。
- API函数名过滤器更正后实际1项通过；隔离PG临时表用例1项通过，含重叠取段、完整查询及稳定排序。实际只读接口脚本通过默认/指定排序、金额完整筛选、父隔离与4类非法范围400，见qa/grid-058-sku-ranges-readback.json。
- 实际Web首次打开SKU范围窗发现与商品范围窗共享ID导致不可见，已按实例分配独立ID。准备时的网络挂起脚本使用了CLI隔离环境不存在的setTimeout而报错；请求仍待完成，此后明确取消并实测能翻到第二页、回到第一页，再正常准备。未把脚本错误计为通过。
- 实际当前SKU页10行，用QA-SF-024的Name填充查询序号11–12，实际只读请求为offset0/limit1与offset10/limit2；预览显示014/013旧名称到name24。独立API确认预览前未写入，显式确认后两次skus/actions HTTP200且两行新值正确。随后通过网格F2编辑恢复name14/name13，最终独立读回和哈希见output/playwright/sku-058-readback.json；取消/预览/保存/恢复截图与qa/grid-058-*-browser.log保留。
- 本批实际14个Rust定向用例及8个独立性能校验器用例通过；0匹配不计数。双端Clippy通过，Web release已包含窗口ID修复；后续桌面默认呈现策略构建单独记录。AG Grid门禁522文件、OpenAPI门禁通过。SQLx历史future-incompat警告仍在。
- 原生同构建AutoVsync/AutoNoVsync对照分别P95 85.3637/7.7168ms，后者通过32ms独立门槛。此结果推动桌面与测试默认采用低延迟呈现，不能据此声称物理输入、无撕裂、读屏、打印或其余规模场景完成。详见qa/performance/RESULTS.md。
- 0.58最终native release（38.20s）及native Clippy通过，fmt检查通过；产物与默认策略记录在qa/acceptance/build-058.json。最后WASM构建30.74s且实际浏览器验收使用该包；API沿同隔离库重启，未重新播种。两个性能进程自行完成退出，验收浏览器已关闭。

## 0.59 公共批量规划与预览（本批完成）

- 两个低成本子任务分别整理core规划器、egui预览面板；服务端额度限制中断后由主线接管，未继续重试耗额度的子任务。
- 新edit_planner和ChangePreview均已用于现有入口；本地TableView、商品/SKU远端适配不再维护重复数列/坐标/预览UI。版本CAS、权限、持久化和重试仍留宿主。移除已被替代的preview_history模块。
- 接管首轮编译发现旧测试导入和paste_retry字段引用未更新；补齐后核心79项通过，应用193项通过/1项按钮文案定位失败。修正定位后该部分失败重试用例实际通过，仍断言两行独立命令、部分失败、新版本重审和原值保持；不是删除断言规避失败。渲染61项通过。
- 主线补齐：取消Unknown收尾限制、标签/结果/显示值预算、万行粘贴ordinal索引及按行命令索引，避免二次扫描；源文本也计入1MiB；移除/撤销/重做清除旧坐标选区。双端Clippy通过，初次两个样式lint按建议修复，没有全局忽略。
- 最后选择清理修复后，ChangePreview定向2项通过；最终Web release 29.86s、native release 46.83s通过。AG Grid门禁523文件通过；产物SHA256见build-059.json。本轮没有修改后端或重播种数据库，未重复运行已通过的旧业务写入集合。
- 实际Edge本地Lab万行夹具：将Product 1填充到Product 2/3，预览显示两项；勾选移除Product 2项后只余Product 3，撤销/重做后确认，主表Product 2不变、Product 3变为Product 1。主表Undo恢复Product 3，截图已目视核对。证据output/playwright/preview-059-removed.png、preview-059-undo.png、preview-059-applied.png、preview-059-restored.png。本地夹具未写业务库，独立浏览器erp-preview-059已关闭。
- 收尾脚本首次误用run-code参数解构导致page未定义，未执行鼠标操作；按CLI的page参数修正后执行成功。没有将该错误计为通过。未进行物理输入法、读屏、打印或其余规模场景验收，完整组件目标仍未完成。

## 0.60 高频更新基础（进行中）

- RemotePages新增有界、原子的已缓存行版本更新，重叠缓存副本同步修改；成功后取消全部旧在途请求，迟到回复不能覆盖。无版本比较排序、静默遗漏未缓存行或自动业务保存。接口条件及限制见组件规范0.60与GRID_INTEGRATION.md。
- 3个不同定向测试通过：版本链及旧请求/重放隔离；整批失败不修改及旧scope拒绝；重叠副本、超限和冻结页拒绝。首次2项通过后只运行新增第3项；其他包0匹配不计数。
- 固定构建入口fmt通过，native Clippy 6.37s、WASM Clippy 4.49s通过。未生成新的release包，运行包仍是0.59；未操作数据库或重跑旧业务保存测试。工作目录无.git，无法提供Git差异。
- 尚未接高频推送、逐帧投递和编辑冲突呈现，PERF-05仍未完成；该批只提供公共缓存应用契约，不当作高频渲染或整体组件验收。
- 接续增加公共UpdateQueue：有界帧投递、保留逐项版本和字段、过期生产者隔离、溢出/断链错误锁定，成功应用才清空。5项更新模块测试通过（新增队列2项、共享校验改动直接回归3项，24.72s）；不把其他包0匹配计数。真实渲染宿主尚未接入，不能将队列测试替代PERF-05。
- 队列补充后fmt、native Clippy 5.97s及Web Clippy 4.43s通过；没有新增release或运行验收声明。
- 补充GridState外来行更新通知：保留未提交草稿与原值，显示冲突并阻止键盘和宿主提交；显式rebase后恢复，awaiting仍按原ticket处理。1项实际egui事件定向测试通过（8.53s构建、0.02s测试），覆盖Enter不发编辑、prepare_edit_commit拒绝、原输入不变、重审基准变化及原回执可解析。格式检查/写入与renderer native Clippy 16.13s通过；更新流宿主尚未接入，不宣称真实推送端到端通过。
- renderer Web Clippy 20.73s通过。双端检查仅针对受影响渲染包及依赖，未重跑全量测试或生成release。
- 后续UpdateLab已实际接入公共队列、缓存、行变化通知与DataGrid。首轮绘制采样尝试修改Fn捕获数组导致编译错误，改为Cell位掩码后native Clippy 3.30s、Web Clippy 3.22s通过。native release 49.57s通过；linker_messages仅生成导入库的信息。
- 桌面本地更新烟测进程24512/session66135正常完成退出：100Hz持续60秒，6000计划事件/6000绘制延迟样本，10行最终版本全部600，无error，queue_pending/queue_bytes均0。独立PowerShell读回断言以上数量和版本。计划时刻到UI文本绘制完成P95=0.8461ms。原始数据qa/performance/update-latest.json，逻辑10000行而缓存100行；不包含网络、GPU呈现、逐个中间版本展示或完整PERF-05验收。未启动其他构建干扰采样，未操作业务数据。
- Web release 34.98s通过，index新增?grid-updates启动开关。独立Edge会话erp-updates-060运行60秒，6000事件/6000延迟样本、10行最终版本600、队列0项/0字节，无error；独立PowerShell解析并核对。绘制延迟P95=17.10ms，原始qa/performance/web-updates-060.json、运行日志及output/playwright/updates-060-running.png保留。运行截图目视确认10行版本持续变化，浏览器已关闭。控制台favicon404及Windows powerPreference提示不计WASM失败。
- 此次桌面/Web均为同一百行缓存、本地有序生产者链路；不把它扩大为完整PERF-05。尚无真实网络、GPU呈现或万行驻留测量。
- 接续补充RemotePages::stats与20轮核心资源生命周期检查：每轮加载12个100行块触发淘汰、注入1个错误和2个在途请求，clear返回2个取消ticket且全部统计归零；40个迟到回包均Stale且不恢复缓存。1项定向测试通过（构建28.87s、运行0.04s），fmt及native/Web Clippy 7.02s/5.13s通过。其他包0匹配不计数。该测试不包含真实HTTP任务和进程内存，不作为完整PERF-04验收；未重建release。
- HttpClient新增按实例活动JSON请求future计数，生命周期守卫在实际future退出时递减，覆盖取消清理，不在点击取消时提前归零。20轮真实回环TCP连接测试覆盖10次cancel和10次Drop，服务端不发送响应，均观察到EOF/reset且客户端任务数归零。初次测试因Windows accepted socket继承nonblocking导致立即WouldBlock，显式改blocking/read-timeout后仅重跑该项通过（7.40s构建、0.13s测试），未放宽EOF/reset断言。native Clippy 5.36s通过。该计数不含下载、池化空闲连接或待消费响应；Web取消的实际浏览器运行仍未据此验收。
- HTTP计数改动Web Clippy 3.58s通过；未为该诊断改动重建release，运行包仍为更新烟测构建。
- Web实际查询取消：独立Edge erp-cancel-060使用既有隔离QA账号进入商品宿主，Playwright拦截只读POST /products/query使响应保持待定；连续20次修改搜索，每次按对应旧Request身份等待requestfailed，全部ERR_ABORTED。最后清空搜索额外取消1个旧请求，共21次；解除拦截后收到真实查询200。日志qa/acceptance/web-cancel-060.log保留精确请求失败数组。捕获20个route，最后一项可在进入拦截回调前取消，不能把route数当请求数。
- 浏览器已关闭，全部仅认证和查询，无业务写入。收尾截图output/playwright/cancel-060-final.png仍是加载重绘帧，不作为最终数据显示完成证据。之前等待误假设GET及/products?两次超时，查源码后改为实际POST /products/query；没有把等待脚本失败计作业务故障。此实验验证浏览器AbortController→请求取消，不证明服务端执行已终止、活动计数或整体进程内存稳定；完整PERF-04仍未完成。
- Web切页资源采样：独立Edge erp-memory-060，1280x800，Lab万行→Reports→Warehouse tree→Lab；预热5轮后20轮，每页12个RAF后继续，每轮显式GC再采样。WASM容量28508160字节固定，DOM节点20/监听器55固定，JS保留堆约2.42→2.44MiB有小幅增长。截图确认三个真实页面确已切换。原始qa/performance/web-memory-060.json/log；该测量不包含完整进程RSS或GPU内存。
- 同一会话20轮不切页对照、同样GC和观测，JS堆2554700→2554688字节基本不变，不能把切页增长直接归为采样开销。进一步20轮切页的分配采样主要指向wasm-bindgen生成胶水makeMutClosure的real回调（约10456字节抽样保留），尚未定位保留链或确认无界泄漏；未修改生成代码或贸然升级依赖。保留web-memory-060-control.json、profile.log和web-memory-060.heapsnapshot供继续定位。独立浏览器已关闭，无业务写入，完整PERF-04尚未通过。
- 内存增长后续定位：新增只读inspect-heap.mjs，排除弱边输出最短强保留路径、同一isolate前后对象ID及类型/名称浅大小差异。旧快照80个JS数组主要是固定WebGPU枚举及Playwright辅助数据；不能由回调分配位置直接推断闭包泄漏。
- 独立会话erp-memory-diff-060预热5轮，保存同一浏览器切页20轮前后堆快照（两个快照采完才下载，避免下载影响差分）。web-memory-060-diff.json显示普通object/Array/closure无净数量或浅大小增长；主要新增为WasmNativeModuleTag +16040字节、InstructionStream +14336字节、浏览器DOM包装弱引用表容量+12288字节及代码元数据。保留路径指向已存在的模块/执行代码和浏览器内部容器，未发现需要修复的业务对象累积证据。不是宣称所有内存无泄漏或容量永久有界，完整RSS/GPU/双端PERF-04仍未验收。浏览器已关闭，不改生成代码、不升级依赖、不重跑编译。

## 0.61 首次加载组件测量

- 新FirstLoadLab与native --grid-first-load / Web ?grid-first-load入口，物化300000文本单元格，5次预热后30次新GridState测量。先绘制再验证下一帧合成方向键焦点移动，虚拟焦点独立于OS前台，不声称物理输入。
- 首次native release采样未响应输入，报告budget_met=false及0样本，保留qa/performance/first-load-native-failed-input.json。原在ui阶段插入事件方式改为eframe原始输入钩子并明确虚拟焦点；未降低断言。尝试debug构建被现存进程占用debug/erp-grid-lab.exe阻止，没有终止进程或另建缓存，使用既有release构建路径继续。
- 最终native release 41.15s，Web release 37.19s，双端Clippy 2.91s/2.30s及fmt通过。两端实际各30样本：native P95 4.8449ms，Web 36.50ms，均小于500ms；每样本都验证方向键改变焦点。PowerShell独立排序取第29/30样本确认门槛与数量，原始first-load-native/web.json、构建SHA/设备first-load.metadata.json保留。Web erp-first-061已关闭，native自行退出。
- 这是复用应用/字体和数据后的组件数据就绪时间，不包含网络、数据生成、OS输入或GPU呈现，不能冒充冷启动整应用或整体验收。PERF-03/04/05完整场景与平台交互剩余项未据此通过。
- 接续补齐HTTP完成单调时刻和首次poll等待，原37ms请求耗时与人为设置的2秒交付等待在定向测试中分别保持，不因重复poll改变。商品宿主当前页接受→首次绘制分段，查询/身份变化清空计时，原加载信息悬停展示，未知为“—”。1项定向测试通过（19.12s构建），fmt、native Clippy 3.91s通过；未运行百万行场景、未重建运行包，不能据此认定PERF-03完成。
- 分段计时Web Clippy 2.81s通过。
- 为PERF-03增加独立loopback HTTP夹具page-server.mjs，逻辑百万条、服务端按页生成最多100条，2请求/16KiB输入/1000条诊断预算；延迟头制造乱序，关闭连接释放计数并清timer。只读实际HTTP验证第一页ID1、末页末项1000000、全局倒序首项1000000、limit101拒绝400；两个并发请求offset200先于100返回。6请求共返回30182字节、active归零，见qa/performance/page-server-readback.json。node语法检查通过，无Rust改动或数据库写入。尚未接实际网格测量，不能称PERF-03完成。夹具PID32860/session12298监听127.0.0.1:15444供后续接线。

## 0.62 HTTP分页组件采样

后续0.63受影响包native/Web Clippy均通过，也覆盖0.62的Web参数平台限定修正；不追补此前丢失的构建退出码。

- 新PageLab与双端启动开关接实际HTTP、公共缓存及DataGrid。每轮保留延迟旧传输并切代次请求新页，严格核对响应总量、页长、身份、版本及焦点移动，错误不能充当成功的过期响应。
- native Clippy通过，release 43.51s完成；Web检查首次发现桌面关闭窗口参数未使用，改为平台限定处理。后续Web构建终端输出在上下文截断时丢失，故不填写不可核实的耗时/退出码；20:42:48生成的WASM实际运行新入口成功，哈希已保存。
- 双端各35组旧/新请求、30有效样本，native P95 2.8932ms、Web 31.3ms；服务端逐组sentOrder独立核对新响应先发送，客户端各拒绝35个旧响应，结束缓存100行、HTTP为0、无错误，客户端字节数与服务端发送字节数一致。证据见qa/performance/page*.json及page-web.log。
- 浏览器首次使用不支持的edge别名启动失败，改为CLI支持的msedge后采样成功；会话已关闭。未修改业务数据，没有重复旧保存测试。当前无.git，无法提供Git差异。此结果仅覆盖合成服务组件场景；真实ERP性能、物理输入、GPU呈现与其他剩余性能项未据此通过。

## 0.63 万行驻留更新

- UpdateLab实际驻留万行，DataGrid绘制整个本地数据集；公共队列/远端缓存不扩容，按变化回执同步行。最终核对万行身份、版本和字段值。
- fmt、native/Web Clippy通过（2.94s/2.22s），native release 42.20s、Web release编译28.54s及打包退出0。双端各60秒、6000样本，无错误、队列归零；P95 native 0.96ms、Web 17.0ms，独立排序复算通过。原始报告、采样内存及构建哈希见qa/performance/update-resident*。
- 未修改业务数据或提高缓存预算，未重跑无关业务测试。此为本地生产者到UI绘制结果，不含网络/GPU呈现或物理输入；完整生命周期与平台验收仍未完成。

## Web IME query integration (2026-09-10, current 0.63 WASM)

- Edge CDP Input.imeSetComposition/insertText exercised actual eframe Web input and authenticated, isolated product-query HTTP. Global search composed `zhong` for 600ms without a query, then committed 中文商品 with exactly one complete query. Cancellation composed `quxiao` for 450ms without a query, cleared composition, then typed QA-FILL with exactly one query. Product quick-filter composed `shangpin` for 600ms without a query, then committed 中文商品 with exactly one filters[1].Text query.
- Independent PowerShell assertions checked counts and exact final request fields. Evidence: web-ime-064.json (WASM hash and method), web-ime-064.log, web-ime-cancel-064.log, web-ime-filter-064.log. The committed-text screenshot was inspected and rendered Chinese correctly. Browser session erp-ime-064 was closed.
- No business records were edited. No source changes or builds were required. These are browser composition events via CDP, not physical Windows IME candidate selection or native desktop input acceptance.

## Browser component CI gate (2026-09-10)

- Added component-smoke.mjs, reusing Playwright CLI and existing PERF-01 WASM input path without API/data dependencies. On current release WASM, actual Edge run completed 30 rendered synthetic-keyboard transitions with no pageerror. Result and raw log in output/playwright/component-smoke. No Rust rebuild was required.
- Added exclusive run lock and owned-server cleanup. Targeted occupied-port failure then verified exit nonzero, lock release and existing static service still returning HTTP200; no second success/performance run was needed. Lock failure does not close someone else's browser session.
- Windows CI now installs checksum-pinned wasm-bindgen 0.2.128, packages via Build-ErpWeb.ps1, runs component browser gate and uploads artifacts. YAML parse and gate presence checked. Archive checksum matches the existing local versioned archive; CI download/remote run has not occurred in this workspace. Build disk/cache admission remains unchanged; insufficient runner space fails rather than bypassing it.
- CI timing is recorded without claiming reference-device performance acceptance. Real API save smoke, physical desktop input and full lifecycle checks remain separate.

## 0.65 wide-table print fix

Follow-up: local edit/export Chinese roundtrip now verified on the unchanged Web build. After double-click, wait for editor focus, then select/insert/Enter with animation frames between events; after the query applied, actual downloaded HTML contained the exact full Chinese/ASCII string. Independent extraction checked15 bands and no horizontal overflow; screenshot reviewed. See print-066-chinese.log/json and output/playwright/grid-print-066-chinese.html/png. This closes the earlier missing Chinese roundtrip evidence. The earlier immediate multi-event script did not establish that the editor had focus, so it is not proof of an applied-value export loss; fast-event handling itself was not separately certified. No API/business data writes or new source fixes; owned browser closed.

- Actual Lab HTML download contained 100 rows x100 columns; print media at1040px had table width2168.65625px and unreadable vertical character wrapping. Original HTML, screenshot and print-065.log retained.
- Shared export now emits bands of at most8 columns, repeating first column, all other fields exactly once; fixed layout and page break before later bands. Repeated cells/text count against100000/8MiB budgets.3 print tests passed (16.98s build,0.02s run), native/Web Clippy3.14s/2.19s andfmt passed. Web release29.55s plus packaging exited0. Desktop binary not rebuilt.
- Actual new WASM download produced15 tables (14x8 columns, final2), each100 rows and1040px wide with no horizontal overflow. Browser screenshots reviewed. Attempted local grid edit did not appear in export, so Chinese roundtrip was not claimed; a clearly labeled post-download DOM text fixture verified Chinese/long-token wrapping using unchanged generated CSS. Unit coverage separately checks Chinese escaping. No business API or data writes.
- Artifacts output/playwright/grid-print-065[-fixed].html, screenshots andqa/acceptance/print-065*.log. Browser sessionclosed. This is print-media CSS layout, not actual paginated PDF, repeated headers on physical pages or printer acceptance.

桌面打印修复产物同步：统一入口native release 48.45s完成、退出0，构建哈希见build-066.json。未启动新的桌面交互验收，不把构建成功等同物理打印通过。

## 0.67 host leave read cancellation

- Found GridLab.apply_destination only closed the Orders host. Added Products leave hook cancelling ordinary page reads, find and option reads, plus SKU page/find reads; invalidate page generations/cache and reset IME composition while retaining query/selection/editor/action state. No write or receipt workflow is cancelled by this method.
- fmt, native/Web Clippy3.65s/2.78s passed; Webrelease34.29s and packaging exit0. Actual Edge20 cycles held product query then switched toLab: each exact Request failed ERR_ABORTED; returning toProducts made a fresh HTTP200 query. Independent log check passed. Evidenceweb-leave-067.log/json. Owned browser closed; no database writes.
- Initial startup screenshot was blank because WASM initialization had not completed; waited for actual login UI before entering credentials. This was not counted as a successful interaction. No unrelated save tests repeated.
- This verifies main product paging cancellation, not all SKU/frozen/export/action tasks or full RSS/GPU lifecycle. Desktoprelease still atbuild-066 until remaining lifecycle changes are consolidated.

## Frozen-selection leave cancellation (after0.67)

- Added FrozenSelection.suspend_reads to Products leave hook. Pending issuance is cancelled and pending selection cleared with a notice; pending count is cancelled and unconfirmed count invalidated, retaining existing credential/binding/revision and matched-selection state. Existing retry-count button reestablishes the authoritative count before actions become enabled. No business action/receipt is cancelled.
- One targeted test passed (29.92s build,0.01s execution), checking pending-count cleanup, credential/binding/revision retention, no usable count, retry notice, and issuance cancellation. It uses loopback task handles and does not certify server cancellation or nonempty-exclusion UI retention. Native Clippy4.89s passed. No release rebuild or repeated browser paging test in this batch; binaries still reflect prior recorded builds.
该改动Web Clippy3.93s通过；接入说明与路线图已同步。

## Export lifecycle follow-up

- Products leave now calls Exports.suspend_reads, cancelling the list task, closing the list and marking it for reload while retaining pagination/filter/context and creation state. Download delivery is polled by the application-level tick even when Products is hidden; completion drops the handle and retains feedback. Existing account clear remains authoritative. No server export cancellation added.
- One targeted list lifecycle test passed (10.67s build,0.00s test), checking task cleanup, reload-on-open and context/feedback retention. Native/Web Clippy3.16s/2.46s andfmt passed. The test does not exercise file download completion, account transitions or complete resource lifecycle. No release build or repeated main-query browser test in this batch.

## Frozen selection actual Web recovery and consolidated builds

- Built native release56.77s, Webrelease34.41s and successful packaging, including the latest lifecycle changes. Build hashes recorded inbuild-068.json.
- Actual Edge isolated selection:29 matching records, exclude first2. Held authoritative count request, left Products forLab, exact Request reported ERR_ABORTED. Return screenshot showed same2 unchecked rows, count unavailable and bulk buttons disabled. Explicit retry sent unchanged excluded IDs, HTTP200 affected27; UI showedChecked27. Independent PowerShell comparison validated both exclusion arrays and count.
- Initial capture predicates incorrectly required URL suffixes without trailing query text and timed out; selection itself had already activated and first exclusion had counted normally. Corrected route/predicate to include query suffix, then performed the held request. Earlier timeouts not counted as success. Browserclosed; no copy/export execution or business-row modification, only isolated expiring selection snapshot issuance.
- Evidencefreeze-068-cancel-fixed.log,freeze-068-recount.log,build-068.json andreturn/recount screenshots. This verifies nonempty-exclusion preservation across cancelled counting; cross-page download delivery and completePERF-04/native physical input remain separate.

## 0.69 frozen-scope closeout

- Fixed download result feedback being erased by export-list reload on returning to Products. Download status now has separate storage; explicit cancellation retains its own result. Targeted lifecycle test, native/Web Clippy and Web release passed in the preceding change batch.
- Actual Edge cross-page download completed using a disclosed route fixture and Blob fallback; returning/reopening the list displayed completion. Final CSV independently matched the exact expected Chinese text and line endings. Evidence: download-069-fixed.log, output/playwright/download-069-fixed-return.png and download-069-fixed.csv. Earlier missing-feedback evidence is retained. This is not a real export Worker, large file or native picker test; no business-row writes.
- Native release synchronized via Invoke-ErpBuild.ps1: exit0,40.66s. Linker reported its import-library creation warning. SHA256 for native/WASM and final CSV recorded in build-069.json. No unrelated tests repeated and no existing process terminated.
- Resource evidence reviewed against current leave hooks: ordinary/SKU reads, frozen reads and export-list reads have cancellation paths; application tick handles download completion. Existing browser cancellation and local-memory samples cover separate scenarios, not complete PERF-04. Full native/GPU/all-subflow lifecycle remains unverified.
- Scope frozen. Remaining actual-device, full lifecycle and remote-CI checks are recorded in MANUAL_CLOSEOUT.md with actions and pass criteria. This record does not claim all acceptance completed. Current workspace has no .git; no Git diff or remote CI evidence is available.

## 0.70 bounded component audit fixes

- Fixed signed numeric TSV/CSV using a shared typed spreadsheet_value helper; text formula protection remains. Added Date/DateTime validation/calendar, Choice dropdown and Entity search with full UUID identity, debounce/IME, bounded options, request/editor/instance identity and existing save tickets. Added optional visible-cell Text/Badge/Image/Action presentation and ERP-internal components facade; existing TableView consumes it.
- Initial targeted 7 tests passed. The grid baseline then passed core86 and application197 but failed2 navigation cases assuming Warehouse was the final editable column. The fixture now explicitly configures Related SKU read-only for those scenarios;3 navigation tests passed. Renderer69 tests passed, including calendar pointer input, entity keyboard commit/conflict, image draw bounds and raw-copy preservation.
- Extended action coverage for Ctrl click: the first test harness attempt used a removed RawInput field; the next omitted the required egui ModifiersChanged event. Correct event injection passed (1 targeted test), proving Ctrl checks the row without firing the action or replacing the copy rectangle. No production workaround was added for the faulty harness.
- Native Clippy first found2 default-initialization test style issues; fixed without suppressing lint. Native/Web Clippy passed (1.97s/4.90s); final renderer Clippy after the extra modifier test passed3.00s. Format applied through the shared entry. No repeated full performance or business-save suites.
- Web release40.09s and packaging succeeded; native release1m00s succeeded via the shared build entry. Native linker import-library informational output remains a warning. Hashes recorded in build-070.json.
- Actual Edge Lab: calendar changed first-row timestamp to2026-01-02 00:00:00; Choice changed Ready toHold; searching option12 then Enter/Enter selected and committed SKU option12; price changed to-1.25. Open on row2 selected its bottom details. Actual downloaded100-row CSV independently read back timestamp,Hold,-1.25 and Related SKU UUID00000000-0000-0000-0000-0000000186ab (not the display label).
- Artifacts: output/playwright/grid-070-date-ready.png, grid-070-choice.png, grid-070-entity-search.png, grid-070-applied.png, grid-070.csv; download log and fix-070 build/test logs under qa/acceptance. Initial popup screenshot caught fade-in; ready screenshot inspected. Console only showed favicon404 and the known Windows powerPreference warning. Owned browser erp-grid-070 closed.
- Scope: local Lab object catalogue, shared contracts and actual browser edit/export, no business API writes or real SKU endpoint implementation. Native physical devices and remote CI are host acceptance boundaries, not new component tasks. No Git directory; no Git diff claimed.

## 0.71 compact header and stable detail height

- Bottom Lab detail height is owned by GridLab and updated by the splitter gesture; release no longer restores framework panel size. Goods/Logs share the retained host height.
- Removed quick-search toggle buttons. Replaced the structural header toggle with a text-free tri-state page checkbox: primary click selects/clears page, secondary click inverts page. Removed duplicate Lab page-select/invert toolbar buttons.
- Serial title is 序; width is 44 logical pixels. Browser page 11 visibly displayed 1001–1006 without truncation.
- Native and Web Clippy passed. Native Release build 56.67s, Web Release 1m37s; Build-ErpWeb packaged the existing build.
- Actual Web interaction: split edge moved from y450 to y350 and stayed there after further clicks/paging; selecting the header checked 100 rows; unchecking row1 then right-clicking header left exactly row1 checked; page11 remained unselected. No business writes. Preview left open.

## 2026-09-11 formal ERP host and legacy visual baseline

- Added independent --erp / ?erp host, bounded navigation tabs, product/SKU DataGrid and optional product inspector. Lab stays available separately. Reports/warehouse remain explicitly labelled previews.
- Visual parameters were read from retired ERP source.zip, not inferred from GridLab: primary #1f3a5f, body12.8, header24/row21, left240/right360/bottom280. Latest Web rendered these with compact default tools and visible selected text.
- Targeted tests: inspector3 and shell3 passed. Renderer baseline reached69/70; remaining frozen divider coordinate was corrected to use structural widths and its targeted test passed. Native/Web Clippy passed. Latest Web Release34.05s packaged; final native Release54.55s passed with known linker import-library informational warning.
- Local QA only: Web changed product QA-P-001 name and brand together, switched Products/Orders/Products with both edits retained, then saved once. Independent API query confirmed both values. Restore via save-and-close also confirmed and removed only product tab. Desktop changed name and saved; independent API query confirmed. Restore subsequently confirmed original name `QA confirmed save and leave`, brand `易格 QA`; no production writes.
- Latest Web was reloaded after presentation-only changes and visibly queried QA-P-001 plus its SKU. Native final executable rebuilt and launched. Native automation returned working accessibility controls but white captured client area, so native pixel/visual acceptance remains unverified. Some toolbar labels remain English. No full ERP rollout or report/warehouse business completion claimed.
- Services used localhost15438 QA PostgreSQL, localhost15441 API and localhost15440 Web. Current workspace has no .git; no Git diff or remote CI evidence claimed. No cache cleanup or unbounded target directory was used.

## 2026-09-11 workbench menu/panel implementation

- Three gpt-5.6-luna subtasks handled menu/tabs, panels, and native presentation; parent integrated and corrected scope reload, dirty close queue, cross-page cancellation, and actual panel rendering. No Git metadata exists.
- Shared native/Web Clippy passed. Targeted workspace_shell3 and workbench_layout1 tests passed; renderer/core implementation unchanged, no repeat of the full grid suites.
- Native Release54.00s and Web Release36.55s packaged. Native running with logged renderer=wgpu present_mode=auto-vsync wgpu_backend=dx12 visibly rendered the new header/menu/login after activation. This replaces the earlier unverified-white-capture status for the current environment; it does not establish the original driver's root cause or every remote environment.
- Actual Web QA: classification directory showed five server options; selecting 杯具 returned QA-P-001/002. Products -> Orders -> Products retained that filter. Orders loaded12 rows; row1 detail showed actual review status/version and2 items. Dragging bottom separator moved from around625 to545 and remained after switching tabs. Right details remained order-specific.
- Actual Web created only an unsent inspector name draft, switched tabs, then chose Close All. Clean order tab closed; product draft visibly remained behind the Unsaved dialog. Discard and Close removed the final tab; function search reopened products. No business mutation was intentionally submitted in this acceptance.
- Visual inspection found sparse side-panel content shrinking widths to their minimum. Final correction expands panel content to its allocated width; final build and drag readback recorded below.
- Final sparse-panel correction: native/Web Clippy3.43s/2.49s passed; native Release51.92s and Web Release35.76s packaged. Actual final Web drag changed left boundary from x168 to x260 and retained the new width after release. Independent API readback confirmed QA-P-001 remained `QA confirmed save and leave` / `易格 QA`; the test draft was not saved.
