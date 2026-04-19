import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const StockAgingPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-inventory-stock-aging-page"
    title="库龄健康"
    goal="衡量库存老化程度并识别高库龄库存，支持去化与补货策略。"
    phase="phase-1.5"
    dimensions={['仓库', 'SKU', '品类', '库龄区间', '批次']}
    metrics={['在库量', '库龄库存额', '平均库龄天数', '呆滞占比', '超阈值批次数']}
    dependencies={['批次库存', '入库日期', '库存余额']}
    caliberNotes={[
      '库龄天数 = 当前日期 - 入库日期或最近有效入库批次日期',
      '呆滞阈值支持按品类配置',
      '优先支持批次级追溯，缺失批次时回退 SKU 级统计',
    ]}
    currentNote="当前页面通过库存与报表接口拼装交付，后续升级为独立主题分析。"
  />
);
