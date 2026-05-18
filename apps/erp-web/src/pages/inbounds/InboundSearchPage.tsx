import {
  Button,
  Card,
  DatePicker,
  Form,
  Input,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type { InboundDocStatus, InboundDocSummary, InboundDocType } from '@/api/inbounds';
import { listInbounds } from '@/api/inbounds';
import { searchSkus, type SkuSearchItem } from '@/api/skus';
import { listSuppliers } from '@/api/suppliers';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  doc_no?: string;
  supplier_id?: string;
  sku_id?: string;
  status?: InboundDocStatus;
  doc_type?: InboundDocType;
  warehouse_id?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const docTypeOptions: Array<{ label: string; value: InboundDocType }> = [
  { label: '采购入库', value: 'PURCHASE' },
  { label: '退货入库', value: 'RETURN' },
  { label: '其他入库', value: 'OTHER' }
];

const statusOptions: Array<{ label: string; value: InboundDocStatus }> = [
  { label: '草稿', value: 'DRAFT' },
  { label: '已审核', value: 'AUDITED' },
  { label: '收货中', value: 'RECEIVING' },
  { label: '已上架', value: 'PUTAWAY_DONE' },
  { label: '已取消', value: 'CANCELLED' }
];

const statusColors: Record<InboundDocStatus, string> = {
  DRAFT: 'default',
  AUDITED: 'gold',
  RECEIVING: 'blue',
  PUTAWAY_DONE: 'green',
  CANCELLED: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatSkuLabel = (sku: SkuSearchItem) => {
  const name = sku.name ? ` ${sku.name}` : '';
  const barcode = sku.barcode ? ` (${sku.barcode})` : '';
  return `${sku.sku_code}${name}${barcode}`;
};

export const InboundSearchPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const tenantReady = isTenantSelected(tenantId);
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuLoading, setSkuLoading] = useState(false);

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'inbound-search'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'inbound-search'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
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

  const supplierOptions = useMemo(
    () =>
      supplierQuery.data?.items.map((supplier) => ({
        label: supplier.name,
        value: supplier.supplier_id
      })) ?? [],
    [supplierQuery.data?.items]
  );

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const supplierMap = useMemo(() => {
    const entries = supplierQuery.data?.items ?? [];
    return new Map(entries.map((supplier) => [supplier.supplier_id, supplier.name]));
  }, [supplierQuery.data?.items]);

  const loadSkuOptions = async (keyword?: string) => {
    setSkuLoading(true);
    try {
      const response = await searchSkus(tenantId, keyword ?? '');
      setSkuOptions(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'SKU 搜索失败');
    } finally {
      setSkuLoading(false);
    }
  };

  const skuSelectOptions = useMemo(
    () =>
      skuOptions.map((sku) => ({
        label: formatSkuLabel(sku),
        value: sku.sku_id
      })),
    [skuOptions]
  );

  const query = useMemo(() => {
    const range = filters.created_range;
    const created_from = range?.[0]?.startOf('day').toISOString();
    const created_to = range?.[1]?.endOf('day').toISOString();
    return {
      doc_no_keyword: filters.doc_no?.trim() || undefined,
      supplier_id: filters.supplier_id,
      sku_id: filters.sku_id,
      status: filters.status,
      doc_type: filters.doc_type,
      warehouse_id: filters.warehouse_id,
      created_from,
      created_to,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const listQuery = useQuery({
    queryKey: ['inbound-search', tenantId, query],
    queryFn: () => listInbounds(tenantId, query),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const columns = useMemo<ColumnsType<InboundDocSummary>>(
    () => [
      { title: '入库单号', dataIndex: 'doc_no' },
      {
        title: '类型',
        dataIndex: 'doc_type',
        render: (value: InboundDocType) => docTypeOptions.find((item) => item.value === value)?.label
      },
      {
        title: '供应商',
        dataIndex: 'supplier_name',
        render: (_, record) =>
          record.supplier_name ||
          (record.supplier_id ? supplierMap.get(record.supplier_id) ?? record.supplier_id : '-')
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: InboundDocStatus) => (
          <Tag color={statusColors[value] || 'default'}>{value}</Tag>
        )
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button size="small" onClick={() => navigate(`/wms/inbounds/${record.doc_id}`)}>
            详情
          </Button>
        )
      }
    ],
    [navigate, supplierMap, warehouseMap]
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

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      收货查询
    </Typography.Title>
  );

  const header = (
    <Space direction="vertical" size={12}>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Space wrap>
            <Form.Item name="doc_no" label="单据号/关键词">
              <Input allowClear placeholder="输入单据号或 PO 关键字" />
            </Form.Item>
            <Form.Item name="supplier_id" label="供应商">
              <Select allowClear options={supplierOptions} placeholder="选择供应商" />
            </Form.Item>
            <Form.Item name="warehouse_id" label="仓库">
              <Select allowClear options={warehouseOptions} placeholder="选择仓库" />
            </Form.Item>
            <Form.Item name="doc_type" label="类型">
              <Select allowClear options={docTypeOptions} placeholder="选择类型" />
            </Form.Item>
            <Form.Item name="status" label="状态">
              <Select allowClear options={statusOptions} placeholder="选择状态" />
            </Form.Item>
            <Form.Item name="sku_id" label="SKU">
              <Select
                showSearch
                allowClear
                placeholder="搜索 SKU"
                options={skuSelectOptions}
                filterOption={false}
                onSearch={(value) => void loadSkuOptions(value)}
                onFocus={() => void loadSkuOptions()}
                loading={skuLoading}
                style={{ width: 220 }}
              />
            </Form.Item>
            <Form.Item name="created_range" label="创建时间">
              <DatePicker.RangePicker allowClear />
            </Form.Item>
            <Form.Item label=" ">
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Form.Item>
          </Space>
        </Form>
      </Card>
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
    </Space>
  );

  return (
    <>
      {inventoryFeature.enabled ? (
        <UiModule03MainTableSection<InboundDocSummary>
          toolbarLeft={toolbarLeft}
          header={header}
          rowKey={(record) => record.doc_id}
          columns={columns}
          data={listQuery.data?.items ?? []}
          loading={listQuery.isFetching}
          error={listQuery.error}
          onRetry={() => listQuery.refetch()}
          errorTitle="收货查询失败"
          emptyText="暂无收货记录"
          total={listQuery.data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="inbounds.search.table.columns"
        />
      ) : null}
    </>
  );
};
