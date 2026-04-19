import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const ReshipQualityPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-service-reship-quality-page"
    title="补发质量"
    goal="跟踪补发、错发、漏发相关质量问题及恢复效率。"
    phase="phase-2"
    dimensions={['原因', '仓库', '操作员', '店铺', '平台', 'SKU']}
    metrics={['补发单量', '错发数', '漏发数', '平均补发周期', '补发完成率']}
    dependencies={['售后单', '补发单', '发货单', '异常作业记录']}
    caliberNotes={[
      '平均补发周期 = 补发创建时间到补发完成时间',
      '补发完成率 = 已完成补发单量 / 补发单量',
      '错发与漏发需保留责任仓和责任人归因',
    ]}
  />
);
