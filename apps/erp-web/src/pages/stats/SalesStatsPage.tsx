import { StatsPlaceholder } from '@/pages/stats/StatsPlaceholder';

const formatCurrency = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const SalesStatsPage = () => (
  <div data-testid="stats-sales-page">
    <StatsPlaceholder
      title="销售统计"
      statsType="sales"
      dataSources={['渠道订单', '退款与售后单', '商品档案']}
      metrics={[
        { key: 'sales_amount_cent', label: '销售额', format: formatCurrency },
        { key: 'order_count', label: '订单量', unit: '单' },
        { key: 'avg_order_value_cent', label: '客单价', format: formatCurrency },
        { key: 'refund_rate', label: '退款率', unit: '%' }
      ]}
      pendingReasons={['多渠道订单汇总在途', '退款统计口径待统一']}
    />
  </div>
);
