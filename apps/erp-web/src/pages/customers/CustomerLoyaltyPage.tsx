import { Button, Col, DatePicker, Form, Input, Row, Space, Tabs } from 'antd';
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
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { SettingsPageTemplate, SettingsPageSection } from '@/components/page-templates';
import { useTenantId } from '@/features/tenant';
import { IndustrialCard } from '@/components/industrial';

type LedgerFilters = {
  keyword?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : `楼${(value / 100).toFixed(2)}`;

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
  const [pagination, setPagination] = useState<IndustrialTablePaginationConfig>({
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
    placeholderData: (previousData) => previousData
  });

  const columns = useMemo<ColumnsType<CustomerPointLedger>>(
    () => [
      {
        title: '客户',
        dataIndex: 'customer_name'
      },
      {
        title: '绉垎鍙樺姩',
        dataIndex: 'change_points',
        render: (value) => formatDelta(value)
      },
      {
        title: '绉垎浣欓',
        dataIndex: 'balance_points'
      },
      {
        title: '鍘熷洜',
        dataIndex: 'reason'
      },
      {
        title: '关联单据',
        dataIndex: 'ref_id',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '鏃堕棿',
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
      <IndustrialCard>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name='keyword' label='Keyword'>
                <Input allowClear placeholder='Customer / phone / id' />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="created_range" label="鏃堕棿鑼冨洿">
                <DatePicker.RangePicker
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['Start time', 'End time']}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  鏌ヨ
                </Button>
                <Button onClick={handleReset}>閲嶇疆</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </IndustrialCard>
      <UiModule03MainTableSection<CustomerPointLedger>
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
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        dataTestId="industrial-main-grid"
      />
    </Space>
  );
};

const StoredValueLedgerPanel = () => {
  const tenantId = useTenantId();
  const [form] = Form.useForm<LedgerFilters>();
  const [filters, setFilters] = useState<LedgerFilters>({});
  const [pagination, setPagination] = useState<IndustrialTablePaginationConfig>({
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
    placeholderData: (previousData) => previousData
  });

  const columns = useMemo<ColumnsType<CustomerStoredValueLedger>>(
    () => [
      {
        title: '客户',
        dataIndex: 'customer_name'
      },
      {
        title: '閲戦鍙樺姩',
        dataIndex: 'change_cent',
        render: (value) => formatCurrency(value)
      },
      {
        title: '浣欓',
        dataIndex: 'balance_cent',
        render: (value) => formatCurrency(value)
      },
      {
        title: '鍘熷洜',
        dataIndex: 'reason'
      },
      {
        title: '关联单据',
        dataIndex: 'ref_id',
        render: (value?: string | null) => value ?? '-'
      },
      {
        title: '鏃堕棿',
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
      <IndustrialCard>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name='keyword' label='Keyword'>
                <Input allowClear placeholder='Customer / phone / id' />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="created_range" label="鏃堕棿鑼冨洿">
                <DatePicker.RangePicker
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['Start time', 'End time']}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  鏌ヨ
                </Button>
                <Button onClick={handleReset}>閲嶇疆</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </IndustrialCard>
      <UiModule03MainTableSection<CustomerStoredValueLedger>
        rowKey="ledger_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="Failed to load stored value ledgers"
        emptyText="No stored value ledgers"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="customer.storedValue.table.columns"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        dataTestId="industrial-main-grid"
      />
    </Space>
  );
};

export const CustomerLoyaltyPage = () => (
  <SettingsPageTemplate title="Points and Stored Value">
    <SettingsPageSection title="璐︽埛娴佹按">
      <Tabs
        items={[
          {
            key: 'points',
            label: '积分流水',
            children: <PointsLedgerPanel />
          },
          {
            key: 'stored-value',
            label: 'Stored Value Ledger',
            children: <StoredValueLedgerPanel />
          }
        ]}
      />
    </SettingsPageSection>
  </SettingsPageTemplate>
);

