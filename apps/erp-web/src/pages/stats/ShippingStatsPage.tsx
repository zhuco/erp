import { StatsPlaceholder } from '@/pages/stats/StatsPlaceholder';

export const ShippingStatsPage = () => (
  <div data-testid="stats-shipping-page">
    <StatsPlaceholder
      title="发货统计"
      statsType="shipping"
      dataSources={['OMS 发货单', 'WMS 出库记录', '物流签收轨迹']}
      metrics={[
        { key: 'shipment_count', label: '发货单量', unit: '单' },
        { key: 'on_time_rate', label: '准时发货率', unit: '%' },
        { key: 'exception_count', label: '异常发货数', unit: '单' },
        { key: 'avg_outbound_hours', label: '平均出库时长', unit: '小时', precision: 1 }
      ]}
      pendingReasons={['物流轨迹对齐持续优化', '发货统计口径持续统一']}
    />
  </div>
);
