import { Card, Tabs, Typography } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import type { ReactNode } from 'react';
import { useSearchParams } from 'react-router-dom';

import { OrderTagBadges } from '@/components/OrderTagBadges';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { builtinOrderTags, type BuiltinOrderTag } from '@/features/orderTags';
import { OrderTagDefinitionsPage } from '@/pages/tags/OrderTagDefinitionsPage';
import { SkuTagDefinitionsPage } from '@/pages/tags/SkuTagDefinitionsPage';

import './tagDirectoryPage.css';

type TagDirectoryTab = 'orders' | 'skus' | 'builtin';

const BuiltinTagDirectory = () => {
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });

  const columns = useMemo<ColumnsType<BuiltinOrderTag>>(
    () => [
      {
        title: 'Tag',
        dataIndex: 'code',
        width: 90,
        render: (_, record) => <OrderTagBadges tags={[record.name]} />,
      },
      {
        title: 'Name',
        dataIndex: 'name',
        width: 160,
      },
      {
        title: 'Description',
        dataIndex: 'description',
      },
    ],
    [],
  );

  const header = (
    <Card size="small">
      <Typography.Text strong>Builtin order tags</Typography.Text>
      <br />
      <Typography.Text type="secondary">System default tags for fast order identification.</Typography.Text>
    </Card>
  );

  return (
    <div className="tag-directory-table">
      <UiModule03MainTableSection<BuiltinOrderTag>
        header={header}
        rowKey="key"
        columns={columns}
        data={builtinOrderTags}
        loading={false}
        error={null}
        onRetry={() => undefined}
        emptyText="No builtin tags"
        total={builtinOrderTags.length}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="tags.builtin.table.columns"
      />
    </div>
  );
};

export const TagDirectoryPage = () => {
  const canReadOrders = useCan('order_tags.read');
  const canReadSkus = useCan('sku_tags.read');
  const [searchParams, setSearchParams] = useSearchParams();

  const availableTabs = useMemo(() => {
    const items = [] as { key: TagDirectoryTab; label: string; children: ReactNode }[];
    if (canReadOrders) {
      items.push({
        key: 'orders',
        label: 'Order Tags',
        children: <OrderTagDefinitionsPage showTitle={false} />,
      });
    }
    if (canReadSkus) {
      items.push({
        key: 'skus',
        label: 'SKU Tags',
        children: <SkuTagDefinitionsPage showTitle={false} />,
      });
    }
    items.push({
      key: 'builtin',
      label: 'Builtin Tags',
      children: <BuiltinTagDirectory />,
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
          Tag Directory
        </Typography.Title>
      </div>
      <Tabs activeKey={activeKey} onChange={handleTabChange} items={availableTabs} />
    </div>
  );
};
