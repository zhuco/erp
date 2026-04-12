import { Button, Card, Col, DatePicker, Form, Input, Row, Space, Tabs } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type {
  CustomerPointLedger,
  CustomerStoredValueLedger
} from '@/api/customerLoyalty';
import {
  listCustomerPointLedgers,
  listCustomerStoredValueLedgers
} from '@/api/customerLoyalty';
import { ServerTable } from '@/components/ServerTable';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';

type LedgerFilters = {
  keyword?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : `¥${(value / 100).toFixed(2)}`;

const formatDelta = (value?: number | null) => {
  if (value == null) {
    return '-';
  }
  return value > 0 ? `+${value}` : `${value}`;
};

const PointsLedgerPanel = () => {
  const tenantId = useTenantId();
  const [form] = Form.useForm<LedgerFilters>();
  const [filters, setFilters] = useState<LedgerFilters>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      created_from: filters.created_range?.[0]?.toISOString(),
      created_to: filters.created_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['customer-point-ledgers', tenantId, query],
    queryFn: () => listCustomerPointLedgers(tenantId, query),
    keepPreviousData: true
  });

  const columns = useMemo<ColumnsType<CustomerPointLedger>>(
    () => [
      {
        title: '客户',
        dataIndex: 'customer_name'
      },
      {
        title: '积分变动',
        dataIndex: 'change_points',
        render: (value) => formatDelta(value)
      },
      {
        title: '积分余额',
        dataIndex: 'balance_points'
      },
      {
        title: '原因',
        dataIndex: 'reason'
      },
      {
        title: '关联单据',
        dataIndex: 'ref_id',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (value: string) => formatDateTime(value)
      }
    ],
    []
  );

  const handleSearch = (values: LedgerFilters) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="客户/手机号/ID" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="created_range" label="时间范围">
                <DatePicker.RangePicker
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['开始时间', '结束时间']}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      <ServerTable<CustomerPointLedger>
        rowKey="ledger_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="加载积分流水失败"
        emptyText="暂无积分流水"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="customer.points.table.columns"
      />
    </Space>
  );
};

const StoredValueLedgerPanel = () => {
  const tenantId = useTenantId();
  const [form] = Form.useForm<LedgerFilters>();
  const [filters, setFilters] = useState<LedgerFilters>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      created_from: filters.created_range?.[0]?.toISOString(),
      created_to: filters.created_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['customer-stored-value-ledgers', tenantId, query],
    queryFn: () => listCustomerStoredValueLedgers(tenantId, query),
    keepPreviousData: true
  });

  const columns = useMemo<ColumnsType<CustomerStoredValueLedger>>(
    () => [
      {
        title: '客户',
        dataIndex: 'customer_name'
      },
      {
        title: '金额变动',
        dataIndex: 'change_cent',
        render: (value) => formatCurrency(value)
      },
      {
        title: '余额',
        dataIndex: 'balance_cent',
        render: (value) => formatCurrency(value)
      },
      {
        title: '原因',
        dataIndex: 'reason'
      },
      {
        title: '关联单据',
        dataIndex: 'ref_id',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (value: string) => formatDateTime(value)
      }
    ],
    []
  );

  const handleSearch = (values: LedgerFilters) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="客户/手机号/ID" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="created_range" label="时间范围">
                <DatePicker.RangePicker
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['开始时间', '结束时间']}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      <ServerTable<CustomerStoredValueLedger>
        rowKey="ledger_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="加载储值流水失败"
        emptyText="暂无储值流水"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="customer.storedValue.table.columns"
      />
    </Space>
  );
};

export const CustomerLoyaltyPage = () => (
  <UiModule07SettingsLayout title="积分与储值">
    <UiModule07SettingsSection title="账户流水">
      <Tabs
        items={[
          {
            key: 'points',
            label: '积分流水',
            children: <PointsLedgerPanel />
          },
          {
            key: 'stored-value',
            label: '储值流水',
            children: <StoredValueLedgerPanel />
          }
        ]}
      />
    </UiModule07SettingsSection>
  </UiModule07SettingsLayout>
);
