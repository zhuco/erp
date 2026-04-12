import { Card, Row, Space, Tag, Typography } from 'antd';

import { EmptyState } from '@/components/EmptyState';
import { LoadingSkeleton } from '@/components/LoadingSkeleton';

type PlaceholderPageProps = {
  title: string;
  description?: string;
  testId?: string;
  titleTestId?: string;
};

const PlaceholderPage = ({
  title,
  description = '页面建设中',
  testId,
  titleTestId
}: PlaceholderPageProps) => (
  <Space direction="vertical" size={16} style={{ width: '100%' }} data-testid={testId}>
    <Row align="middle" justify="space-between">
      <Typography.Title level={3} style={{ margin: 0 }} data-testid={titleTestId}>
        {title}
      </Typography.Title>
      <Tag>建设中</Tag>
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

export const InventoryPage = () => (
  <PlaceholderPage
    title="库存"
    description="库存能力建设中"
    testId="inventory-placeholder"
    titleTestId="inventory-title"
  />
);

export const WarehousesPage = () => <PlaceholderPage title="多仓" description="多仓管理建设中" />;

export const SkuPage = () => <PlaceholderPage title="商品SKU" description="SKU 管理建设中" />;

export const BundlePage = () => <PlaceholderPage title="组合装" description="组合装管理建设中" />;

export const PrintJobsPage = () => <PlaceholderPage title="打印" description="打印任务建设中" />;

export const SettingsPage = () => <PlaceholderPage title="设置" description="设置中心建设中" />;

export const FailureQueuePage = () => <PlaceholderPage title="失败队列" description="失败队列监控建设中" />;
