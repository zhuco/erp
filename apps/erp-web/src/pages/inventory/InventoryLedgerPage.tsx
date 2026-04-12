import { Button, Card, Col, DatePicker, Form, Input, Row, Select, Space, Tag, Typography } from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type { InventoryLedger } from '@/api/inventoryLedgers';
import { listInventoryLedgers } from '@/api/inventoryLedgers';
import { listWarehouses } from '@/api/warehouses';
import { EmptyState } from '@/components/EmptyState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  sku_id?: string;
  sku_code?: string;
  warehouse_id?: string;
  time_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const reasonColors: Record<string, string> = {
  STOCK_COUNT_ADJUST: 'blue',
  PURCHASE_RECEIPT: 'green',
  SALES_SHIP: 'volcano',
  RESERVATION_CREATE: 'orange',
  RESERVATION_RELEASE: 'green',
  RETURN_IN: 'purple',
  TRANSFER_IN: 'cyan',
  TRANSFER_OUT: 'magenta'
};

export const InventoryLedgerPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'ledger'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const query = useMemo(
    () => ({
      sku_id: filters.sku_id?.trim() || undefined,
      sku_code: filters.sku_code?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      occurred_from: filters.time_range?.[0]?.toISOString(),
      occurred_to: filters.time_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const enabled = Boolean(query.sku_id || query.sku_code || query.warehouse_id);
  const ledgerQuery = useQuery({
    queryKey: ['inventory-ledgers', tenantId, query],
    queryFn: () => listInventoryLedgers(tenantId, query),
    enabled: tenantReady && enabled && inventoryFeature.enabled,
    retry: false
  });

  const guidanceMessage = !inventoryFeature.enabled
    ? '库存管理已关闭，库存流水查询不可用。'
    : !enabled
      ? '请输入 SKU 或选择仓库进行查询。'
      : null;
  const showTable = inventoryFeature.enabled && enabled;

  const columns = useMemo<ColumnsType<InventoryLedger>>(
    () => [
      {
        title: '类型',
        dataIndex: 'reason_type',
        render: (value: string) => <Tag color={reasonColors[value] ?? 'default'}>{value}</Tag>
      },
      {
        title: '在库变化',
        dataIndex: 'delta_on_hand'
      },
      {
        title: '预占变化',
        dataIndex: 'delta_reserved'
      },
      {
        title: '冻结变化',
        dataIndex: 'delta_frozen'
      },
      {
        title: '在途变化',
        dataIndex: 'delta_in_transit'
      },
      {
        title: '关联类型',
        dataIndex: 'ref_type',
        render: (value) => value ?? '-'
      },
      {
        title: '关联单号',
        dataIndex: 'ref_id',
        render: (value) => value ?? '-'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: 'SKU ID',
        dataIndex: 'sku_id'
      },
      {
        title: 'SKU 编码',
        dataIndex: 'sku_code'
      },
      {
        title: '发生时间',
        dataIndex: 'occurred_at',
        render: (value) => formatDateTime(value)
      }
    ],
    [warehouseMap]
  );

  const handleSearch = (values: FilterValues) => {
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
      <Row justify="space-between" align="middle">
        <Typography.Title level={3} style={{ margin: 0 }}>
          库存流水
        </Typography.Title>
        <Space>
          <Button onClick={() => navigate('/inventory')}>返回库存</Button>
        </Space>
      </Row>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={8}>
              <Form.Item name="sku_id" label="SKU ID">
                <Input allowClear placeholder="输入 SKU ID" data-testid="ledger-sku-id-input" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="sku_code" label="SKU 编码">
                <Input allowClear placeholder="输入 SKU 编码" data-testid="ledger-sku-code-input" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="仓库">
                <Select
                  allowClear
                  options={warehouseOptions}
                  placeholder="选择仓库"
                  loading={warehouseQuery.isFetching}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="time_range" label="时间范围">
                <DatePicker.RangePicker
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['开始时间', '结束时间']}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit" data-testid="ledger-search-submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      {guidanceMessage ? (
        <Card>
          <EmptyState description={guidanceMessage} />
        </Card>
      ) : null}
      {showTable ? (
        <UiModule03MainTableSection<InventoryLedger>
          rowKey="ledger_id"
          columns={columns}
          data={ledgerQuery.data?.items ?? []}
          loading={ledgerQuery.isFetching}
          error={ledgerQuery.error}
          onRetry={() => ledgerQuery.refetch()}
          errorTitle="库存流水加载失败"
          emptyText="暂无库存流水记录"
          total={ledgerQuery.data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="inventory.ledger.table.columns"
          enablePanels={false}
        />
      ) : null}
    </Space>
  );
};
