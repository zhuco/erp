import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const NegativeStockAlertPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-inventory-negative-stock-alert-page"
    title="负库存预警"
    goal="发现并追踪负库存事件与恢复情况，降低库存风险和账实偏差。"
    phase="phase-2"
    dimensions={['仓库', 'SKU', '事务来源', '日期']}
    metrics={['负结余次数', '负结余量', '未恢复数量', '平均恢复时长']}
    dependencies={['库存流水', '异常规则', '库存余额快照']}
    caliberNotes={[
      '单次负库存事件定义为首次跌破 0 到恢复为非负的完整闭环',
      '平均恢复时长按闭环事件耗时均值计算',
      '未恢复数量为当前仍为负值的 SKU 数量',
    ]}
  />
);
