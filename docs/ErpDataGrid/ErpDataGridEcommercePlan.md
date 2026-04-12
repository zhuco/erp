# ErpDataGrid 电商 ERP 开发详细文档

## 背景与目标
- 电商 ERP 的核心场景是订单、商品、库存、资金对账等列表密集操作，要求**筛选快、导出快、批量操作稳**。
- 目标是在不替换 Tabulator 技术栈的前提下，让 ErpDataGrid 在电商 ERP 里具备“可用即稳定”的核心体验，并把重分析能力留到后续迭代。

## 功能分级（必要 vs 后续）

### P0 必要功能（必须具备）
| 功能 | 说明 | 当前状态 | 是否补齐 | 备注 |
| --- | --- | --- | --- | --- |
| 列管理 | 列显隐/冻结/拖拽/宽度持久化 | 已具备 | 否 | 继续沿用现有列设置弹窗 |
| 多列排序 | 支持多列优先级排序 | 已具备 | 否 | 使用 `useGridSorting` |
| 快速筛选 | 全局关键字 + 条件标签 | 已具备 | 否 | 过滤条已实现 |
| **高级筛选** | 多条件 AND/OR 组合 | 部分具备 | **是** | 本次整合到主组件 |
| 批量编辑 | 批量修改 + 校验 | 已具备 | 否 | 订单/商品批改必需 |
| 行选择 | 单选/多选/范围复制 | 已具备 | 否 | 适配批量操作 |
| 状态栏汇总 | 总数/选中/汇总 | 已具备 | 否 | 适配订单金额汇总 |
| 导出 | CSV/Excel | 已具备 | 否 | Excel 依赖前端 XLSX 或后端导出 |
| 右键菜单 | 快捷操作入口 | 已具备 | 否 | 保持现有菜单结构 |

### P1 后续完善（可延后）
| 功能 | 原因 | 依赖 |
| --- | --- | --- |
| 服务端行模型 | 需要后端分页/缓存协议 | API 协作 |
| 透视分析 | 分析类需求，非业务操作主链路 | UI + 计算层 |
| 图表分析 | 需要接入图表库 | ECharts/AntV |
| 工具侧边栏 | 体验提升类入口 | 列管理/高级筛选改造 |
| 树形数据 | 仅少量场景（BOM、类目树） | 表格渲染扩展 |
| 主从明细 | 订单/入库明细可选 | 行展开容器 |
| 高级导出 | 多 Sheet/样式/服务端导出 | 需要后端导出 |

## 本次补齐能力：高级筛选整合

### 设计目标
- 在主组件中直接支持高级筛选规则（AND/OR、多类型字段）。
- 高级筛选与现有筛选条联动，能够清空、统计条件数并影响导出/统计。
- 不影响已存在的 `filterPanel` 自定义能力。

### 接口设计
```ts
type ErpDataGridProps<T> = {
  advancedFilterConfig?: ErpDataGridAdvancedFilterConfig<T>;
};

type ErpDataGridAdvancedFilterConfig<T> = {
  enabled?: boolean;
  mode?: 'local' | 'remote';
  model?: ErpDataGridAdvancedFilterGroup;
  onModelChange?: (model: ErpDataGridAdvancedFilterGroup) => void;
  fields?: Array<{
    field: keyof T | string;
    label: string;
    type?: 'text' | 'number' | 'date' | 'select';
    options?: Array<{ label: string; value: unknown }>;
  }>;
};
```

### 行为说明
- 当 `advancedFilterConfig.enabled=true` 且 `filterPanel` 未提供时，自动使用 `AdvancedFilterPanel`。
- 当 `fields` 未传入时，会根据 `columns` 自动生成可筛选字段与类型。
- 当 `advancedFilterConfig.mode !== 'remote'` 时，前端使用规则函数过滤数据。
- 过滤条会显示“高级筛选：X 条”标签，清空条件会同时清空高级筛选。
- 过滤后的结果参与导出、状态栏统计与已选行统计。

### 兼容性
- `advancedFilterConfig` 默认关闭，不影响既有页面。
- 已存在自定义 `filterPanel` 时，不自动替换。

## 验收标准
- 高级筛选面板能新增/删除规则并实时生效。
- 清空筛选条件会清空高级筛选规则。
- 高级筛选生效后，导出行数与状态栏统计同步变化。
- 未启用高级筛选时，行为与当前版本保持一致。

## 手动测试步骤
1. 在订单列表页传入 `advancedFilterConfig.enabled=true`，配置字段列表。
2. 添加 2 条条件（AND/OR），观察表格实时过滤。
3. 点击“清空条件”，高级筛选规则与过滤结果恢复。
4. 导出 CSV，确认导出行数与当前过滤结果一致。

## 本次补齐能力：透视分析集成

### 设计目标
- 支持基于现有数据的透视汇总视图（行维度 + 列维度 + 值字段）。
- 允许业务页面直接传入 `pivotConfig` 完成透视展示，保持对普通表格的兼容。

### 接口设计
```ts
type ErpDataGridProps<T> = {
  pivotConfig?: ErpDataGridPivotConfig<T>;
};

type ErpDataGridPivotConfig<T> = {
  enabled?: boolean;
  mode?: 'client' | 'server';
  rowGroups: string[];
  columnGroups?: string[];
  values: Array<{
    field: string;
    label: string;
    agg: 'sum' | 'count' | 'avg' | 'min' | 'max';
    format?: (value: number | null) => string;
  }>;
  showSubTotal?: boolean;
  showGrandTotal?: boolean;
  onPivotChange?: (config: ErpDataGridPivotConfig<T>) => void;
};
```

### 行为说明
- `pivotConfig.enabled=true` 时，表格渲染透视后的行列数据。
- 默认采用客户端透视计算；`mode='server'` 时只派发配置变更，数据由服务端返回。
- 透视数据与导出、状态栏统计保持一致。

### 验收标准
- 透视配置生效后，表格展示行列交叉汇总。
- 关闭透视配置时，表格恢复原始列与数据。

### 手动测试步骤
1. 传入 `pivotConfig`，设置行维度与值字段。
2. 切换 `showGrandTotal`，验证总计行显示/隐藏。
3. 导出 CSV/Excel，确认导出内容与透视视图一致。
