import { StatsPlaceholder } from '@/pages/stats/StatsPlaceholder';

const formatCurrency = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const SalesDailyReportPage = () => (
  <StatsPlaceholder
    title="销售日报表"
    statsType="sales-daily"
    dataSources={['每日订单', '发货明细', '退款记录']}
    metrics={[
      { key: 'daily_sales_cent', label: '日GMV', format: formatCurrency },
      { key: 'daily_order_count', label: '日订单数', unit: '单' },
      { key: 'daily_refund_rate', label: '日退款率', unit: '%' },
      { key: 'daily_on_time_rate', label: '日发货及时率', unit: '%' }
    ]}
    pendingReasons={['日报汇总任务未接入', '统计任务调度待配置']}
  />
);
