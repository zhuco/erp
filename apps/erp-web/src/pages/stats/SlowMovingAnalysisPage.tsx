import { StatsPlaceholder } from '@/pages/stats/StatsPlaceholder';

const formatCurrency = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const SlowMovingAnalysisPage = () => (
  <StatsPlaceholder
    title="滞销分析"
    statsType="slow-moving"
    dataSources={['SKU销量', '库存周转', '滞销规则']}
    metrics={[
      { key: 'slow_sku_count', label: '滞销SKU数', unit: '个' },
      { key: 'slow_stock_amount_cent', label: '滞销库存额', format: formatCurrency },
      { key: 'avg_turnover_days', label: '平均周转天数', unit: '天', precision: 1 },
      { key: 'dead_stock_ratio', label: '呆滞占比', unit: '%' }
    ]}
    pendingReasons={['滞销规则未配置', '库存周转接口待接入']}
  />
);
