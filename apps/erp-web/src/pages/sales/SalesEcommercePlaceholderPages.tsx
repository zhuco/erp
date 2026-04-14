import { Card, Row, Space, Tag, Typography } from 'antd';

import { EmptyState } from '@/components/EmptyState';
import { LoadingSkeleton } from '@/components/LoadingSkeleton';

type EcommercePlaceholderPageProps = {
  title: string;
  description: string;
  testId: string;
};

const EcommercePlaceholderPage = ({
  title,
  description,
  testId,
}: EcommercePlaceholderPageProps) => (
  <Space direction="vertical" size={16} style={{ width: '100%' }} data-testid={testId}>
    <Row align="middle" justify="space-between">
      <Typography.Title level={3} style={{ margin: 0 }}>
        {title}
      </Typography.Title>
      <Tag color="blue">建设中</Tag>
    </Row>
    <Card>
      <LoadingSkeleton rows={2} />
    </Card>
    <Card>
      <LoadingSkeleton rows={4} />
    </Card>
    <Card>
      <EmptyState description={description} />
    </Card>
  </Space>
);

export const ShopOrderDownloadPage = () => (
  <EcommercePlaceholderPage
    title="网店订单下载"
    description="网店订单下载页面建设中"
    testId="shop-order-download-placeholder"
  />
);

export const ShopProductMatchQueryPage = () => (
  <EcommercePlaceholderPage
    title="网店商品匹配查询"
    description="网店商品匹配查询页面建设中"
    testId="shop-product-match-query-placeholder"
  />
);

export const ShopInventorySyncPage = () => (
  <EcommercePlaceholderPage
    title="网店库存同步"
    description="网店库存同步页面建设中"
    testId="shop-inventory-sync-placeholder"
  />
);
