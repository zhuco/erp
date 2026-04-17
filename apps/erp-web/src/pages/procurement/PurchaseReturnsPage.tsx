import {
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import { ReloadOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  PurchaseReturnDetail,
  PurchaseReturnListQuery,
  PurchaseReturnSummary
} from '@/api/purchaseReturns';
import {
  confirmPurchaseReturn,
  createPurchaseReturn,
  getPurchaseReturnDetail,
  listPurchaseReturns
} from '@/api/purchaseReturns';
import { listSuppliers } from '@/api/suppliers';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type CreateFormValues = {
  po_id: string;
  warehouse_id: string;
  items: Array<{ sku_id: string; qty: number }>;
};

type FilterValues = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const statusOptions = [
  { label: 'DRAFT', value: 'DRAFT' },
  { label: 'DONE', value: 'DONE' }
];

export const PurchaseReturnsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWrite = useCan('purchase_returns.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeDetail, setActiveDetail] = useState<PurchaseReturnDetail | null>(null);
  const [form] = Form.useForm<CreateFormValues>();
  const [filterForm] = Form.useForm<FilterValues>();

  const query = useMemo<PurchaseReturnListQuery>(
    () => ({
      supplier_id: filters.supplier_id?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      created_from: filters.created_range?.[0]?.toISOString(),
      created_to: filters.created_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['purchase-returns', tenantId, query],
    queryFn: () => listPurchaseReturns(tenantId, query),
    enabled: tenantReady
  });

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'purchase-returns'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'purchase-returns'],
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

  const createMutation = useMutation({
    mutationFn: (payload: CreateFormValues) => createPurchaseReturn(tenantId, payload),
    onSuccess: () => {
      message.success('Purchase return created');
      setCreateOpen(false);
      form.resetFields();
      queryClient.invalidateQueries({ queryKey: ['purchase-returns', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to create return');
    }
  });

  const confirmMutation = useMutation({
    mutationFn: (returnId: string) => confirmPurchaseReturn(tenantId, returnId),
    onSuccess: () => {
      message.success('Purchase return confirmed');
      queryClient.invalidateQueries({ queryKey: ['purchase-returns', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to confirm return');
    }
  });

  const columns = useMemo<ColumnsType<PurchaseReturnSummary>>(
    () => [
      { title: 'Return ID', dataIndex: 'return_id' },
      { title: 'PO ID', dataIndex: 'po_id' },
      {
        title: 'Supplier',
        dataIndex: 'supplier_id',
        render: (_, record) =>
          record.supplier_name ?? (record.supplier_id ? supplierMap.get(record.supplier_id) : '-') ?? '-'
      },
      {
        title: 'Warehouse',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: 'Status',
        dataIndex: 'status',
        render: (value: string) => <Tag color={value === 'DONE' ? 'green' : 'default'}>{value}</Tag>
      },
      {
        title: 'Confirmed At',
        dataIndex: 'confirmed_at',
        render: formatDateTime
      },
      {
        title: 'Created At',
        dataIndex: 'created_at',
        render: formatDateTime
      },
      {
        title: 'Actions',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" onClick={() => handleViewDetail(record.return_id)}>
              Detail
            </Button>
            <Button
              size="small"
              disabled={!canWrite || record.status !== 'DRAFT'}
              loading={confirmMutation.isPending}
              onClick={() => confirmMutation.mutate(record.return_id)}
            >
              Confirm
            </Button>
          </Space>
        )
      }
    ],
    [canWrite, confirmMutation, supplierMap, warehouseMap]
  );

  const detailColumns = useMemo<ColumnsType<PurchaseReturnDetail['items'][number]>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: 'Qty', dataIndex: 'qty' }
    ],
    []
  );

  const openCreate = () => {
    setCreateOpen(true);
    form.resetFields();
    form.setFieldsValue({ items: [{ sku_id: '', qty: 1 }] });
  };

  const handleCreate = async () => {
    const values = await form.validateFields();
    createMutation.mutate(values);
  };

  const handleViewDetail = async (returnId: string) => {
    try {
      const detail = await getPurchaseReturnDetail(tenantId, returnId);
      setActiveDetail(detail);
      setDetailOpen(true);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'Failed to load return detail');
    }
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
    listQuery.refetch();
    supplierQuery.refetch();
    warehouseQuery.refetch();
  };

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Row justify="space-between" align="middle">
        <Typography.Title level={3} style={{ margin: 0 }}>
          Purchase Returns
        </Typography.Title>
        <Space>
          <Button type="primary" disabled={!canWrite} onClick={openCreate}>
            New Return
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
      <UiModule03MainTableSection<PurchaseReturnSummary>
        header={header}
        rowKey="return_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="Failed to load purchase returns"
        emptyText="No returns"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="purchase-returns.table.columns"
      />
      <Modal
        title="Create Purchase Return"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={handleCreate}
        confirmLoading={createMutation.isPending}
        okText="Create"
        destroyOnHidden
      >
        <Form form={form} layout="vertical" initialValues={{ items: [{ sku_id: '', qty: 1 }] }}>
          <Form.Item
            name="po_id"
            label="PO ID"
            rules={[{ required: true, message: 'Enter PO ID' }]}
          >
            <Input placeholder="Purchase order ID" />
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
                    {fields.length > 1 ? (
                      <Button type="link" onClick={() => remove(field.name)}>
                        Remove
                      </Button>
                    ) : null}
                    {index === fields.length - 1 ? (
                      <Button type="link" onClick={() => add({ sku_id: '', qty: 1 })}>
                        Add
                      </Button>
                    ) : null}
                  </Space>
                ))}
              </Space>
            )}
          </Form.List>
        </Form>
      </Modal>
      <Drawer
        title="Purchase Return Detail"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={520}
      >
        {activeDetail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Descriptions bordered size="small" column={1}>
              <Descriptions.Item label="Return ID">
                {activeDetail.purchase_return.return_id}
              </Descriptions.Item>
              <Descriptions.Item label="PO ID">
                {activeDetail.purchase_return.po_id}
              </Descriptions.Item>
              <Descriptions.Item label="Supplier">
                {activeDetail.purchase_return.supplier_name ??
                  (activeDetail.purchase_return.supplier_id
                    ? supplierMap.get(activeDetail.purchase_return.supplier_id) ??
                      activeDetail.purchase_return.supplier_id
                    : '-')}
              </Descriptions.Item>
              <Descriptions.Item label="Warehouse">
                {warehouseMap.get(activeDetail.purchase_return.warehouse_id) ??
                  activeDetail.purchase_return.warehouse_id}
              </Descriptions.Item>
              <Descriptions.Item label="Status">
                {activeDetail.purchase_return.status}
              </Descriptions.Item>
              <Descriptions.Item label="Confirmed At">
                {formatDateTime(activeDetail.purchase_return.confirmed_at)}
              </Descriptions.Item>
              <Descriptions.Item label="Created At">
                {formatDateTime(activeDetail.purchase_return.created_at)}
              </Descriptions.Item>
            </Descriptions>
            <Table
              rowKey="sku_id"
              columns={detailColumns}
              dataSource={activeDetail.items}
              pagination={false}
              size="small"
            />
          </Space>
        ) : (
          <Typography.Text type="secondary">No detail</Typography.Text>
        )}
      </Drawer>
    </>
  );
};
