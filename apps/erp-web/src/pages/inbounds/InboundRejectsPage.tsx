import {
  Button,
  Card,
  DatePicker,
  Form,
  Input,
  Select,
  Space,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type { InboundReject } from '@/api/inboundRejects';
import { listInboundRejects } from '@/api/inboundRejects';
import { searchSkus, type SkuSearchItem } from '@/api/skus';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  doc_no_keyword?: string;
  sku_id?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatSkuLabel = (sku: SkuSearchItem) => {
  const name = sku.name ? ` ${sku.name}` : '';
  const barcode = sku.barcode ? ` (${sku.barcode})` : '';
  return `${sku.sku_code}${name}${barcode}`;
};

export const InboundRejectsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuLoading, setSkuLoading] = useState(false);

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'inbound-rejects'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

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
    return {
      doc_no_keyword: filters.doc_no_keyword?.trim() || undefined,
      sku_id: filters.sku_id,
      created_from: range?.[0]?.startOf('day').toISOString(),
      created_to: range?.[1]?.endOf('day').toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const listQuery = useQuery({
    queryKey: ['inbound-rejects', tenantId, query],
    queryFn: () => listInboundRejects(tenantId, query),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const columns = useMemo<ColumnsType<InboundReject>>(
    () => [
      { title: '入库单号', dataIndex: 'doc_no' },
      { title: '入库类型', dataIndex: 'doc_type' },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: 'SKU',
        dataIndex: 'sku_id',
        render: (_, record) => record.sku_code ?? record.sku_id
      },
      { title: '数量', dataIndex: 'qty' },
      { title: '拒收原因', dataIndex: 'reason', render: (value) => value ?? '-' },
      { title: '操作人', dataIndex: 'operator_id', render: (value) => value ?? '-' },
      { title: '时间', dataIndex: 'created_at', render: (value) => formatDateTime(value) }
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

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      拒收查询
    </Typography.Title>
  );

  const header = (
    <Space direction="vertical" size={12}>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Space wrap>
            <Form.Item name="doc_no_keyword" label="单据号">
              <Input allowClear placeholder="输入入库单号/关键词" />
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
            <Form.Item name="created_range" label="拒收时间">
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
    <UiModule03MainTableSection<InboundReject>
      toolbarLeft={toolbarLeft}
      header={header}
      rowKey={(record) => record.reject_id}
      columns={columns}
      data={listQuery.data?.items ?? []}
      loading={listQuery.isFetching}
      error={listQuery.error}
      onRetry={() => listQuery.refetch()}
      errorTitle="拒收记录加载失败"
      emptyText="暂无拒收记录"
      total={listQuery.data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      columnSettingsKey="inbound-rejects.table.columns"
    />
  );
};
