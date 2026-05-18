import {
  Alert,
  Button,
  Checkbox,
  Col,
  Form,
  Input,
  Row,
  Select,
  Space,
  Typography
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type { InventoryCostRecord } from '@/api/inventory';
import { listInventoryCosts } from '@/api/inventory';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  sku_code?: string;
  barcode?: string;
  tag?: string;
  on_hand_negative?: boolean;
  available_below_safety?: boolean;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : `¥${(value / 100).toFixed(2)}`;

export const InventoryCostsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const navigate = useNavigate();
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const costFeature = useFeatureFlag('cost.enabled');
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'inventory-costs'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const costEnabled = costFeature.enabled;

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const query = useMemo(
    () => ({
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      sku_code: filters.sku_code?.trim() || undefined,
      barcode: filters.barcode?.trim() || undefined,
      tag: filters.tag?.trim() || undefined,
      on_hand_negative: filters.on_hand_negative ? true : undefined,
      available_below_safety: filters.available_below_safety ? true : undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const costQuery = useQuery({
    queryKey: ['inventory-costs', tenantId, query],
    queryFn: () => listInventoryCosts(tenantId, query),
    enabled: tenantReady && inventoryFeature.enabled && costEnabled,
    retry: false
  });

  const columns = useMemo<ColumnsType<InventoryCostRecord>>(
    () => [
      {
        title: 'SKU 编码',
        dataIndex: 'sku_code'
      },
      {
        title: 'SKU ID',
        dataIndex: 'sku_id'
      },
      {
        title: '库存余额',
        key: 'on_hand',
        render: (_, record) =>
          record.on_hand ?? record.available + record.reserved + (record.frozen ?? 0)
      },
      {
        title: '预占',
        dataIndex: 'reserved'
      },
      {
        title: '冻结',
        dataIndex: 'frozen',
        render: (value) => value ?? 0
      },
      {
        title: '在途',
        dataIndex: 'in_transit',
        render: (value) => value ?? 0
      },
      {
        title: '可用',
        dataIndex: 'available'
      },
      {
        title: '单位成本',
        dataIndex: 'unit_cost_cent',
        render: (value: number) => formatCurrency(value)
      },
      {
        title: '成本合计',
        dataIndex: 'total_cost_cent',
        render: (value: number) => formatCurrency(value)
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value) => formatDateTime(value)
      }
    ],
    []
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

  const showCostDisabled = inventoryFeature.enabled && !costEnabled;

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        库存成本
      </Typography.Title>
      <Typography.Text type="secondary">
        统一查看库存数量、可用量、冻结量与成本余额
      </Typography.Text>
    </Space>
  );

  const toolbarRight = (
    <Space align="center" size={8} wrap>
      <Button onClick={() => costQuery.refetch()} disabled={!inventoryFeature.enabled || !costEnabled}>
        刷新
      </Button>
      <Button onClick={() => navigate('/inventory')}>返回库存</Button>
    </Space>
  );

  const tableHeader = (
    <Form form={form} layout="vertical" onFinish={handleSearch}>
      <Row gutter={[16, 16]}>
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
          <Form.Item name="sku_code" label="SKU 编码">
            <Input allowClear placeholder="输入 SKU 编码" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="barcode" label="条码">
            <Input allowClear placeholder="输入条码" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="tag" label="标签">
            <Input allowClear placeholder="输入标签" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="on_hand_negative" valuePropName="checked" label=" ">
            <Checkbox>库存 &lt; 0</Checkbox>
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="available_below_safety" valuePropName="checked" label=" ">
            <Checkbox>可用 &lt; 安全库存</Checkbox>
          </Form.Item>
        </Col>
        <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
          <Space>
            <Button type="primary" htmlType="submit">
              查询
            </Button>
            <Button onClick={handleReset}>重置</Button>
          </Space>
        </Col>
      </Row>
    </Form>
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      {!inventoryFeature.enabled ? (
        <Alert
          type="warning"
          showIcon
          message="库存管理已关闭"
          description="库存成本已暂停，如需使用请先在设置中开启。"
        />
      ) : null}
      {showCostDisabled ? (
        <Alert
          type="warning"
          showIcon
          message="库存成本未开启"
          description="请在系统设置中开启成本管理。"
        />
      ) : null}
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
      {inventoryFeature.enabled && costEnabled ? (
        <UiModule03MainTableSection<InventoryCostRecord>
          dataTestId="inventory-costs-main-grid"
          toolbarLeft={toolbarLeft}
          toolbarRight={toolbarRight}
          header={tableHeader}
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
          rowKey={(record) => record.sku_id}
          columns={columns}
          data={costQuery.data?.items ?? []}
          loading={costQuery.isFetching}
          error={costQuery.error}
          onRetry={() => costQuery.refetch()}
          errorTitle="库存成本加载失败"
          emptyText="暂无库存成本记录"
          total={costQuery.data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="inventory.costs.table.columns"
          showColumnSettings
          virtual
          enablePanels={false}
        />
      ) : null}
    </Space>
  );
};
