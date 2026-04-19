import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const StocktakeVariancePage = () => (
  <InsightPlaceholderPage
    data-testid="insights-fulfillment-stocktake-variance-page"
    title="盘点偏差"
    goal="识别盘点差异集中仓库、SKU 和批次，支撑内控与责任追溯。"
    phase="phase-2"
    dimensions={['仓库', '盘点批次', 'SKU', '盘点单据', '操作员']}
    metrics={['差异单数', '差异 SKU 数', '差异量', '差异金额', '差异率']}
    dependencies={['盘点单', '库存余额', '库存成本', '操作员档案']}
    caliberNotes={[
      '差异率 = 差异量 / 账面量',
      '差异金额按库存成本口径计算',
      '盘点批次需支持跳转盘点任务详情',
    ]}
  />
);
