# AI-8 分组与聚合（P1）

## 目标
- 支持按平台/店铺/状态分组，并展示小计/总计。

## 范围
- 分组配置、分组头部展示、聚合字段计算。

## 允许修改文件
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/erp-data-grid/hooks/useGridGrouping.ts`
- `apps/erp-web/src/components/erp-data-grid/utils/aggregation.ts`

## 实现策略
- `groupConfig` 定义 `groupBy`、`groupHeader`、`openByDefault`。
- 聚合使用 Tabulator `groupHeader` 或自定义汇总。
- 与列冻结、排序、筛选联动。

## 验收标准
- 分组展开/折叠流畅，小计与总计数值准确。
- 分组状态可保存并恢复。

## 测试建议
- 订单审核按平台/店铺分组测试。

## 修改文件清单
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/erp-data-grid/hooks/useGridGrouping.ts`
- `apps/erp-web/src/components/erp-data-grid/utils/aggregation.ts`

## 关键实现说明
### 分组配置
- `groupConfig` 新增为可选参数，包含 `groupBy`、`groupHeader`、`openByDefault`。
- `groupBy` 直接透传 Tabulator `setGroupBy`，支持字符串、数组或函数。
- `groupHeader` 支持单个回调或按层级数组，默认渲染为“分组值（条数 + 小计）”。
- `openByDefault` 支持布尔值或数组，用于控制各层级默认展开。

### 聚合字段识别与计算
- 新增 `aggregation.ts`，解析列配置与样本数据，生成聚合字段（优先使用列的 `bottomCalc/topCalc`）。
- 若列没有显式聚合但符合数值特征（右对齐或数值排序），自动使用 `sum`。
- 通过 `applyAggregationToColumns` 自动补齐 `bottomCalc`，确保分组下的小计行与整体总计准确显示。

### 分组状态持久化
- 展开/折叠状态以本地存储持久化，存储键格式：
  `erp-data-grid-grouping:${columnSettingsKey || dataTestId}:${groupByKey}`。
- 监听 `groupVisibilityChanged` 写入状态，`dataGrouped` 时自动恢复。

## 手动测试步骤
1. 进入订单审核页，给 `ErpDataGrid` 传入 `groupConfig`：
   - `groupBy` 设置为平台 → 店铺 → 状态。
2. 展开/折叠任意分组，刷新页面，确认分组状态可恢复。
3. 对列表进行筛选或排序，观察分组头部的小计与表格底部总计准确。

## 使用示例
```tsx
<ErpDataGrid
  columns={columns}
  data={data}
  rowKey={(row) => row.id}
  columnSettingsKey="order-review-data-grid-columns"
  groupConfig={{
    groupBy: ['platform', 'shopName', 'status'],
    openByDefault: [true, false, false]
  }}
/>
```

## 可直接投喂提示词
请在 ERP 项目中完成「ErpDataGrid 分组与聚合」任务：  
【仓库】`F:\ERP`  
【核心目标】分组展开/折叠稳定，小计/总计准确，与筛选/排序联动。  
【允许修改】`apps/erp-web/src/components/ErpDataGrid.tsx`、`apps/erp-web/src/components/erp-data-grid/hooks/useGridGrouping.ts`、`apps/erp-web/src/components/erp-data-grid/utils/aggregation.ts`  
【接口建议】  
- `groupConfig`：`groupBy`、`groupHeader`、`openByDefault`  
【实现要求】  
- 支持平台/店铺/状态分组。  
- 分组状态可保存与恢复。  
【交付物】  
- 修改文件清单  
- 关键实现说明  
- 手动测试步骤
