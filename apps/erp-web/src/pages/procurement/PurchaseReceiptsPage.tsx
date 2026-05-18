import {
  Button,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
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
import { ErpDataGrid } from '@/components/ErpDataGrid';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  supplier_id?: string;
  warehouse_id?: string;
  status?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const statusOptions = [{ label: '已确认', value: 'CONFIRMED' }];

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
      { title: '收货单号', dataIndex: 'receipt_id' },
      { title: '采购单号', dataIndex: 'po_id' },
      {
        title: '供应商',
        dataIndex: 'supplier_id',
        render: (_, record) =>
          record.supplier_name ?? supplierMap.get(record.supplier_id) ?? record.supplier_id
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: string) => <Tag color={value === 'CONFIRMED' ? 'green' : 'default'}>{value}</Tag>
      },
      { title: '确认时间', dataIndex: 'confirmed_at', render: formatDateTime },
      { title: '创建时间', dataIndex: 'created_at', render: formatDateTime },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button size="small" onClick={() => handleViewDetail(record.receipt_id)}>
            查看详情
          </Button>
        )
      }
    ],
    [supplierMap, warehouseMap]
  );

  const itemColumns = useMemo<ColumnsType<PurchaseReceiptDetail['items'][number]>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: '数量', dataIndex: 'qty' }
    ],
    []
  );

  const handleViewDetail = async (receiptId: string) => {
    try {
      const detail = await getPurchaseReceiptDetail(tenantId, receiptId);
      setActiveDetail(detail);
      setDetailOpen(true);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '采购收货详情加载失败');
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
    <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
      <Row gutter={[16, 16]}>
        <Col xs={24} md={6}>
          <Form.Item name="supplier_id" label="供应商">
            <Select
              allowClear
              options={supplierOptions}
              placeholder="请选择供应商"
              loading={supplierQuery.isFetching}
            />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="warehouse_id" label="仓库">
            <Select
              allowClear
              options={warehouseOptions}
              placeholder="请选择仓库"
              loading={warehouseQuery.isFetching}
            />
          </Form.Item>
        </Col>
        <Col xs={24} md={4}>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="请选择状态" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="created_range" label="创建时间">
            <DatePicker.RangePicker showTime style={{ width: '100%' }} allowClear />
          </Form.Item>
        </Col>
        <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end', justifyContent: 'flex-end' }}>
          <Space>
            <Button type="primary" htmlType="submit" data-testid="purchase-receipts-search-button">
              查询
            </Button>
            <Button onClick={handleReset} data-testid="purchase-receipts-reset-button">
              重置
            </Button>
          </Space>
        </Col>
      </Row>
    </Form>
  );

  return (
    <>
      <UiModule03MainTableSection<PurchaseReceiptSummary>
        dataTestId="purchase-receipts-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        toolbarLeft={
          <Typography.Title level={3} style={{ margin: 0 }}>
            采购收货
          </Typography.Title>
        }
        toolbarRight={
          <Button icon={<ReloadOutlined />} onClick={handleRefresh} data-testid="purchase-receipts-refresh-button">
            刷新
          </Button>
        }
        header={header}
        rowKey="receipt_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="采购收货列表加载失败"
        emptyText="暂无采购收货记录"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="purchase-receipts.table.columns"
        showColumnSettings
      />
      <Drawer
        title="采购收货详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={520}
      >
        {activeDetail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Descriptions bordered size="small" column={1}>
              <Descriptions.Item label="收货单号">
                {activeDetail.receipt.receipt_id}
              </Descriptions.Item>
              <Descriptions.Item label="采购单号">{activeDetail.receipt.po_id}</Descriptions.Item>
              <Descriptions.Item label="供应商">
                {activeDetail.receipt.supplier_name ??
                  (activeDetail.receipt.supplier_id
                    ? supplierMap.get(activeDetail.receipt.supplier_id) ??
                      activeDetail.receipt.supplier_id
                    : '-')}
              </Descriptions.Item>
              <Descriptions.Item label="仓库">
                {warehouseMap.get(activeDetail.receipt.warehouse_id) ??
                  activeDetail.receipt.warehouse_id}
              </Descriptions.Item>
              <Descriptions.Item label="状态">
                {activeDetail.receipt.status}
              </Descriptions.Item>
              <Descriptions.Item label="确认时间">
                {formatDateTime(activeDetail.receipt.confirmed_at)}
              </Descriptions.Item>
              <Descriptions.Item label="创建时间">
                {formatDateTime(activeDetail.receipt.created_at)}
              </Descriptions.Item>
            </Descriptions>
            <ErpDataGrid<PurchaseReceiptDetail['items'][number]>
              rowKey={(record) => record.sku_id}
              columns={itemColumns}
              data={activeDetail.items}
              compact
              showRowIndex={false}
              data-testid="purchase-receipts-detail-items-grid"
            />
          </Space>
        ) : (
          <Typography.Text type="secondary">暂无详情</Typography.Text>
        )}
      </Drawer>
    </>
  );
};
