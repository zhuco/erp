import {
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import { ReloadOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { PurchaseOrderCreatePayload } from '@/api/purchaseOrders';
import { createPurchaseOrder } from '@/api/purchaseOrders';
import type { PurchaseSuggestion, PurchaseSuggestionListQuery } from '@/api/purchaseSuggestions';
import { listPurchaseSuggestions } from '@/api/purchaseSuggestions';
import { listSuppliers } from '@/api/suppliers';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type CreateFormValues = {
  supplier_id: string;
  warehouse_id: string;
  expected_at?: dayjs.Dayjs;
  remark?: string;
  qty: number;
  price_cent: number;
};

type FilterValues = {
  sku_id?: string;
  reason?: string;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const reasonColors: Record<string, string> = {
  sales_velocity: 'blue',
  safety_stock: 'purple',
  low_stock: 'orange'
};

export const PurchaseSuggestionsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWrite = useCan('purchase_orders.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [activeSuggestion, setActiveSuggestion] = useState<PurchaseSuggestion | null>(null);
  const [filterForm] = Form.useForm<FilterValues>();
  const [createForm] = Form.useForm<CreateFormValues>();

  const query = useMemo<PurchaseSuggestionListQuery>(
    () => ({
      sku_id: filters.sku_id?.trim() || undefined,
      reason: filters.reason,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const suggestionQuery = useQuery({
    queryKey: ['purchase-suggestions', tenantId, query],
    queryFn: () => listPurchaseSuggestions(tenantId, query),
    enabled: tenantReady
  });

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'suggestions'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'purchase-suggestions'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const supplierOptions = useMemo(
    () =>
      supplierQuery.data?.items.map((supplier) => ({
        label: supplier.name,
        value: supplier.supplier_id
      })) ?? [],
    [supplierQuery.data?.items]
  );

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const reasonOptions = useMemo(
    () => [
      { value: 'sales_velocity', label: 'Sales Velocity' },
      { value: 'safety_stock', label: 'Safety Stock' },
      { value: 'low_stock', label: 'Low Stock' }
    ],
    []
  );

  const createMutation = useMutation({
    mutationFn: (payload: PurchaseOrderCreatePayload) => createPurchaseOrder(tenantId, payload),
    onSuccess: () => {
      message.success('Purchase order created');
      setCreateOpen(false);
      setActiveSuggestion(null);
      createForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['purchase-orders', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to create purchase order');
    }
  });

  const columns = useMemo<ColumnsType<PurchaseSuggestion>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: 'SKU Code', dataIndex: 'sku_code' },
      { title: 'Suggested Qty', dataIndex: 'suggested_qty' },
      {
        title: 'Reason',
        dataIndex: 'reason',
        render: (value: string) => <Tag color={reasonColors[value] || 'default'}>{value}</Tag>
      },
      {
        title: 'Computed At',
        dataIndex: 'computed_at',
        render: formatDateTime
      },
      {
        title: 'Actions',
        key: 'actions',
        render: (_, record) => (
          <Button
            size="small"
            disabled={!canWrite}
            onClick={() => {
              setActiveSuggestion(record);
              setCreateOpen(true);
              createForm.setFieldsValue({
                supplier_id: undefined,
                warehouse_id: warehouseOptions[0]?.value,
                expected_at: undefined,
                remark: undefined,
                qty: record.suggested_qty,
                price_cent: 0
              });
            }}
          >
            Create PO
          </Button>
        )
      }
    ],
    [canWrite, createForm, warehouseOptions]
  );

  const handleCreate = async () => {
    if (!activeSuggestion) {
      return;
    }
    const values = await createForm.validateFields();
    const expectedAt = values.expected_at ? values.expected_at.toISOString() : undefined;
    createMutation.mutate({
      supplier_id: values.supplier_id,
      warehouse_id: values.warehouse_id,
      expected_at: expectedAt,
      remark: values.remark?.trim(),
      items: [
        {
          sku_id: activeSuggestion.sku_id,
          qty: values.qty,
          price_cent: values.price_cent
        }
      ]
    });
  };

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    filterForm.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleRefresh = () => {
    suggestionQuery.refetch();
    supplierQuery.refetch();
    warehouseQuery.refetch();
  };

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Row justify="space-between" align="middle">
        <Typography.Title level={3} style={{ margin: 0 }}>
          Purchase Suggestions
        </Typography.Title>
        <Button icon={<ReloadOutlined />} onClick={handleRefresh}>
          Refresh
        </Button>
      </Row>
      <Card>
        <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={8}>
              <Form.Item name="sku_id" label="SKU ID">
                <Input allowClear placeholder="Enter SKU ID" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="reason" label="Reason">
                <Select allowClear options={reasonOptions} placeholder="Select reason" />
              </Form.Item>
            </Col>
            <Col
              xs={24}
              md={10}
              style={{ display: 'flex', alignItems: 'end', justifyContent: 'flex-end' }}
            >
              <Space>
                <Button type="primary" htmlType="submit">
                  Search
                </Button>
                <Button onClick={handleReset}>Reset</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<PurchaseSuggestion>
        header={header}
        rowKey="sku_id"
        columns={columns}
        data={suggestionQuery.data?.items ?? []}
        loading={suggestionQuery.isFetching}
        error={suggestionQuery.error}
        onRetry={() => suggestionQuery.refetch()}
        errorTitle="Failed to load purchase suggestions"
        emptyText="No suggestions"
        total={suggestionQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="purchase-suggestions.table.columns"
      />
      <Modal
        title="Create Purchase Order"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={handleCreate}
        confirmLoading={createMutation.isPending}
        okText="Create"
        destroyOnHidden
      >
        <Form form={createForm} layout="vertical">
          <Form.Item
            name="supplier_id"
            label="Supplier"
            rules={[{ required: true, message: 'Select supplier' }]}
          >
            <Select options={supplierOptions} placeholder="Select supplier" />
          </Form.Item>
          <Form.Item
            name="warehouse_id"
            label="Warehouse"
            rules={[{ required: true, message: 'Select warehouse' }]}
          >
            <Select
              options={warehouseOptions}
              placeholder="Select warehouse"
              loading={warehouseQuery.isFetching}
            />
          </Form.Item>
          <Form.Item name="expected_at" label="Expected At">
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="remark" label="Remark">
            <Input.TextArea rows={3} placeholder="Optional remark" />
          </Form.Item>
          <Form.Item
            name="qty"
            label="Quantity"
            rules={[{ required: true, message: 'Enter qty' }]}
          >
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="price_cent"
            label="Price (cent)"
            rules={[{ required: true, message: 'Enter price' }]}
          >
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
