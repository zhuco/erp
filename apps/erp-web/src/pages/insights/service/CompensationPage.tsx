import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const CompensationPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-service-compensation-page"
    title="补偿看板"
    goal="统一观察补偿退款、补差、补贴等成本支出，识别异常成本和审核瓶颈。"
    phase="phase-2"
    dimensions={['补偿原因', '店铺', '平台', '审核人', '日期']}
    metrics={['补偿单量', '补偿金额', '单均补偿', '审核周期', '驳回率']}
    dependencies={['售后单', '退款单', '审核记录', '财务补偿台账']}
    caliberNotes={[
      '单均补偿 = 补偿金额 / 补偿单量',
      '审核周期 = 提交时间到审核完成时间',
      '驳回率 = 驳回单量 / 审核单量',
    ]}
    currentNote="当前页面用于统一补偿治理口径，待售后补偿主题接口上线后切换正式统计视图。"
  />
);
