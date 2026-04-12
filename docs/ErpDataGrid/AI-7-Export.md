# AI-7 导出能力（P0）
## 目标
- 支持 CSV/Excel 导出，并保持与当前筛选/排序/列显隐一致。
- 导出入口可按需启用，避免影响现有页面。

## 范围
- 导出配置 `exportConfig`、字段选择、文件名与导出入口。

## 允许修改文件
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/erp-data-grid/utils/export.ts`

## 接口设计
- `exportConfig`
  - `type`/`types`: `'csv' | 'excel'`，支持单个或多个导出类型。
  - `fileName`: `string | (type) => string`，支持按类型生成文件名。
  - `columns`: `'visible' | 'all' | string[]`，默认跟随可见列。
  - `includeHeader`: `boolean`，默认带表头。

## 实现策略
- CSV 优先使用 Tabulator `download('csv')`，并传入 `active` 数据集确保筛选/排序一致。
- Excel 仅在存在 `window.XLSX` 时执行，否则自动回退为 CSV 并提示需要后端导出接口。
- 文件名自动补全扩展名，默认 `导出数据-YYYYMMDDHHmmss.csv`。
- 空数据时导出按钮禁用，并在运行时二次检查给出中文提示。

## 验收标准
- 导出结果与当前列显隐/排序/筛选一致。
- Excel 无依赖时提示回退为 CSV。
- 空数据时导出按钮不可用或提示“暂无可导出的数据”。

## 测试建议
- 在订单审核页筛选后导出，核对列顺序、排序与筛选结果一致。

## 可直接投喂提示词
请在 ERP 项目中完成「ErpDataGrid 导出能力」任务：  
【仓库】`F:\ERP`  
【核心目标】CSV/Excel 导出与当前列显隐/排序/筛选保持一致。  
【允许修改】`apps/erp-web/src/components/ErpDataGrid.tsx`、`apps/erp-web/src/components/erp-data-grid/utils/export.ts`  
【接口建议】  
- `exportConfig`（类型、文件名、列范围、是否带表头）  
【实现要求】  
- 优先使用 Tabulator `download('csv')`。  
- Excel 若无依赖则先做 CSV；需要 Excel 时提示后端导出。  
- 空数据时给出中文提示或禁用导出。  
【交付物】  
- 修改文件清单  
- 关键实现说明  
- 手动测试步骤
