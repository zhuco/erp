import { InsightPlaceholderPage } from '@/pages/insights/shared/InsightPlaceholderPage';

export const BundleComboPage = () => (
  <InsightPlaceholderPage
    data-testid="insights-catalog-bundle-combo-page"
    title="连带组合"
    goal="发现一起购买的商品组合，为搭配销售、推荐和组合装策略提供依据。"
    phase="phase-2"
    dimensions={['主 SKU', '连带 SKU', '品类', '店铺', '平台']}
    metrics={['共购次数', '连带率', '组合销售额', '组合订单量']}
    dependencies={['订单行', 'SKU 关系', '商品分类']}
    caliberNotes={[
      '连带率 = 同时购买连带 SKU 的订单数 / 购买主 SKU 的订单数',
      '组合销售额按同单内组合 SKU 的实付金额汇总',
      '可按主 SKU 视角和连带 SKU 视角双向查看',
    ]}
  />
);
