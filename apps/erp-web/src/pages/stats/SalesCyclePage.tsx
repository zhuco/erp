import { StatsPlaceholder } from '@/pages/stats/StatsPlaceholder';

export const SalesCyclePage = () => (
  <StatsPlaceholder
    title="销售周期"
    statsType="sales-cycle"
    dataSources={['下单时间', '发货时间', '签收时间', '物流节点']}
    metrics={[
      { key: 'avg_order_cycle_hours', label: '下单到发货', unit: '小时', precision: 1 },
      { key: 'avg_ship_cycle_hours', label: '发货到签收', unit: '小时', precision: 1 },
      { key: 'avg_return_cycle_days', label: '退货周期', unit: '天', precision: 1 },
      { key: 'on_time_delivery_rate', label: '按时履约率', unit: '%' }
    ]}
    pendingReasons={['物流节点不完整', '周期口径待统一']}
  />
);
