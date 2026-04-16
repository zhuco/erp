import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const OperationAccuracyPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-fulfillment-operation-accuracy-page"
    title="作业准确度"
    goal="跟踪错发、漏发、误拣等质量问题密度，持续提升仓内作业准确率。"
    phase="phase-2"
    dimensions={['仓库', '配货员', '打包员', '班次', '错误类型']}
    metrics={['错漏数', '错发率', '漏发率', '复核覆盖率', '返工单量']}
    dependencies={['发货异常记录', '复核日志', '售后补发数据']}
    caliberNotes={[
      '错发率 = 错发订单数 / 发货订单数',
      '漏发率 = 漏发订单数 / 发货订单数',
      '复核覆盖率 = 经过复核流程的发货单数 / 发货单数',
    ]}
  />
);
