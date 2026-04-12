import { Card, Space, Tabs, Typography } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import { OrderTagBadges } from '@/components/OrderTagBadges';
import { ServerTable } from '@/components/ServerTable';
import { useCan } from '@/features/rbac';
import { builtinOrderTags, type BuiltinOrderTag } from '@/features/orderTags';
import { OrderTagDefinitionsPage } from '@/pages/tags/OrderTagDefinitionsPage';
import { SkuTagDefinitionsPage } from '@/pages/tags/SkuTagDefinitionsPage';

import './tagDirectoryPage.css';

type TagDirectoryTab = 'orders' | 'skus' | 'builtin';

const BuiltinTagDirectory = () => {
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const columns = useMemo<ColumnsType<BuiltinOrderTag>>(
    () => [
      {
        title: '标记',
        dataIndex: 'code',
        width: 90,
        render: (_, record) => <OrderTagBadges tags={[record.name]} />
      },
      {
        title: '名称',
        dataIndex: 'name',
        width: 160
      },
      {
        title: '说明',
        dataIndex: 'description'
      }
    ],
    []
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card size="small">
        <Space direction="vertical" size={4}>
          <Typography.Text strong>内置订单标记</Typography.Text>
          <Typography.Text type="secondary">
            系统默认标记，用于快速识别特殊订单。
          </Typography.Text>
        </Space>
      </Card>
      <div className="tag-directory-table">
        <ServerTable<BuiltinOrderTag>
          rowKey="key"
          columns={columns}
          data={builtinOrderTags}
          loading={false}
          error={null}
          onRetry={() => undefined}
          emptyText="暂无内置标记"
          total={builtinOrderTags.length}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="tags.builtin.table.columns"
        />
      </div>
    </Space>
  );
};

export const TagDirectoryPage = () => {
  const canReadOrders = useCan('order_tags.read');
  const canReadSkus = useCan('sku_tags.read');
  const [searchParams, setSearchParams] = useSearchParams();

  const availableTabs = useMemo(() => {
    const items = [] as { key: TagDirectoryTab; label: string; children: React.ReactNode }[];
    if (canReadOrders) {
      items.push({
        key: 'orders',
        label: '订单标记',
        children: <OrderTagDefinitionsPage showTitle={false} />
      });
    }
    if (canReadSkus) {
      items.push({
        key: 'skus',
        label: '货品标记',
        children: <SkuTagDefinitionsPage showTitle={false} />
      });
    }
    items.push({
      key: 'builtin',
      label: '内置标记',
      children: <BuiltinTagDirectory />
    });
    return items;
  }, [canReadOrders, canReadSkus]);

  const activeKey = useMemo<TagDirectoryTab>(() => {
    const raw = searchParams.get('tab') as TagDirectoryTab | null;
    if (raw && availableTabs.some((tab) => tab.key === raw)) {
      return raw;
    }
    return availableTabs[0]?.key ?? 'builtin';
  }, [availableTabs, searchParams]);

  const handleTabChange = (key: string) => {
    const next = new URLSearchParams(searchParams);
    next.set('tab', key);
    setSearchParams(next, { replace: true });
  };

  return (
    <div className="tag-directory-page">
      <div className="tag-directory-toolbar">
        <Typography.Title level={3} style={{ margin: 0 }}>
          标记目录
        </Typography.Title>
      </div>
      <Tabs activeKey={activeKey} onChange={handleTabChange} items={availableTabs} />
    </div>
  );
};
