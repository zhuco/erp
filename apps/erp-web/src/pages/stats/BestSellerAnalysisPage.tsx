import { StatsPlaceholder } from '@/pages/stats/StatsPlaceholder';

const formatCurrency = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const BestSellerAnalysisPage = () => (
  <StatsPlaceholder
    title="畅销分析"
    statsType="best-sellers"
    dataSources={['SKU销量排行', '毛利明细', '评价数据']}
    metrics={[
      { key: 'top_sku_sales_cent', label: '畅销SKU销售额', format: formatCurrency },
      { key: 'top_sku_order_count', label: '畅销SKU订单量', unit: '单' },
      { key: 'top_sku_share', label: '畅销SKU贡献率', unit: '%' },
      { key: 'repeat_purchase_rate', label: '复购率', unit: '%' }
    ]}
    pendingReasons={['排行分析服务未接入', '评价数据未同步']}
  />
);
