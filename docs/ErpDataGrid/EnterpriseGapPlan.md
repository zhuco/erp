# ErpDataGrid 企业版差距补齐开发文档

## 背景与目标
- 在不更换技术栈的前提下，让 `ErpDataGrid` 对标 AG Grid 企业版核心体验。
- 保持现有页面稳定，所有新增能力通过可选配置启用。
- 聚焦可复用的基础设施，优先解决大数据、配置面板与分析能力。

## 现状与缺口概览
| 能力模块 | 现状 | 需要补齐 |
| --- | --- | --- |
| 侧边栏工具面板 | 无统一入口 | 列/筛选/透视/导出工具面板 |
| 透视/交叉分析 | 无 | 透视配置、交叉汇总、总计 |
| 服务端行模型 | 仅本地数据 | 服务端分页/分块/缓存 |
| 高级筛选器 | 简单筛选 | 过滤器面板、集合/日期/数值过滤 |
| 高级导出 | CSV/基础 Excel | 多 Sheet/汇总/服务端导出 |
| 图表/可视化 | 无 | 可选图表视图与导出 |
| 树形数据/主从明细 | 无 | 树形层级、详情展开 |

## 设计约束与原则
- `ErpDataGrid.tsx` 仅做模块编排与 props 透传，复杂逻辑放入 `erp-data-grid` 目录。
- 新增能力默认关闭，全部通过可选参数开启并保持向后兼容。
- UI 文案、日志与注释保持中文。
- 避免引入大型依赖；若必须引入需在文档中明确评估理由。

## 模块规划
```
apps/erp-web/src/components/erp-data-grid/
  components/
    SidePanel.tsx
    AdvancedFilterPanel.tsx
    PivotPanel.tsx
    ChartPanel.tsx
  hooks/
    useGridSidePanel.ts
    useGridPivot.ts
    useGridServerSide.ts
    useGridAdvancedFilter.ts
    useGridTree.ts
    useGridDetail.ts
  utils/
    pivot.ts
    serverSide.ts
    advancedFilter.ts
    chart.ts
```

## 统一接口设计（新增可选项）
```ts
type ErpDataGridProps<T> = {
  sidePanelConfig?: ErpDataGridSidePanelConfig;
  pivotConfig?: ErpDataGridPivotConfig<T>;
  rowModel?: 'client' | 'server';
  serverSideConfig?: ErpDataGridServerSideConfig<T>;
  advancedFilterConfig?: ErpDataGridAdvancedFilterConfig<T>;
  treeConfig?: ErpDataGridTreeConfig<T>;
  detailConfig?: ErpDataGridDetailConfig<T>;
  chartConfig?: ErpDataGridChartConfig;
};
```

## 功能设计与实现细则

### 1. 侧边栏工具面板（Tool Panel）
**目标**
- 提供统一入口承载列管理、筛选、透视、导出与图表功能。
- 支持用户自定义扩展面板。

**交互与 UI**
- 使用右侧抽屉或固定侧栏，默认右侧展开。
- Tab 结构：列管理、筛选、透视、导出、图表（按配置开启）。
- 支持快捷键：`Ctrl + Shift + L` 打开列面板、`Ctrl + Shift + F` 打开筛选面板。

**接口设计**
```ts
type ErpDataGridSidePanelConfig = {
  enabled?: boolean;
  placement?: 'left' | 'right';
  width?: number;
  defaultOpen?: boolean;
  defaultTab?: 'columns' | 'filters' | 'pivot' | 'export' | 'chart';
  tabs?: Partial<Record<'columns' | 'filters' | 'pivot' | 'export' | 'chart', boolean>>;
  customTabs?: Array<{ key: string; title: string; render: () => ReactNode }>;
  onOpenChange?: (open: boolean) => void;
};
```

**关键实现**
- `SidePanel.tsx` 复用 `ColumnSettingsPanel`、`AdvancedFilterPanel`、`PivotPanel` 等。
- `useGridSidePanel` 负责快捷键、打开/关闭与持久化。
- 持久化键：`erp-data-grid-sidepanel:${columnSettingsKey || dataTestId}`。

**验收标准**
- 侧边栏开关与 Tab 切换稳定；刷新后保留最后的打开状态与选中 Tab。
- 侧边栏打开时不影响表格滚动与编辑。

**手动测试**
1. 打开侧边栏，切换至筛选/列管理/透视。
2. 刷新页面确认 Tab 与开关状态持久化。

### 2. 透视/交叉分析（Pivot）
**目标**
- 支持“行维度 + 列维度 + 值字段”交叉汇总。
- 支持小计、总计与格式化输出。

**接口设计**
```ts
type ErpDataGridPivotValue = {
  field: string;
  label: string;
  agg: 'sum' | 'count' | 'avg' | 'min' | 'max';
  format?: (value: number | null) => string;
};

type ErpDataGridPivotConfig<T> = {
  enabled?: boolean;
  mode?: 'client' | 'server';
  rowGroups: string[];
  columnGroups?: string[];
  values: ErpDataGridPivotValue[];
  showSubTotal?: boolean;
  showGrandTotal?: boolean;
  onPivotChange?: (config: ErpDataGridPivotConfig<T>) => void;
};
```

**关键实现**
- `useGridPivot` 在 `mode='client'` 时将原始数据转换为透视列与透视数据。
- `utils/pivot.ts` 输出 `pivotColumns` 与 `pivotRows`，并提供小计/总计计算。
- `mode='server'` 时仅派发配置，由服务端完成透视并返回结果。

**验收标准**
- 多维度交叉结果正确，小计与总计准确。
- 透视配置变更后表格自动刷新。

**手动测试**
1. 以“平台 → 店铺”为行维度，“状态”为列维度。
2. 校验小计/总计与手算一致。

### 3. 服务端行模型（Server Side Row Model）
**目标**
- 支持百万级数据的分块加载与缓存。
- 支持远程排序、筛选、分组、透视的统一请求。

**接口设计**
```ts
type ErpDataGridServerSideParams<T> = {
  startRow: number;
  endRow: number;
  sortStates: ErpDataGridSortStates;
  globalFilter: string;
  filterState: GridFilterState<T>;
  advancedFilter?: ErpDataGridAdvancedFilterModel;
  groupConfig?: ErpDataGridGroupConfig<T>;
  pivotConfig?: ErpDataGridPivotConfig<T>;
};

type ErpDataGridServerSideResult<T> = {
  rows: T[];
  total?: number;
  lastRow?: number;
  summary?: Record<string, number | null>;
};

type ErpDataGridServerSideConfig<T> = {
  getRows: (params: ErpDataGridServerSideParams<T>) => Promise<ErpDataGridServerSideResult<T>>;
  blockSize?: number;
  maxBlocks?: number;
  debounceMs?: number;
  autoRefreshOnChange?: boolean;
};
```

**关键实现**
- `useGridServerSide` 维护区块缓存与请求节流，支持滚动触底预取。
- 结合 Tabulator `setData`/`addData`/`replaceData` 实现增量填充。
- `autoRefreshOnChange` 监听排序、筛选、透视配置变化并刷新缓存。

**验收标准**
- 10 万行数据滚动稳定，区块请求不重复。
- 排序/筛选变化后自动刷新并回到顶部。

**手动测试**
1. 配置 `blockSize=200`，快速滚动验证分块请求。
2. 切换排序/筛选后验证请求参数与结果同步。

### 4. 高级筛选器（Advanced Filters）
**目标**
- 提供类似 AG Grid 的过滤器面板与多条件组合。
- 支持“集合筛选 / 数值筛选 / 日期筛选 / 文本筛选”。

**接口设计**
```ts
type ErpDataGridAdvancedFilterRule = {
  id: string;
  field: string;
  operator: 'contains' | 'equals' | 'in' | 'gt' | 'gte' | 'lt' | 'lte';
  value?: unknown;
};

type ErpDataGridAdvancedFilterGroup = {
  id: string;
  logic: 'and' | 'or';
  children: Array<ErpDataGridAdvancedFilterRule | ErpDataGridAdvancedFilterGroup>;
};

type ErpDataGridAdvancedFilterConfig<T> = {
  enabled?: boolean;
  model?: ErpDataGridAdvancedFilterGroup;
  onModelChange?: (model: ErpDataGridAdvancedFilterGroup) => void;
  distinctValues?: Record<string, Array<string | number>>;
  mode?: 'local' | 'remote';
};
```

**关键实现**
- `AdvancedFilterPanel` 提供条件构建器与字段/操作符选择。
- `utils/advancedFilter.ts` 将模型转换为本地过滤函数或请求参数。
- 与 `FilterBar` 标签联动，条件标签可移除。

**验收标准**
- 本地模式下多条件过滤结果正确。
- 远程模式下条件变化会触发 `onModelChange`。

**手动测试**
1. 组合两个字段条件（AND/OR）验证结果。
2. 切换为远程模式，检查参数派发。

### 5. 高级导出（Advanced Export）
**目标**
- 支持多 Sheet、分组/透视/汇总导出。
- 支持服务端导出（大数据量）。

**接口设计**
```ts
type ErpDataGridExportConfig = {
  mode?: 'client' | 'server';
  types?: Array<'csv' | 'excel'>;
  fileName?: string | ((type: 'csv' | 'excel') => string);
  sheetName?: string;
  includeSummary?: boolean;
  includeGroup?: boolean;
  includePivot?: boolean;
  serverRequest?: (type: 'csv' | 'excel') => Promise<Blob>;
};
```

**关键实现**
- 客户端导出：使用 `xlsx` 生成多 Sheet，复用列渲染格式化。
- 服务端导出：调用 `serverRequest` 获取文件并下载，失败时提示中文错误。
- 支持导出当前筛选/排序/列显隐视图。

**验收标准**
- 导出文件列顺序、筛选结果与页面一致。
- 服务器导出失败时提示可读错误。

**手动测试**
1. 筛选后导出并核对行数与列顺序。
2. 切换服务端导出验证下载与错误提示。

### 6. 图表/可视化（可选）
**目标**
- 对标 AG Grid Charts，提供基础统计图表。
- 作为侧边栏 Tab 独立启用，不影响表格核心逻辑。

**接口设计**
```ts
type ErpDataGridChartConfig = {
  enabled?: boolean;
  defaultType?: 'bar' | 'line' | 'pie';
  dataResolver?: (rows: unknown[]) => { labels: string[]; values: number[] };
};
```

**关键实现**
- 若后续引入图表库（如 ECharts/AntV），封装在 `ChartPanel` 内。
- 未引入图表库时，图表 Tab 默认隐藏。

**验收标准**
- 启用图表后可展示基础统计结果。
- 关闭图表时不影响表格性能与渲染。

### 7. 树形数据与主从明细
**目标**
- 支持树形层级展开与详情行展示。

**接口设计**
```ts
type ErpDataGridTreeConfig<T> = {
  enabled?: boolean;
  childField?: string;
  expandByDefault?: boolean | number;
  lazy?: boolean;
  loadChildren?: (record: T) => Promise<T[]>;
};

type ErpDataGridDetailConfig<T> = {
  enabled?: boolean;
  render: (record: T) => ReactNode;
  height?: number;
  expandOnRowClick?: boolean;
};
```

**关键实现**
- 树形数据使用 Tabulator `dataTree`，支持懒加载子节点。
- 详情行通过 `rowFormatter` 或自定义行模板渲染。
- 详情展开状态可持久化（按 `rowKey`）。

**验收标准**
- 展开/折叠流畅，懒加载请求可复用缓存。
- 详情行内容与行数据同步更新。

**手动测试**
1. 打开树形数据，展开多级节点验证加载。
2. 展开详情行并更新数据，验证刷新表现。

## 交付节奏与优先级
- **P0**：侧边栏工具面板、服务端行模型、高级筛选器。
- **P1**：透视/交叉分析、高级导出、树形数据/主从明细。
- **P2**：图表/可视化与细节打磨（动效、快捷键）。

## 全局验收清单
- 旧页面与原有 props 全量兼容。
- 侧边栏与高级筛选可独立关闭。
- 服务端行模型下排序/筛选/透视请求参数一致。
- 导出结果与当前视图一致。

## 通用手测步骤
1. 订单审核页开启侧边栏，依次打开列/筛选/导出面板。
2. 切换服务端行模型，快速滚动并验证请求参数。
3. 添加高级筛选条件并导出，核对导出结果。
