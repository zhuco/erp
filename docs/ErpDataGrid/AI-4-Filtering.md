# AI-4 快速筛选与条件面板（P0/P1）

## 目标
- 全局快速筛选、条件面板与筛选标签化展示。
- 支持本地过滤与服务端过滤两种模式。

## 范围
- 全局筛选输入、条件面板、筛选状态管理。

## 允许修改文件
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/erp-data-grid/components/FilterBar.tsx`
- `apps/erp-web/src/components/erp-data-grid/hooks/useGridFilter.ts`

## 接口设计
- `filterMode: 'local' | 'remote'`：默认 `local`，决定本地过滤或仅派发筛选状态。
- `globalFilter` / `onGlobalFilterChange`：全局关键字与变更回调。
- `filterState` / `onFilterChange`：结构化条件列表与变更回调。
- `filterPanel` / `filterPanelTitle` / `filterBarPlaceholder` / `showFilterBar`：筛选面板与搜索栏显示配置。
- `GridFilterCondition` 结构：
  - `key`: 唯一标识，用于标签移除。
  - `label`: 条件名称，用于标签展示。
  - `field`: 字段路径（支持 `a.b.c`）。
  - `operator`: `contains | equals | startsWith | endsWith | gt | gte | lt | lte | in`。
  - `value` / `displayValue`: 实际值与标签展示值。
  - `predicate`: 自定义行过滤函数（优先生效）。
  - `removable` / `disabled`: 控制标签可移除与条件禁用。

## 实现策略
- 新增 `filterMode: 'local' | 'remote'`（默认 `local`）。
- `globalFilter` 作为全局关键字，`filterState` 记录结构化条件。
- 本地模式：使用 Tabulator `setFilter` + 自定义函数。
  - 关键字优先扫描 `columns.field`，未配置时遍历整行可枚举值。
  - 条件项按 `operator` 执行匹配，支持自定义 `predicate`。
- 远程模式：触发 `onFilterChange` 由页面发请求。
- 条件面板输出“筛选标签”，可一键清除。
  - `FilterBar` 在传入筛选参数或 `filterPanel` 时显示。
  - 面板按钮提供“保存筛选/清空条件”快捷操作。
  - 全局关键字标签为“关键字：xxx”，条件标签为“label：value”。
  - 标签移除会同步更新 `globalFilter` 或 `filterState`。

## 验收标准
- 全局搜索可立即过滤或触发远程查询。
- 条件面板可保存/清空，标签可移除单个条件。

## 测试建议
- 订单审核页模拟全局关键字 + 条件组合筛选。

## 可直接投喂提示词
请在 ERP 项目中完成「ErpDataGrid 快速筛选与条件面板」任务：  
【仓库】`F:\ERP`  
【核心目标】全局筛选 + 条件面板 + 标签化筛选，支持本地与服务端模式。  
【允许修改】`apps/erp-web/src/components/ErpDataGrid.tsx`、`apps/erp-web/src/components/erp-data-grid/components/FilterBar.tsx`、`apps/erp-web/src/components/erp-data-grid/hooks/useGridFilter.ts`  
【接口建议】  
- `filterMode: 'local' | 'remote'`  
- `globalFilter`、`onGlobalFilterChange`  
- `filterState`、`onFilterChange`  
【实现要求】  
- 本地模式使用 Tabulator `setFilter` 或自定义过滤函数。  
- 远程模式只派发状态，由页面请求数据。  
- 条件面板显示标签，支持一键清空与单项移除。  
【交付物】  
- 修改文件清单  
- 关键实现说明  
- 手动测试步骤
