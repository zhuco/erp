# AI-1 渲染与性能（P0）
## 目标
- 5k 行数据滚动不卡顿、拖拽列宽无明显掉帧。
- 数据更新时不跳行、不丢焦点、不闪屏。

## 范围
- 数据差量更新、重绘节流、滚动与焦点保持。

## 允许修改文件
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/erp-data-grid/hooks/useGridRender.ts`
- `apps/erp-web/src/components/erp-data-grid/utils/diff.ts`

## 实现策略
- **差量更新**
  - 维护 `prevMap(rowKey -> rowData)` 与排序 `order`，对比新数据计算新增/删除/更新。
  - 新增用 `addData`，更新用 `updateData`，删除用 `deleteRow`。
  - 变更比例过大（例如 >30%）或排序变化时，回退为 `setData` 全量刷新。
- **重绘节流**
  - 所有 `redraw` 统一走 `requestAnimationFrame` 合并。
  - `ResizeObserver` 仅在尺寸变化超过阈值时触发重绘。
- **焦点与滚动**
  - 更新前记录 `scrollTop/scrollLeft` 与 `activeRowKey`，更新后恢复。
- **虚拟渲染**
  - 启用 `renderVertical`，设置 `renderVerticalBuffer`。
  - 固定 `rowHeight`，避免布局抖动。

## 验收标准
- 5k 行滚动无明显卡顿，列宽拖拽不卡顿。
- 双击弹窗/关闭弹窗后，表格滚动位置与焦点保持不变。
- 数据刷新时不闪屏、不跳到首行。

## 测试建议
- 手动模拟 5k 行数据；拖拽列宽、滚动、频繁刷新。

## 可直接投喂提示词
请在 ERP 项目中完成「ErpDataGrid 渲染与性能优化」任务：  
【仓库】`F:\ERP`  
【核心目标】5k 行不卡顿、重绘不抖动、焦点与滚动稳定。  
【允许修改】`apps/erp-web/src/components/ErpDataGrid.tsx` 及 `apps/erp-web/src/components/erp-data-grid/**` 指定文件。  
【实现要求】  
- 基于 `rowKey` 做差量更新（新增/更新/删除）；变更比例过高时回退全量刷新。  
- 重绘使用 `requestAnimationFrame` 合并；`ResizeObserver` 只在尺寸变化超阈值时触发。  
- 更新前后保存/恢复 `scrollTop/scrollLeft` 与 `activeRowKey`。  
【交付物】  
- 修改文件清单  
- 关键实现说明  
- 手动测试步骤（含 5k 行场景）
