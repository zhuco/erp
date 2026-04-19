import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const OrderMixPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-revenue-order-mix-page"
    title="订单结构"
    goal="用于观察收入构成是否过度集中，识别平台、店铺、品类和品牌结构失衡。"
    phase="phase-2"
    dimensions={['平台', '店铺', '品类', '品牌', 'SKU', '订单来源']}
    metrics={['订单量', '售出件数', 'GMV', 'TOP5 贡献率', '长尾占比']}
    dependencies={['订单明细', 'SKU 主数据', '品类主数据', '品牌主数据']}
    caliberNotes={[
      '结构占比 = 当前维度 GMV / 总 GMV',
      'TOP5 贡献率用于观察收入头部集中度',
      '长尾占比可按排名后 80% SKU 对应 GMV 占比计算',
    ]}
    currentNote="当前页面先承载结构分析定义，待结构分析主题合同落地后接入图表与明细分页。"
  />
);
