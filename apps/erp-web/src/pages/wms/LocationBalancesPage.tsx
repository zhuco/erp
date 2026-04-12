import { Button, Card, Col, Form, Input, Row, Space, Typography } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type {
  LocationBalance,
  LocationBalanceListQuery
} from '@/api/locationBalances';
import { listLocationBalances } from '@/api/locationBalances';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  location_id?: string;
  sku_id?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const LocationBalancesPage = () => {
  const tenantId = useTenantId();
  const [filterForm] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const query = useMemo<LocationBalanceListQuery>(
    () => ({
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      location_id: filters.location_id?.trim() || undefined,
      sku_id: filters.sku_id?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['location-balances', tenantId, query],
    queryFn: () => listLocationBalances(tenantId, query),
    keepPreviousData: true
  });

  const columns = useMemo<ColumnsType<LocationBalance>>(
    () => [
      { title: '仓库ID', dataIndex: 'warehouse_id' },
      { title: '货位ID', dataIndex: 'location_id' },
      { title: '货位编码', dataIndex: 'location_code', render: (value) => value ?? '-' },
      { title: 'SKU ID', dataIndex: 'sku_id' },
      { title: 'SKU编码', dataIndex: 'sku_code', render: (value) => value ?? '-' },
      { title: '库存数量', dataIndex: 'on_hand' },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      }
    ],
    []
  );

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      库位库存
    </Typography.Title>
  );

  const header = (
    <Card>
      <Form
        form={filterForm}
        layout="vertical"
        onFinish={(values) => {
          setFilters(values);
          setPagination((prev) => ({ ...prev, current: 1 }));
        }}
      >
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="warehouse_id" label="仓库ID">
              <Input allowClear placeholder="仓库ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="location_id" label="货位ID">
              <Input allowClear placeholder="货位ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="sku_id" label="SKU ID">
              <Input allowClear placeholder="SKU ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
            <Space>
              <Button type="primary" onClick={() => filterForm.submit()}>
                查询
              </Button>
              <Button
                onClick={() => {
                  filterForm.resetFields();
                  setFilters({});
                  setPagination((prev) => ({ ...prev, current: 1 }));
                }}
              >
                重置
              </Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
  );

  return (
    <UiModule03MainTableSection<LocationBalance>
      toolbarLeft={toolbarLeft}
      header={header}
      rowKey={(record) => `${record.location_id}-${record.sku_id}`}
      columns={columns}
      data={data?.items ?? []}
      loading={isLoading || isFetching}
      error={error}
      onRetry={() => refetch()}
      errorTitle="库位库存加载失败"
      emptyText="暂无库存记录"
      total={data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      columnSettingsKey="location-balances.table.columns"
    />
  );
};
