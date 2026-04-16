import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const TransferBenefitPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-inventory-transfer-benefit-page"
    title="调拨效益"
    goal="分析调拨行为是否改善库存平衡与履约效率，避免高成本低收益调拨。"
    phase="phase-2"
    dimensions={['调出仓', '调入仓', 'SKU', '调拨类型', '在途状态']}
    metrics={['调拨量', '在途量', '在途天数', '调拨成本', '调拨收益差', '调拨完成率']}
    dependencies={['调拨单', '在途记录', '缺货预警', '履约结果']}
    caliberNotes={[
      '调拨完成率 = 已完成调拨单量 / 调拨单量',
      '调拨收益差可先按缺货减少量或履约成功率改善量估算',
      '收益数据缺失时先展示调拨效率指标',
    ]}
  />
);
