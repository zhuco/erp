import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const WarehouseNetworkEfficiencyPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-fulfillment-warehouse-network-efficiency-page"
    title="仓网效能"
    goal="衡量仓库、班组和人员单位时间内的产出效率，识别产能瓶颈。"
    phase="phase-2"
    dimensions={['仓库', '团队', '班次', '日期']}
    metrics={['处理单量', '处理行数', '人均产能', '时效达成率', '忙闲比']}
    dependencies={['作业日志', '班次记录', '人员排班', '发货与收货汇总']}
    caliberNotes={[
      '人均产能 = 处理行数 / 在岗人数',
      '忙闲比 = 峰值处理量 / 谷值处理量',
      '时效达成率沿用履约时效统一口径',
    ]}
  />
);
