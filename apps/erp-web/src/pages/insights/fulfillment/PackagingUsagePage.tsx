import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const PackagingUsagePage = () => (
  <InsightPlaceholderPage
    data-testid="insights-fulfillment-packaging-usage-page"
    title="包装耗用"
    goal="识别包材消耗异常，追踪单均耗材成本并支持成本治理。"
    phase="phase-2"
    dimensions={['仓库', '包材类型', '操作员', '发货渠道']}
    metrics={['包材用量', '包材成本', '单均耗材', '耗用异常单量']}
    dependencies={['包材主数据', '发货单', '包装作业记录']}
    caliberNotes={[
      '单均耗材 = 包材总用量 / 发货单量',
      '包材成本优先按标准成本，缺失时按最近采购价',
      '异常单量按阈值规则标记高耗用作业单据',
    ]}
  />
);
