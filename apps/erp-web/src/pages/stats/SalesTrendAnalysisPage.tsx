import { StatsPlaceholder } from '@/pages/stats/StatsPlaceholder';

const formatCurrency = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const SalesTrendAnalysisPage = () => (
  <div data-testid="stats-sales-trends-page">
    <StatsPlaceholder
      title="销售趋势分析"
      statsType="sales-trends"
      dataSources={['历史订单', '营销活动', '库存变化']}
      metrics={[
        { key: 'yoy_growth_rate', label: '同比增长', unit: '%' },
        { key: 'mom_growth_rate', label: '环比增长', unit: '%' },
        { key: 'moving_avg_7d_cent', label: '7日移动均值', format: formatCurrency },
        { key: 'season_peak_cent', label: '时季峰值', format: formatCurrency },
        { key: 'channel_trend_index', label: '渠道趋势指数', unit: '点', precision: 1 }
      ]}
      pendingReasons={['历史数据聚合持续优化', '趋势模型参数持续校准']}
    />
  </div>
);
