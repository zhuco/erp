import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const StockBalancePage = () => (
  <InsightPlaceholderPage
    data-testid="insights-inventory-stock-balance-page"
    title="收发存总览"
    goal="一屏查看期初、收入、发出、结余的库存变化，帮助快速判断库存波动是否健康。"
    phase="phase-1.5"
    dimensions={['仓库', 'SKU', '品类', '事务类型']}
    metrics={['期初量', '入库量', '出库量', '期末量', '净变化量']}
    dependencies={['库存流水', '仓库主数据', 'SKU 主数据']}
    caliberNotes={[
      '期初量取统计期间第一天开始库存',
      '净变化量 = 入库量 - 出库量',
      '收发存视图优先与 reports 与库存接口拼装结果保持一致',
    ]}
    currentNote="当前页面按 Phase 1.5 先做拼装交付，后续会沉淀为独立主题分析页。"
  />
);
