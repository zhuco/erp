import {
  Button,
  Card,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Table,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  applyStockCount,
  confirmStockCountDiff,
  createStockCount,
  getStockCount,
  listStockCounts,
  updateStockCountItems
} from '@/api/stockCounts';
import type {
  StockCountCreatePayload,
  StockCountItem,
  StockCountListQuery,
  StockCountSummary
} from '@/api/stockCounts';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { ErrorState } from '@/components/QueryState';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

type StockCountCreateFormValues = {
  warehouse_id: string;
  strategy: string;
  dimension: string;
  recent_days?: number;
  brand?: string;
  category?: string;
};

type StockCountEntryValues = {
  sku_id: string;
  location_id?: string;
  counted_qty: number;
};

export const StockCountsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeCountId, setActiveCountId] = useState<string | null>(null);
  const [countedMap, setCountedMap] = useState<Record<string, number>>({});
  const [createForm] = Form.useForm<StockCountCreateFormValues>();
  const [entryForm] = Form.useForm<StockCountEntryValues>();

  const syncCountedMap = (items: StockCountItem[]) => {
    const nextMap: Record<string, number> = {};
    items.forEach((item) => {
      nextMap[item.count_item_id] = item.counted_qty;
    });
    setCountedMap(nextMap);
  };

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'stock-counts'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady,
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

  const listQuery = useMemo<StockCountListQuery>(
    () => ({
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [pagination.current, pagination.pageSize]
  );

  const listQueryState = useQuery({
    queryKey: ['stock-counts', tenantId, listQuery],
    queryFn: () => listStockCounts(tenantId, listQuery),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const detailQuery = useQuery({
    queryKey: ['stock-counts', tenantId, activeCountId],
    queryFn: () => getStockCount(tenantId, activeCountId ?? ''),
    enabled: tenantReady && Boolean(activeCountId),
    onSuccess: (detail) => {
      syncCountedMap(detail.items);
    }
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      const values = await createForm.validateFields();
      const payload: StockCountCreatePayload = {
        warehouse_id: values.warehouse_id,
        strategy: values.strategy,
        dimension: values.dimension
      };
      if (values.strategy === 'RECENT_DAYS') {
        payload.recent_days = values.recent_days;
      }
      if (values.strategy === 'BRAND') {
        payload.brand = values.brand?.trim() || undefined;
      }
      if (values.strategy === 'CATEGORY') {
        payload.category = values.category?.trim() || undefined;
      }
      return createStockCount(tenantId, payload);
    },
    onSuccess: async () => {
      message.success('盘点单已创建');
      setCreateOpen(false);
      createForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['stock-counts', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '创建盘点单失败');
    }
  });

  const updateMutation = useMutation({
    mutationFn: async (items: StockCountItem[]) =>
      updateStockCountItems(
        tenantId,
        activeCountId ?? '',
        items.map((item) => ({
          count_item_id: item.count_item_id,
          counted_qty: countedMap[item.count_item_id] ?? item.counted_qty
        }))
      ),
    onSuccess: (detail) => {
      message.success('盘点数量已保存');
      queryClient.setQueryData(['stock-counts', tenantId, activeCountId], detail);
      syncCountedMap(detail.items);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存盘点数量失败');
    }
  });

  const entryMutation = useMutation({
    mutationFn: async (values: StockCountEntryValues) => {
      if (!activeCountId) {
        throw new Error('请选择盘点单');
      }
      const detail = detailQuery.data;
      if (!detail) {
        throw new Error('盘点详情未加载');
      }
      const skuId = values.sku_id.trim();
      if (!skuId) {
        throw new Error('请输入SKU');
      }
      if (values.counted_qty == null) {
        throw new Error('请输入盘点数量');
      }
      const locationId = values.location_id?.trim() || undefined;
      if (detail.count.dimension === 'LOCATION' && !locationId) {
        throw new Error('请输入货位');
      }
      const match = detail.items.find((item) => {
        if (item.sku_id !== skuId) {
          return false;
        }
        if (detail.count.dimension === 'LOCATION') {
          return item.location_id === locationId;
        }
        return true;
      });
      if (!match) {
        throw new Error('未找到对应盘点行');
      }
      return updateStockCountItems(tenantId, activeCountId, [
        {
          count_item_id: match.count_item_id,
          counted_qty: values.counted_qty
        }
      ]);
    },
    onSuccess: (detail) => {
      message.success('盘点录入已保存');
      queryClient.setQueryData(['stock-counts', tenantId, activeCountId], detail);
      syncCountedMap(detail.items);
      entryForm.resetFields();
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '盘点录入失败');
    }
  });

  const confirmMutation = useMutation({
    mutationFn: () => confirmStockCountDiff(tenantId, activeCountId ?? ''),
    onSuccess: (detail) => {
      message.success('差异已确认');
      queryClient.setQueryData(['stock-counts', tenantId, activeCountId], detail);
      queryClient.invalidateQueries({ queryKey: ['stock-counts', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '确认差异失败');
    }
  });

  const applyMutation = useMutation({
    mutationFn: () => applyStockCount(tenantId, activeCountId ?? ''),
    onSuccess: (detail) => {
      message.success('盘点已应用');
      queryClient.setQueryData(['stock-counts', tenantId, activeCountId], detail);
      queryClient.invalidateQueries({ queryKey: ['stock-counts', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '应用盘点失败');
    }
  });

  const formatStrategy = useCallback((record: StockCountSummary) => {
    const strategy = record.strategy;
    if (strategy === 'RECENT_DAYS') {
      const days = record.filters?.recent_days;
      return days ? `近${days}天出入库` : '近N天出入库';
    }
    if (strategy === 'BRAND') {
      return record.filters?.brand ? `品牌：${record.filters.brand}` : '按品牌';
    }
    if (strategy === 'CATEGORY') {
      return record.filters?.category ? `品类：${record.filters.category}` : '按品类';
    }
    if (strategy === 'SUGGESTION') {
      return '建议盘点';
    }
    return '全仓盘点';
  }, []);

  const formatDimension = useCallback(
    (value: string) => (value === 'LOCATION' ? '按货位' : '按SKU'),
    []
  );

  const columns = useMemo<ColumnsType<StockCountSummary>>(
    () => [
      {
        title: '盘点单号',
        dataIndex: 'count_id'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value) => warehouseMap.get(value) ?? value
      },
      {
        title: '状态',
        dataIndex: 'status'
      },
      {
        title: '策略',
        dataIndex: 'strategy',
        render: (_, record) => formatStrategy(record)
      },
      {
        title: '维度',
        dataIndex: 'dimension',
        render: (value) => formatDimension(value)
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              onClick={() => {
                setActiveCountId(record.count_id);
                setDetailOpen(true);
              }}
            >
              详情
            </Button>
          </Space>
        )
      }
    ],
    [formatDimension, formatStrategy, warehouseMap]
  );

  const detail = detailQuery.data;
  const detailItems = detail?.items ?? [];
  const showLocation = detail?.count.dimension === 'LOCATION';
  const auditRequired = detail?.audit_required ?? true;
  const canConfirm = detail?.count.status === 'COUNTING';
  const canApply =
    detail?.count.status === 'DIFF_CONFIRMED' ||
    (!auditRequired &&
      detail?.count.status !== 'APPLIED' &&
      detail?.count.status !== 'CANCELLED');
  const detailColumns = useMemo<ColumnsType<StockCountItem>>(
    () => {
      const cols: ColumnsType<StockCountItem> = [
        {
          title: 'SKU ID',
          dataIndex: 'sku_id'
        }
      ];
      if (showLocation) {
        cols.push({
          title: '货位',
          dataIndex: 'location_id',
          render: (value) => value ?? '-'
        });
      }
      cols.push(
        {
          title: '系统数量',
          dataIndex: 'system_qty'
        },
        {
          title: '盘点数量',
          dataIndex: 'counted_qty',
          render: (_, record) => (
            <InputNumber
              min={0}
              value={countedMap[record.count_item_id] ?? record.counted_qty}
              onChange={(value) => {
                setCountedMap((prev) => ({
                  ...prev,
                  [record.count_item_id]: value ?? 0
                }));
              }}
            />
          )
        },
        {
          title: '差异',
          dataIndex: 'diff_qty',
          render: (_, record) => {
            const counted = countedMap[record.count_item_id] ?? record.counted_qty;
            return counted - record.system_qty;
          }
        }
      );
      return cols;
    },
    [countedMap, showLocation]
  );

  const handleExportDiff = () => {
    if (!detail) {
      return;
    }
    const rows = detail.items
      .map((item) => {
        const counted = countedMap[item.count_item_id] ?? item.counted_qty;
        const diff = counted - item.system_qty;
        return {
          sku_id: item.sku_id,
          location_id: item.location_id ?? '',
          system_qty: item.system_qty,
          counted_qty: counted,
          diff_qty: diff
        };
      })
      .filter((row) => row.diff_qty !== 0);
    if (rows.length === 0) {
      message.info('暂无差异可导出');
      return;
    }
    const header = ['sku_id', 'location_id', 'system_qty', 'counted_qty', 'diff_qty'];
    const csv = [
      header.join(','),
      ...rows.map((row) =>
        [row.sku_id, row.location_id, row.system_qty, row.counted_qty, row.diff_qty].join(',')
      )
    ].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `stock-count-${detail.count.count_id}-diff.csv`;
    link.click();
    URL.revokeObjectURL(url);
  };

  useEffect(() => {
    if (!detailOpen) {
      setActiveCountId(null);
      entryForm.resetFields();
    }
  }, [detailOpen, entryForm]);

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        盘点管理
      </Typography.Title>
      <Button type="primary" onClick={() => setCreateOpen(true)}>
        新建盘点
      </Button>
    </Space>
  );

  const header = warehouseQuery.error ? (
    <ErrorState
      title="仓库加载失败"
      error={warehouseQuery.error}
      onRetry={() => warehouseQuery.refetch()}
    />
  ) : null;

  return (
    <>
      <UiModule03MainTableSection<StockCountSummary>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="count_id"
        columns={columns}
        data={listQueryState.data?.items ?? []}
        loading={listQueryState.isLoading || listQueryState.isFetching}
        error={listQueryState.error}
        onRetry={() => listQueryState.refetch()}
        errorTitle="盘点列表加载失败"
        emptyText="暂无盘点单"
        total={listQueryState.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
      />
      <Modal
        title="新建盘点单"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={() => createMutation.mutate()}
        confirmLoading={createMutation.isPending}
        destroyOnHidden
      >
        <Form
          form={createForm}
          layout="vertical"
          initialValues={{ strategy: 'FULL', dimension: 'SKU', recent_days: 7 }}
        >
          <Form.Item
            name="warehouse_id"
            label="仓库"
            rules={[{ required: true, message: '请选择仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
          <Form.Item
            name="strategy"
            label="盘点策略"
            rules={[{ required: true, message: '请选择盘点策略' }]}
          >
            <Select
              options={[
                { label: '全仓盘点', value: 'FULL' },
                { label: '近N天出入库', value: 'RECENT_DAYS' },
                { label: '按品牌盘点', value: 'BRAND' },
                { label: '按品类盘点', value: 'CATEGORY' }
              ]}
            />
          </Form.Item>
          <Form.Item
            name="dimension"
            label="盘点维度"
            rules={[{ required: true, message: '请选择盘点维度' }]}
          >
            <Select
              options={[
                { label: '按SKU', value: 'SKU' },
                { label: '按货位', value: 'LOCATION' }
              ]}
            />
          </Form.Item>
          <Form.Item shouldUpdate={(prev, next) => prev.strategy !== next.strategy} noStyle>
            {({ getFieldValue }) => {
              const strategy = getFieldValue('strategy');
              if (strategy === 'RECENT_DAYS') {
                return (
                  <Form.Item
                    name="recent_days"
                    label="最近天数"
                    rules={[{ required: true, message: '请输入最近天数' }]}
                  >
                    <InputNumber min={1} style={{ width: '100%' }} />
                  </Form.Item>
                );
              }
              if (strategy === 'BRAND') {
                return (
                  <Form.Item
                    name="brand"
                    label="品牌"
                    rules={[{ required: true, message: '请输入品牌' }]}
                  >
                    <Input placeholder="品牌名称" />
                  </Form.Item>
                );
              }
              if (strategy === 'CATEGORY') {
                return (
                  <Form.Item
                    name="category"
                    label="品类"
                    rules={[{ required: true, message: '请输入品类' }]}
                  >
                    <Input placeholder="品类名称" />
                  </Form.Item>
                );
              }
              return null;
            }}
          </Form.Item>
        </Form>
      </Modal>
      <Drawer
        title="盘点详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={720}
        destroyOnHidden
      >
        {detailQuery.isLoading ? (
          <Card loading />
        ) : detail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Typography.Text type="secondary">
              策略：{formatStrategy(detail.count)}｜维度：
              {formatDimension(detail.count.dimension)}
            </Typography.Text>
            <Form
              form={entryForm}
              layout="inline"
              onFinish={(values) => entryMutation.mutate(values)}
              initialValues={{ counted_qty: 0 }}
            >
              <Form.Item
                name="sku_id"
                label="SKU"
                rules={[{ required: true, message: '请输入SKU' }]}
              >
                <Input placeholder="SKU ID/编码" />
              </Form.Item>
              {showLocation ? (
                <Form.Item
                  name="location_id"
                  label="货位"
                  rules={[{ required: true, message: '请输入货位' }]}
                >
                  <Input placeholder="货位 ID/编码" />
                </Form.Item>
              ) : null}
              <Form.Item
                name="counted_qty"
                label="盘点数量"
                rules={[{ required: true, message: '请输入盘点数量' }]}
              >
                <InputNumber min={0} />
              </Form.Item>
              <Form.Item>
                <Button type="primary" htmlType="submit" loading={entryMutation.isPending}>
                  录入
                </Button>
              </Form.Item>
            </Form>
            <Space>
              <Button
                onClick={() => updateMutation.mutate(detail.items)}
                loading={updateMutation.isPending}
              >
                保存盘点数量
              </Button>
              <Button onClick={handleExportDiff}>导出差异</Button>
              <Button
                onClick={() => confirmMutation.mutate()}
                loading={confirmMutation.isPending}
                disabled={!canConfirm}
              >
                确认差异
              </Button>
              <Button
                type="primary"
                onClick={() => applyMutation.mutate()}
                loading={applyMutation.isPending}
                disabled={!canApply}
              >
                应用盘点
              </Button>
            </Space>
            <Table
              rowKey="count_item_id"
              columns={detailColumns}
              dataSource={detailItems}
              pagination={false}
              size="small"
            />
          </Space>
        ) : null}
      </Drawer>
    </>
  );
};
