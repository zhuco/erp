import {
  Button,
  Card,
  Col,
  DatePicker,
  Drawer,
  Form,
  Input,
  Row,
  Select,
  Space,
  Switch,
  Tabs,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import {
  createSkuBatch,
  listSkuBatches,
  updateSkuBatch,
  type SkuBatch,
  type SkuBatchCreateRequest,
  type SkuBatchUpdateRequest
} from '@/api/skuBatches';
import {
  createSkuBatchStrategy,
  deleteSkuBatchStrategy,
  listSkuBatchStrategies,
  updateSkuBatchStrategy,
  type SkuBatchStrategy,
  type SkuBatchStrategyPayload
} from '@/api/skuBatchStrategies';
import { searchSkus, type SkuSearchItem } from '@/api/skus';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  sku_id?: string;
  batch_no?: string;
};

type FormValues = {
  sku_id?: string;
  batch_no: string;
  expire_at?: dayjs.Dayjs;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const buildSkuLabel = (sku: SkuSearchItem) => {
  const name = sku.name ? ` ${sku.name}` : '';
  const barcode = sku.barcode ? ` (${sku.barcode})` : '';
  return `${sku.sku_code}${name}${barcode}`;
};

type StrategyFilterValues = {
  sku_id?: string;
  enabled?: boolean;
};

type StrategyFormValues = {
  sku_id?: string;
  name: string;
  enabled?: boolean;
  expire_threshold_days?: number;
  fifo_enabled?: boolean;
};

const SkuBatchStrategiesPanel = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<StrategyFilterValues>();
  const [drawerForm] = Form.useForm<StrategyFormValues>();
  const [filters, setFilters] = useState<StrategyFilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingStrategy, setEditingStrategy] = useState<SkuBatchStrategy | null>(null);
  const [saving, setSaving] = useState(false);
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuLoading, setSkuLoading] = useState(false);

  const query = useMemo(
    () => ({
      sku_id: filters.sku_id,
      enabled: filters.enabled,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters.enabled, filters.sku_id, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sku-batch-strategies', tenantId, query],
    queryFn: () => listSkuBatchStrategies(tenantId, query),
    enabled: Boolean(tenantId)
  });

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

  const columns = useMemo<ColumnsType<SkuBatchStrategy>>(
    () => [
      {
        title: '策略名称',
        dataIndex: 'name'
      },
      {
        title: 'SKU',
        dataIndex: 'sku_id',
        render: (value) => value ?? '通用'
      },
      {
        title: '效期阈值(日)',
        dataIndex: 'expire_threshold_days'
      },
      {
        title: '先进先出',
        dataIndex: 'fifo_enabled',
        render: (value) => (value ? '是' : '否')
      },
      {
        title: '启用',
        dataIndex: 'enabled',
        render: (value) => (value ? '启用' : '停用')
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setEditingStrategy(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  sku_id: record.sku_id ?? undefined,
                  name: record.name,
                  enabled: record.enabled,
                  expire_threshold_days: record.expire_threshold_days,
                  fifo_enabled: record.fifo_enabled
                });
              }}
            >
              编辑
            </Button>
            <Button
              size="small"
              danger
              disabled={!canWrite}
              onClick={async () => {
                try {
                  await deleteSkuBatchStrategy(tenantId, record.strategy_id);
                  message.success('策略已删除');
                  await queryClient.invalidateQueries({
                    queryKey: ['sku-batch-strategies', tenantId]
                  });
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '删除失败');
                }
              }}
            >
              删除
            </Button>
          </Space>
        )
      }
    ],
    [canWrite, drawerForm, queryClient, tenantId]
  );

  const handleSearch = (values: StrategyFilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingStrategy(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ enabled: true, fifo_enabled: false, expire_threshold_days: 0 });
    void loadSkuOptions();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const expireThreshold = Number(values.expire_threshold_days ?? 0);
      const payload: SkuBatchStrategyPayload = {
        sku_id: values.sku_id ?? null,
        name: values.name.trim(),
        enabled: values.enabled ?? true,
        expire_threshold_days: Number.isNaN(expireThreshold) ? 0 : expireThreshold,
        fifo_enabled: values.fifo_enabled ?? false
      };
      setSaving(true);
      if (editingStrategy) {
        await updateSkuBatchStrategy(tenantId, editingStrategy.strategy_id, payload);
        message.success('策略已更新');
      } else {
        await createSkuBatchStrategy(tenantId, payload);
        message.success('策略已创建');
      }
      setDrawerOpen(false);
      setEditingStrategy(null);
      await queryClient.invalidateQueries({ queryKey: ['sku-batch-strategies', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={4} style={{ margin: 0 }}>
          批次策略
        </Typography.Title>
        <Button type="primary" onClick={openCreateDrawer} disabled={!canWrite}>
          新建策略
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="sku_id" label="SKU">
                <Select
                  showSearch
                  allowClear
                  placeholder="Search SKU"
                  options={skuOptions.map((sku) => ({
                    value: sku.sku_id,
                    label: buildSkuLabel(sku)
                  }))}
                  filterOption={false}
                  loading={skuLoading}
                  onSearch={(value) => loadSkuOptions(value)}
                  onFocus={() => loadSkuOptions()}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="enabled" label="状态">
                <Select
                  allowClear
                  options={[
                    { value: true, label: '启用' },
                    { value: false, label: '停用' }
                  ]}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
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
      <UiModule03MainTableSection<SkuBatchStrategy>
        rowKey="strategy_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="批次策略加载失败"
        emptyText="暂无策略"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="sku-batch-strategies.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editingStrategy ? '编辑策略' : '新建策略'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>Cancel</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              Save
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical" initialValues={{ enabled: true }}>
          <Form.Item name="name" label="策略名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input placeholder="策略名称" />
          </Form.Item>
          <Form.Item name="sku_id" label="SKU">
            <Select
              showSearch
              allowClear
              placeholder="Search SKU"
              options={skuOptions.map((sku) => ({
                value: sku.sku_id,
                label: buildSkuLabel(sku)
              }))}
              filterOption={false}
              loading={skuLoading}
              onSearch={(value) => loadSkuOptions(value)}
              onFocus={() => loadSkuOptions()}
            />
          </Form.Item>
          <Form.Item name="expire_threshold_days" label="效期阈值(日)">
            <Input type="number" min={0} />
          </Form.Item>
          <Form.Item name="fifo_enabled" label="先进先出" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Drawer>
    </Space>
  );
};

export const SkuBatchesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [cursorMap, setCursorMap] = useState<Record<number, string | undefined>>({});
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingBatch, setEditingBatch] = useState<SkuBatch | null>(null);
  const [saving, setSaving] = useState(false);
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuLoading, setSkuLoading] = useState(false);

  const currentPage = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  const cursor = cursorMap[currentPage];

  const query = useMemo(
    () => ({
      sku_id: filters.sku_id,
      batch_no: filters.batch_no?.trim() || undefined,
      cursor,
      limit: pageSize
    }),
    [cursor, filters.batch_no, filters.sku_id, pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sku-batches', tenantId, query],
    queryFn: () => listSkuBatches(tenantId, query),
    keepPreviousData: true,
    enabled: Boolean(tenantId)
  });

  useEffect(() => {
    if (!data?.next_cursor) {
      return;
    }
    const nextPage = currentPage + 1;
    setCursorMap((prev) => {
      if (prev[nextPage] === data.next_cursor) {
        return prev;
      }
      return { ...prev, [nextPage]: data.next_cursor ?? undefined };
    });
  }, [currentPage, data?.next_cursor]);

  const total = useMemo(() => {
    if (!data) {
      return 0;
    }
    if (data.next_cursor) {
      return currentPage * pageSize + pageSize;
    }
    return (currentPage - 1) * pageSize + data.data.length;
  }, [currentPage, data, pageSize]);

  const skuMap = useMemo(() => {
    const map = new Map<string, SkuSearchItem>();
    skuOptions.forEach((item) => map.set(item.sku_id, item));
    if (editingBatch && !map.has(editingBatch.sku_id)) {
      map.set(editingBatch.sku_id, {
        sku_id: editingBatch.sku_id,
        sku_code: editingBatch.sku_id,
        name: null,
        barcode: null,
        sku_kind: null
      });
    }
    return map;
  }, [editingBatch, skuOptions]);

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

  const columns = useMemo<ColumnsType<SkuBatch>>(
    () => [
      {
        title: 'SKU',
        dataIndex: 'sku_id',
        render: (_, record) => {
          const sku = skuMap.get(record.sku_id);
          return sku ? buildSkuLabel(sku) : record.sku_id;
        }
      },
      {
        title: 'Batch No',
        dataIndex: 'batch_no'
      },
      {
        title: 'Expire At',
        dataIndex: 'expire_at',
        render: (_, record) => formatDateTime(record.expire_at)
      },
      {
        title: 'Updated At',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      },
      {
        title: 'Actions',
        key: 'actions',
        render: (_, record) => (
          <Button
            size="small"
            disabled={!canWrite}
            onClick={() => {
              setEditingBatch(record);
              setDrawerOpen(true);
              drawerForm.setFieldsValue({
                sku_id: record.sku_id,
                batch_no: record.batch_no,
                expire_at: record.expire_at ? dayjs(record.expire_at) : undefined
              });
            }}
          >
            Edit
          </Button>
        )
      }
    ],
    [canWrite, drawerForm, skuMap]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setCursorMap({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setCursorMap({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingBatch(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    void loadSkuOptions();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const expireAt = values.expire_at ? values.expire_at.toISOString() : undefined;
      setSaving(true);
      if (editingBatch) {
        const payload: SkuBatchUpdateRequest = {
          batch_no: values.batch_no?.trim() || undefined,
          expire_at: expireAt
        };
        await updateSkuBatch(tenantId, editingBatch.batch_id, payload);
        message.success('批次已更新');
      } else {
        if (!values.sku_id) {
          message.error('请选择 SKU');
          setSaving(false);
          return;
        }
        const payload: SkuBatchCreateRequest = {
          sku_id: values.sku_id,
          batch_no: values.batch_no.trim(),
          expire_at: expireAt
        };
        await createSkuBatch(tenantId, payload);
        message.success('批次已创建');
      }
      setDrawerOpen(false);
      setEditingBatch(null);
      await queryClient.invalidateQueries({ queryKey: ['sku-batches', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const batchListContent = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          SKU 批次
        </Typography.Title>
        <Button type="primary" onClick={openCreateDrawer} disabled={!canWrite}>
          New Batch
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="sku_id" label="SKU">
                <Select
                  showSearch
                  allowClear
                  placeholder="Search SKU"
                  options={skuOptions.map((sku) => ({
                    value: sku.sku_id,
                    label: buildSkuLabel(sku)
                  }))}
                  filterOption={false}
                  loading={skuLoading}
                  onSearch={(value) => loadSkuOptions(value)}
                  onFocus={() => loadSkuOptions()}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="batch_no" label="Batch No">
                <Input allowClear placeholder="Batch number" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
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
      <UiModule03MainTableSection<SkuBatch>
        rowKey="batch_id"
        columns={columns}
        data={data?.data ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="批次加载失败"
        emptyText="暂无批次"
        total={total}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="sku-batches.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editingBatch ? 'Edit Batch' : 'New Batch'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>Cancel</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              Save
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="sku_id"
            label="SKU"
            rules={[{ required: true, message: '请选择 SKU' }]}
          >
            <Select
              showSearch
              allowClear
              disabled={Boolean(editingBatch)}
              placeholder="Search SKU"
              options={skuOptions.map((sku) => ({
                value: sku.sku_id,
                label: buildSkuLabel(sku)
              }))}
              filterOption={false}
              loading={skuLoading}
              onSearch={(value) => loadSkuOptions(value)}
              onFocus={() => loadSkuOptions()}
            />
          </Form.Item>
          <Form.Item
            name="batch_no"
            label="Batch No"
            rules={[{ required: true, message: '输入批次号' }]}
          >
            <Input placeholder="Batch number" />
          </Form.Item>
          <Form.Item name="expire_at" label="Expire At">
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Drawer>
    </Space>
  );

  return (
    <Tabs
      defaultActiveKey="batches"
      items={[
        { key: 'batches', label: '批次列表', children: batchListContent },
        { key: 'strategies', label: '批次策略', children: <SkuBatchStrategiesPanel /> }
      ]}
    />
  );
};
