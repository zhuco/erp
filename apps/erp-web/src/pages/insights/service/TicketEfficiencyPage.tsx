import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const TicketEfficiencyPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-service-ticket-efficiency-page"
    title="工单效率"
    goal="看清客服团队处理工单的响应与结案效率，持续跟踪 SLA 达成。"
    phase="phase-2"
    dimensions={['工单类型', '服务团队', '坐席', '优先级', '渠道']}
    metrics={['工单量', '首次响应时长', '结案时长', 'SLA 达成率', '转派率']}
    dependencies={['服务工单', '客服组织', 'SLA 配置']}
    caliberNotes={[
      '首次响应时长 = 工单创建到首次人工响应',
      'SLA 达成率 = 在 SLA 时间内关闭的工单数 / 已关闭工单数',
      '转派率 = 发生转派的工单数 / 工单总数',
    ]}
  />
);
