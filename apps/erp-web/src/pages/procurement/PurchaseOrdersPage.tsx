import {
  Button,
  Card,
  Col,
  DatePicker,
  Drawer,
  Form,
  Input,
  InputNumber,
  Popconfirm,
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
import { useNavigate } from 'react-router-dom';

import type {
  PurchaseOrderCreatePayload,
  PurchaseOrderDetail,
  PurchaseOrderItemPayload,
  PurchaseOrderListQuery,
  PurchaseOrderStatus,
  PurchaseOrderSummary,
  PurchaseOrderUpdatePayload
} from '@/api/purchaseOrders';
import {
  createPurchaseOrder,
  deletePurchaseOrder,
  getPurchaseOrder,
  listPurchaseOrders,
  sendPurchaseOrder,
  updatePurchaseOrder
} from '@/api/purchaseOrders';
import { listSuppliers } from '@/api/suppliers';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: PurchaseOrderStatus;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type FormValues = {
  supplier_id: string;
  warehouse_id: string;
  expected_at?: dayjs.Dayjs;
  remark?: string;
  items: PurchaseOrderItemPayload[];
};

const statusOptions: Array<{ label: string; value: PurchaseOrderStatus }> = [
  { label: 'DRAFT', value: 'DRAFT' },
  { label: 'SENT', value: 'SENT' },
  { label: 'CONFIRMED', value: 'CONFIRMED' },
  { label: 'RECEIVING', value: 'RECEIVING' },
  { label: 'DONE', value: 'DONE' },
  { label: 'CANCELLED', value: 'CANCELLED' }
];

const statusColors: Record<PurchaseOrderStatus, string> = {
  DRAFT: 'default',
  SENT: 'blue',
  CONFIRMED: 'purple',
  RECEIVING: 'gold',
  DONE: 'green',
  CANCELLED: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatAmount = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const PurchaseOrdersPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const canWrite = useCan('purchase_orders.write');
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingOrder, setEditingOrder] = useState<PurchaseOrderDetail | null>(null);
  const [saving, setSaving] = useState(false);
  const [sendingId, setSendingId] = useState<string | null>(null);

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'purchase-orders'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'purchase-orders'],
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

  const supplierMap = useMemo(() => {
    const entries = supplierQuery.data?.items ?? [];
    return new Map(entries.map((supplier) => [supplier.supplier_id, supplier.name]));
  }, [supplierQuery.data?.items]);

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

  const query = useMemo<PurchaseOrderListQuery>(
    () => ({
      supplier_id: filters.supplier_id?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status,
      created_from: filters.created_range?.[0]?.toISOString(),
      created_to: filters.created_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['purchase-orders', tenantId, query],
    queryFn: () => listPurchaseOrders(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (poId: string) => deletePurchaseOrder(tenantId, poId),
    onSuccess: () => {
      message.success('Purchase order deleted');
      queryClient.invalidateQueries({ queryKey: ['purchase-orders', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to delete purchase order');
    }
  });

  const sendMutation = useMutation({
    mutationFn: (poId: string) => sendPurchaseOrder(tenantId, poId),
    onMutate: (poId) => setSendingId(poId),
    onSuccess: () => {
      message.success('Purchase order sent');
      queryClient.invalidateQueries({ queryKey: ['purchase-orders', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to send purchase order');
    },
    onSettled: () => setSendingId(null)
  });

  const handleEdit = async (poId: string) => {
    try {
      const detail = await getPurchaseOrder(tenantId, poId);
      setEditingOrder(detail);
      setDrawerOpen(true);
      drawerForm.setFieldsValue({
        supplier_id: detail.order.supplier_id,
        warehouse_id: detail.order.warehouse_id ?? undefined,
        expected_at: detail.order.expected_at ? dayjs(detail.order.expected_at) : undefined,
        remark: detail.order.remark ?? undefined,
        items: detail.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          price_cent: item.price_cent
        }))
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'Failed to load purchase order');
    }
  };

  const columns = useMemo<ColumnsType<PurchaseOrderSummary>>(
    () => [
      {
        title: 'PO ID',
        dataIndex: 'po_id'
      },
      {
        title: 'Supplier',
        dataIndex: 'supplier_id',
        render: (_, record) => record.supplier_name ?? supplierMap.get(record.supplier_id) ?? '-'
      },
      {
        title: 'Warehouse',
        dataIndex: 'warehouse_id',
        render: (_, record) =>
          record.warehouse_id ? warehouseMap.get(record.warehouse_id) ?? record.warehouse_id : '-'
      },
      {
        title: 'Status',
        dataIndex: 'status',
        render: (value: PurchaseOrderStatus) => (
          <Tag color={statusColors[value] || 'default'}>{value}</Tag>
        )
      },
      {
        title: 'Total',
        dataIndex: 'total_cent',
        render: (value: number) => formatAmount(value)
      },
      {
        title: 'ETA',
        dataIndex: 'expected_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: 'Remark',
        dataIndex: 'remark',
        render: (value: string | null | undefined) => value || '-'
      },
      {
        title: 'Created At',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: 'Actions',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" onClick={() => navigate(`/procurement/orders/${record.po_id}`)}>
              Detail
            </Button>
            <Button
              size="small"
              disabled={!canWrite || record.status !== 'DRAFT'}
              onClick={() => handleEdit(record.po_id)}
            >
              Edit
            </Button>
            <Button
              size="small"
              disabled={!canWrite || record.status !== 'DRAFT'}
              loading={sendMutation.isPending && sendingId === record.po_id}
              onClick={() => sendMutation.mutate(record.po_id)}
            >
              Send
            </Button>
            <Popconfirm
              title="Confirm delete?"
              okText="Delete"
              cancelText="Cancel"
              onConfirm={() => deleteMutation.mutate(record.po_id)}
            >
              <Button size="small" danger disabled={!canWrite}>
                Delete
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation, navigate, sendMutation, sendingId, supplierMap, warehouseMap]
  );

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
    supplierQuery.refetch();
    warehouseQuery.refetch();
    listQuery.refetch();
  };

  const openCreateDrawer = () => {
    setEditingOrder(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      warehouse_id: warehouseOptions[0]?.value,
      items: [{ sku_id: '', qty: 1, price_cent: 0 }]
    });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const items = values.items.map((item) => ({
        sku_id: item.sku_id.trim(),
        qty: item.qty,
        price_cent: item.price_cent
      }));
      const expectedAt = values.expected_at ? values.expected_at.toISOString() : undefined;
      const payload: PurchaseOrderCreatePayload = {
        supplier_id: values.supplier_id,
        warehouse_id: values.warehouse_id,
        expected_at: expectedAt,
        remark: values.remark?.trim(),
        items
      };
      setSaving(true);
      if (editingOrder) {
        const updatePayload: PurchaseOrderUpdatePayload = {
          supplier_id: payload.supplier_id,
          warehouse_id: payload.warehouse_id,
          expected_at: payload.expected_at,
          remark: payload.remark,
          items
        };
        await updatePurchaseOrder(tenantId, editingOrder.order.po_id, updatePayload);
        message.success('Purchase order updated');
      } else {
        await createPurchaseOrder(tenantId, payload);
        message.success('Purchase order created');
      }
      setDrawerOpen(false);
      setEditingOrder(null);
      queryClient.invalidateQueries({ queryKey: ['purchase-orders', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Row justify="space-between" align="middle">
        <Typography.Title level={3} style={{ margin: 0 }}>
          Purchase Orders
        </Typography.Title>
        <Space>
          <Button type="primary" onClick={openCreateDrawer} disabled={!canWrite}>
            New Purchase Order
          </Button>
          <Button icon={<ReloadOutlined />} onClick={handleRefresh}>
            Refresh
          </Button>
        </Space>
      </Row>
      <Card>
        <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={6}>
              <Form.Item name="supplier_id" label="Supplier">
                <Select
                  allowClear
                  options={supplierOptions}
                  placeholder="Select supplier"
                  loading={supplierQuery.isFetching}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="Warehouse">
                <Select
                  allowClear
                  options={warehouseOptions}
                  placeholder="Select warehouse"
                  loading={warehouseQuery.isFetching}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={4}>
              <Form.Item name="status" label="Status">
                <Select allowClear options={statusOptions} placeholder="Select status" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="created_range" label="Created Range">
                <DatePicker.RangePicker showTime style={{ width: '100%' }} allowClear />
              </Form.Item>
            </Col>
            <Col
              xs={24}
              md={4}
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
      <UiModule03MainTableSection<PurchaseOrderSummary>
        header={header}
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="Failed to load purchase orders"
        emptyText="No purchase orders"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="purchase-orders.table.columns"
        rowKey="po_id"
      />
      <Drawer
        title={editingOrder ? 'Edit Purchase Order' : 'New Purchase Order'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={640}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>Cancel</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              Save
            </Button>
          </Space>
        }
      >
        <Form
          form={drawerForm}
          layout="vertical"
          initialValues={{ items: [{ sku_id: '', qty: 1, price_cent: 0 }] }}
        >
          <Form.Item
            name="supplier_id"
            label="Supplier"
            rules={[{ required: true, message: 'Please select supplier' }]}
          >
            <Select options={supplierOptions} placeholder="Select supplier" />
          </Form.Item>
          <Form.Item
            name="warehouse_id"
            label="Warehouse"
            rules={[{ required: true, message: 'Please select warehouse' }]}
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
          <Form.List name="items">
            {(fields, { add, remove }) => (
              <Space direction="vertical" style={{ width: '100%' }}>
                {fields.map((field, index) => (
                  <Space key={field.key} align="baseline" style={{ display: 'flex' }}>
                    <Form.Item
                      {...field}
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true, message: 'SKU required' }]}
                    >
                      <Input placeholder="SKU ID" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'qty']}
                      rules={[{ required: true, message: 'Qty required' }]}
                    >
                      <InputNumber min={1} />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'price_cent']}
                      rules={[{ required: true, message: 'Price required' }]}
                    >
                      <InputNumber min={0} />
                    </Form.Item>
                    {fields.length > 1 ? (
                      <Button type="link" onClick={() => remove(field.name)}>
                        Remove
                      </Button>
                    ) : null}
                    {index === fields.length - 1 ? (
                      <Button type="link" onClick={() => add({ sku_id: '', qty: 1, price_cent: 0 })}>
                        Add
                      </Button>
                    ) : null}
                  </Space>
                ))}
              </Space>
            )}
          </Form.List>
        </Form>
      </Drawer>
    </>
  );
};
