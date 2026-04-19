import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const DiscountInsightPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-revenue-discount-insight-page"
    title="让利诊断"
    goal="用于判断促销和折扣是否真实拉动订单增长，还是过度侵蚀利润空间。"
    phase="phase-2"
    dimensions={['平台', '店铺', '活动', 'SKU', '品类']}
    metrics={['折扣金额', '折扣率', '折扣订单量', '净销售额', '活动拉动率']}
    dependencies={['订单明细', '活动信息', 'SKU 主数据', '价格快照']}
    caliberNotes={[
      '折扣金额 = 原价销售额 - 实付销售额',
      '活动拉动率 = 活动期间净销售额相对基线的增长幅度',
      '若未接入利润数据，先展示销售侧影响指标',
    ]}
    currentNote="当前页面用于固化让利分析口径，待折扣与活动主题接口上线后补齐正式图表。"
  />
);
