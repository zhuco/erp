# ERP 网格实验台

当前实现为独立交互实验台，使用同一 `erp-ui-egui::DataGrid`，不访问业务 API 或数据库。

已实现矩形拖选、Shift/Ctrl/Cmd 行连续勾选与取消、Escape 清选/取消手势、列拖序与拖宽、方向键、越界自动滚动、双向可见范围绘制。合成数据规模为 10/1 万/10 万/100 万行 × 100 列；绘制仅取可见单元格，筛选、排序和全量汇总分帧处理。

左侧冻结、编辑和视图持久化已实现；真实 API、完整无障碍及真实宿主中文 IME 仍待验收。中文字体目前仅 Windows 加载系统字体；Web 实验台暂用英文标签，后续需可分发 CJK 字体和中文验收。可见单元格数量测试不等于性能预算通过。

## 行勾选与快速搜索

左侧固定勾选列与活动行/复制选区相互独立。普通点击仅激活行；有勾选时操作只针对勾选，否则针对活动行。右键业务动作在变更预览后应用到本地模拟数据，不发送真实业务请求。Shift/Ctrl/Cmd 点击或拖动数据行也会勾选/取消行，不再扩展单元格复制选区；Shift 点击复选框连续勾选或取消，Space 切换焦点行。拖框仅改变复制选区，Ctrl/Cmd+C 复制 TSV（最多 10,000 单元格/1 MiB）。

勾选列表头 +/− 展开或折叠逐列搜索，也可用工具栏切换。默认展开；修改条件后自动查询（250 毫秒防抖，输入法组合态不提交），按列类型筛选整个本地集合：文本包含、Price 精确金额范围、Created at 日期时间范围、Status/Warehouse 下拉多选。不同列 AND、同列多选 OR；Reset search 清除。金额在搜索行直接输入最小值 - 最大值；日期在弹窗中选择起止范围，倒置/非法值阻止应用。下拉仅适用分类完整候选不超过 100 个（含 100），不完整或超限使用文本。日期时间原型按 UTC 秒精度。折叠保留条件。查询分帧执行，处理中禁用操作预览，完成后清除旧行选择。搜索使用稳定 ID 映射，不以筛选后的行号作为业务 ID。

## 构建与测试

在项目根执行，复用既有 `build/cargo`，不另设会话缓存：

```powershell
./scripts/Invoke-ErpBuild.ps1 -Action test
./scripts/Invoke-ErpBuild.ps1 -Action clippy
./scripts/Invoke-ErpBuild.ps1 -Action fmt
./scripts/Invoke-ErpBuild.ps1 -Action build
./build/cargo/debug/erp-grid-lab.exe
./scripts/Invoke-ErpBuild.ps1 -Action check -Platform web
```

Web 使用共享 `src/lib.rs` 的 wasm-bindgen 启动入口。Web 检查不生成可部署 JS。制作浏览器包需先经入口 `-Action build -Platform web` 构建，再用与 Cargo.lock 一致的 wasm-bindgen CLI 对 `build/cargo/wasm32-unknown-unknown/debug/erp_ui.wasm` 执行 `--target web`，输出至发布目录 `pkg` 并配本目录 index.html，通过本机 HTTP 服务访问。浏览器打包和运行验收尚未完成。

## 调用约束

每帧绘制前调用 `GridState::bind_view(generation)`；作用域、行数量/顺序/身份变化必须递增 generation。复制选区仅用于显示/复制，不能作为后台操作 ID；业务范围经 row_selection.targets() 得到稳定 RowId。ColumnId 必须唯一，宽度限制 64—640pt；列重排明确清除旧坐标选区。

实验台仅支持有限本地数据；远端保存与客户扩展按 [组件规范](../../docs/UI_COMPONENT_LIBRARY.md) 接入，新增能力不能只声明配置。

界面不显示说明性副标签或常驻功能介绍，使用控件悬停提示。

## 列配置、分页与短表

表头右键 → Columns 打开列面板：搜索列名、显示/隐藏、Lock 左侧锁定、↑/↓ 调整组内列顺序。右键也可直接隐藏或锁定当前列。锁定列横向滚动时留在勾选列右侧；锁定区过宽时自动从最右侧解除锁定并提示。配置已接入版本化持久化。

工具栏可切换 10 条数据；横向滚动条始终贴网格底部。底部分页器提供页码、首页/末页及每页 10/20/50/100/200/500 行，默认 100 行。搜索作用于全量合成数据后再分页；换页清除旧选择，查询成功回第一页。未接服务端分页。

当前缺口与底部子表方案见 [组件库规范](../../docs/UI_COMPONENT_LIBRARY.md) 的交互差距审计。

勾选列、序号列始终固定显示，列面板不能隐藏或移动；横向滚动条从这两列右侧开始。序号在筛选结果内跨页连续，第二页 100 行显示 101 起。

## 排序、编辑和底部子表

- 单击表头切换排序，Shift 单击叠加；右键 Sort / Then 指定方向。
- 主表普通点击驱动底部 Goods/Logs；拖动分隔线调整高度。子表独立查询、勾选、分页和列偏好。
- 双击/F2 编辑名称、数量、金额、状态、仓库；Enter/Tab 提交，Escape 取消。SKU/日期/日志只读。
- Ctrl+V 粘贴矩形数据，先看预览再 Apply；无效内容整批拒绝。Undo/Redo 或 Ctrl+Z/Ctrl+Y 撤销/重做本地修改。
- 数据区右键可复制、编辑，以及预览后设置 Ready/Hold；动作按勾选行优先，否则当前行。
- Page / Checked / All matches 切换汇总范围；Export 导出 CSV 或 XLSX，桌面生成 Downloads 新文件，Web 发起浏览器下载。
- 视图配置自动保存；模拟数据修改仅在当前运行中有效。API 保存、生产权限、真实宿主下载和存储失败尚需验收。
- 行内展开尚未获用户确认，本轮不实现。
