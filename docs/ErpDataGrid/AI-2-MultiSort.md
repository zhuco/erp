# AI-2 多列排序（P0）

## 目标
- 支持多列排序与优先级提示。
- 与服务端排序参数兼容。

## 范围
- 排序状态结构、多列排序交互、表头排序优先级显示。

## 允许修改文件
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/ErpDataGrid.css`
- `apps/erp-web/src/components/erp-data-grid/hooks/useGridSorting.ts`

## 接口设计
- `sortStates: Array<{ field: string; order: 'ascend' | 'descend' }>`
- `onSortStatesChange: (states) => void`

## 实现说明
- 使用 `useGridSorting` 同步多列排序状态，将 Tabulator 的 `sorters` 映射为 `sortStates`。
- 开启 `columnHeaderSortMulti` 以支持 Shift 追加排序；普通点击保持单列排序行为。
- `sortStates` 受控时通过 `setSort([{ column, dir }])` 恢复排序，确保刷新/切换视图后可恢复。
- 列头写入 `data-sort-priority` 并用 CSS `::after` 展示排序优先级序号；未排序列不显示。
- `sortStates` 未传入时继续使用单列 `sortState`，避免破坏既有行为。

## 验收标准
- Shift 可追加排序，排序优先级正确显示。
- 刷新或切换视图后排序状态可恢复。
- 未排序列不显示排序符号。

## 手动测试
- 在订单审核表依次点击两列并按住 Shift 追加排序，确认优先级序号为 1、2。
- 普通点击任一列，确认只保留单列排序且序号更新。
- 刷新页面或切换视图后，确认排序状态与序号恢复。

## 交付物
- 修改文件清单：`ErpDataGrid.tsx`、`ErpDataGrid.css`、`useGridSorting.ts`。
- 关键实现说明：多列排序状态同步、优先级序号渲染。
- 手动测试步骤：订单审核表多列排序与恢复验证。
