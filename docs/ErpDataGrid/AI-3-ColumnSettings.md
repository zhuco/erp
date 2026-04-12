# AI-3 列管理面板（P0）

## 完成状态
- 已完成列显隐、搜索、拖拽排序、冻结列、视图保存与恢复。
- 已提供视图重置默认能力。
- 已兼容旧版 `columnSettingsKey` 的数组存储格式。

## 功能范围
- 列设置面板 UI。
- 列状态持久化与迁移。
- 冻结列（左/右/取消）应用。

## 实现说明
### 列状态结构
- 列状态：`{ key, visible, width, frozen: 'left' | 'right' | null, order }`。
- `defaultColumnState` 由列定义生成，保留默认显示、宽度与冻结信息。

### 存储与兼容
- `columnViewKey = columnSettingsKey`，存储到 `localStorage`。
- 新格式：`{ version: 1, columns: ColumnViewState[] }`。
- 旧格式：若读取到 `string[]`，视为可见列列表并平滑迁移。
- `normalizeColumnViewState` 负责列缺失回退与冻结排序归位（左/中/右）。

### 交互与视图
- 搜索：按列标题模糊过滤可选/已选列列表。
- 批量操作：支持“全部显示 / 全部隐藏 / 恢复默认”。
- 拖拽排序：仅允许同冻结分组内排序，避免冻结顺序错乱。
- 冻结列：在已选列中通过下拉菜单设置“左冻结 / 右冻结 / 取消冻结”。

### 表格应用
- `columnMoved`：更新列顺序并保存。
- `columnResized`：保存列宽。
- `column.freeze()` / `column.unfreeze()`：应用冻结与取消冻结。

## 修改文件清单
- `apps/erp-web/src/components/erp-data-grid/components/ColumnSettingsPanel.tsx`
- `apps/erp-web/src/components/erp-data-grid/utils/columnStorage.ts`
- `apps/erp-web/src/components/erp-data-grid/ErpDataGridImpl.tsx`

## 验收标准
- 列显隐、顺序、冻结可被保存与恢复。
- 视图重置可恢复默认列状态。
- 与现有 `columnSettingsKey` 兼容或平滑迁移。

## 手动测试步骤
1. 进入订单审核页面，打开列设置面板。
2. 进行列显隐、拖拽排序、冻结设置后刷新页面确认状态保持。
3. 点击“恢复默认”并刷新页面，确认列状态恢复初始值。
