import {
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
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
import { useQuery } from '@tanstack/react-query';

import type {
  PurchaseReceiptDetail,
  PurchaseReceiptListQuery,
  PurchaseReceiptSummary
} from '@/api/purchaseReceipts';
import { getPurchaseReceiptDetail, listPurchaseReceipts } from '@/api/purchaseReceipts';
import { listSuppliers } from '@/api/suppliers';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const statusOptions = [{ label: 'CONFIRMED', value: 'CONFIRMED' }];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const PurchaseReceiptsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canRead = useCan('purchase_orders.read');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [filterForm] = Form.useForm<FilterValues>();
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeDetail, setActiveDetail] = useState<PurchaseReceiptDetail | null>(null);

  const query = useMemo<PurchaseReceiptListQuery>(
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
    queryKey: ['purchase-receipts', tenantId, query],
    queryFn: () => listPurchaseReceipts(tenantId, query),
    enabled: tenantReady && canRead,
    placeholderData: (previousData) => previousData
  });

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'purchase-receipts'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && canRead
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'purchase-receipts'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && canRead
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

  const supplierMap = useMemo(() => {
    const entries = supplierQuery.data?.items ?? [];
    return new Map(entries.map((supplier) => [supplier.supplier_id, supplier.name]));
  }, [supplierQuery.data?.items]);

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const columns = useMemo<ColumnsType<PurchaseReceiptSummary>>(
    () => [
      { title: 'Receipt ID', dataIndex: 'receipt_id' },
      { title: 'PO ID', dataIndex: 'po_id' },
      {
        title: 'Supplier',
        dataIndex: 'supplier_id',
        render: (_, record) =>
          record.supplier_name ?? supplierMap.get(record.supplier_id) ?? record.supplier_id
      },
      {
        title: 'Warehouse',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: 'Status',
        dataIndex: 'status',
        render: (value: string) => <Tag color={value === 'CONFIRMED' ? 'green' : 'default'}>{value}</Tag>
      },
      { title: 'Confirmed At', dataIndex: 'confirmed_at', render: formatDateTime },
      { title: 'Created At', dataIndex: 'created_at', render: formatDateTime },
      {
        title: 'Actions',
        key: 'actions',
        render: (_, record) => (
          <Button size="small" onClick={() => handleViewDetail(record.receipt_id)}>
            Detail
          </Button>
        )
      }
    ],
    [supplierMap, warehouseMap]
  );

  const itemColumns = useMemo<ColumnsType<PurchaseReceiptDetail['items'][number]>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: 'Qty', dataIndex: 'qty' }
    ],
    []
  );

  const handleViewDetail = async (receiptId: string) => {
    try {
      const detail = await getPurchaseReceiptDetail(tenantId, receiptId);
      setActiveDetail(detail);
      setDetailOpen(true);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'Failed to load purchase receipt');
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
          Purchase Receipts
        </Typography.Title>
        <Button icon={<ReloadOutlined />} onClick={handleRefresh}>
          Refresh
        </Button>
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
      <UiModule03MainTableSection<PurchaseReceiptSummary>
        header={header}
        rowKey="receipt_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="Failed to load purchase receipts"
        emptyText="No receipts"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="purchase-receipts.table.columns"
      />
      <Drawer
        title="Purchase Receipt Detail"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={520}
      >
        {activeDetail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Descriptions bordered size="small" column={1}>
              <Descriptions.Item label="Receipt ID">
                {activeDetail.receipt.receipt_id}
              </Descriptions.Item>
              <Descriptions.Item label="PO ID">{activeDetail.receipt.po_id}</Descriptions.Item>
              <Descriptions.Item label="Supplier">
                {activeDetail.receipt.supplier_name ??
                  (activeDetail.receipt.supplier_id
                    ? supplierMap.get(activeDetail.receipt.supplier_id) ??
                      activeDetail.receipt.supplier_id
                    : '-')}
              </Descriptions.Item>
              <Descriptions.Item label="Warehouse">
                {warehouseMap.get(activeDetail.receipt.warehouse_id) ??
                  activeDetail.receipt.warehouse_id}
              </Descriptions.Item>
              <Descriptions.Item label="Status">
                {activeDetail.receipt.status}
              </Descriptions.Item>
              <Descriptions.Item label="Confirmed At">
                {formatDateTime(activeDetail.receipt.confirmed_at)}
              </Descriptions.Item>
              <Descriptions.Item label="Created At">
                {formatDateTime(activeDetail.receipt.created_at)}
              </Descriptions.Item>
            </Descriptions>
            <Table
              rowKey="sku_id"
              columns={itemColumns}
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
