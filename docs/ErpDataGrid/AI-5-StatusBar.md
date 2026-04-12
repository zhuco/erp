# AI-5 状态栏与汇总（P0）

## 目标
- 展示总数、选中数与金额等关键汇总信息。
- 与筛选、排序、选择联动更新。

## 范围
- 状态栏展示组件与汇总计算逻辑。
- `ErpDataGrid` 统一接入状态栏能力。

## 设计与接口
- `summaryFields`：定义需要汇总的字段与格式化方式。
  - `key`：字段唯一标识。
  - `label`：显示文案。
  - `field`：字段路径（支持 `a.b`）。
  - `getValue`：自定义取值函数（优先于 `field`）。
  - `format`：自定义格式化函数（如金额格式化）。
  - `scope`：`all` 或 `selected`，指定统计范围。
- `summaryResolver`：支持服务端汇总注入。
  - 支持返回 `{ all, selected }` 或仅返回汇总对象。
  - 可为函数，形参包含数据、选中行与字段定义。
  - 服务端返回值会覆盖本地计算值。
- `showStatusBar`：布尔值，默认 `true`，可关闭状态栏。

## 计算与联动
- 本地筛选模式下，以 Tabulator 的 `active` 数据为统计基准。
- 远端筛选或未启用筛选时，统计 `data` 全量。
- 选中数据与选中数实时更新汇总。
- 无数据时，汇总值显示 `--` 占位，不影响布局。

## 组件与样式
- 状态栏复用 `.order-workbench-pagination` 与 `.order-workbench-pagination-summary`。
- 展示格式：`总数 x 条，选中 y 条，汇总项：值`。

## 实现清单
- `apps/erp-web/src/components/ErpDataGrid.tsx`
- `apps/erp-web/src/components/erp-data-grid/components/StatusBar.tsx`
- `apps/erp-web/src/components/erp-data-grid/utils/summary.ts`

## 使用示例
```tsx
const summaryFields = [
  {
    key: 'amount',
    label: '金额合计',
    field: 'pay_amount',
    format: (value) => (value == null ? '--' : `￥${value.toFixed(2)}`)
  },
  {
    key: 'selectedAmount',
    label: '选中金额',
    field: 'pay_amount',
    scope: 'selected',
    format: (value) => (value == null ? '--' : `￥${value.toFixed(2)}`)
  }
];

<ErpDataGrid
  data={rows}
  columns={columns}
  rowKey={(row) => row.id}
  summaryFields={summaryFields}
  summaryResolver={({ data }) => ({
    all: { amount: data.reduce((sum, row) => sum + (row.pay_amount ?? 0), 0) }
  })}
/>;
```

## 验收标准
- 总数/选中数/合计实时更新。
- 无数据时显示占位，不影响布局。
- 状态栏样式与页脚一致。

## 测试建议
- 在订单审核页勾选多行，验证合计数与金额。
