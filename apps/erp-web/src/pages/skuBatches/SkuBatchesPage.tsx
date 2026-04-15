import {
  Alert,
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
  Tag,
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
import { listProducts, type Product } from '@/api/products';
import { listSkus, searchSkus, type Sku, type SkuSearchItem } from '@/api/skus';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import {
  getFoodPolicySummary,
  getProductIndustryProfile,
  readNumberAttr,
  readProductAttrs
} from '@/pages/products/productIndustry';

type FilterValues = {
  sku_id?: string;
  batch_no?: string;
  governance_status?: 'NORMAL' | 'NEAR_EXPIRY' | 'FORBID_SALE';
};

type FormValues = {
  sku_id?: string;
  batch_no: string;
  production_date?: dayjs.Dayjs;
  expire_at?: dayjs.Dayjs;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const buildSkuLabel = (sku: SkuSearchItem) => {
  const name = sku.name ? ` ${sku.name}` : '';
  const barcode = sku.barcode ? ` (${sku.barcode})` : '';
  return `${sku.sku_code}${name}${barcode}`;
};

type BatchGovernanceStatus = 'NORMAL' | 'NEAR_EXPIRY' | 'FORBID_SALE';
type BatchGovernanceMeta = {
  status: BatchGovernanceStatus | null;
  label: string;
  color: string;
  isFood: boolean;
  remainingDays: number | null;
  productionDateText: string;
  shelfLifeText: string;
  governanceAction: string;
};

const batchGovernanceStatusMeta: Record<BatchGovernanceStatus, { label: string; color: string }> = {
  NORMAL: { label: '正常', color: 'green' },
  NEAR_EXPIRY: { label: '临期', color: 'orange' },
  FORBID_SALE: { label: '禁售', color: 'red' }
};

const formatDayCount = (value: number | null) => {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return '-';
  }
  return `${value} 天`;
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
  const selectedSkuId = Form.useWatch('sku_id', drawerForm);
  const selectedProductionDate = Form.useWatch('production_date', drawerForm);
  const selectedExpireAt = Form.useWatch('expire_at', drawerForm);

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

  const { data: skuMasterData } = useQuery({
    queryKey: ['sku-batches-runtime-skus', tenantId],
    queryFn: () => listSkus(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 5 * 60 * 1000
  });

  const { data: productMasterData } = useQuery({
    queryKey: ['sku-batches-runtime-products', tenantId],
    queryFn: () => listProducts(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 5 * 60 * 1000
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

  const skuRuntimeMap = useMemo(() => {
    const map = new Map<string, Sku>();
    skuMasterData?.data.forEach((item) => map.set(item.sku_id, item));
    return map;
  }, [skuMasterData?.data]);

  const productMap = useMemo(() => {
    const map = new Map<string, Product>();
    productMasterData?.data.forEach((item) => map.set(item.product_id, item));
    return map;
  }, [productMasterData?.data]);

  const selectedRuntimeSku = selectedSkuId ? skuRuntimeMap.get(selectedSkuId) : undefined;
  const selectedRuntimeProduct = selectedRuntimeSku
    ? productMap.get(selectedRuntimeSku.product_id)
    : undefined;
  const isFoodGovernedSku = getProductIndustryProfile(selectedRuntimeProduct) === 'FOOD';
  const foodPolicySummary = selectedRuntimeProduct
    ? getFoodPolicySummary(selectedRuntimeProduct)
    : '-';
  const selectedFoodAttrs = selectedRuntimeProduct ? readProductAttrs(selectedRuntimeProduct) : {};
  const selectedShelfLifeDays = selectedRuntimeProduct?.shelf_life_days ?? null;
  const selectedProductionDateEnabled = Boolean(selectedFoodAttrs.production_date_enabled);
  const batchGovernanceMetaMap = useMemo(() => {
    const map = new Map<string, BatchGovernanceMeta>();
    (data?.data ?? []).forEach((batch) => {
      const runtimeSku = skuRuntimeMap.get(batch.sku_id);
      const runtimeProduct = runtimeSku ? productMap.get(runtimeSku.product_id) : undefined;
      if (!runtimeSku || getProductIndustryProfile(runtimeProduct) !== 'FOOD') {
        map.set(batch.batch_id, {
          status: null,
          label: '-',
          color: 'default',
          isFood: false,
          remainingDays: null,
          productionDateText: '-',
          shelfLifeText: '-',
          governanceAction: '-'
        });
        return;
      }
      const expireAt = batch.expire_at ? dayjs(batch.expire_at) : null;
      const attrs = readProductAttrs(runtimeProduct);
      const nearExpiryWarningDays = readNumberAttr(attrs, 'near_expiry_warning_days') ?? 0;
      const forbidSaleDays = runtimeProduct?.forbid_sale_days ?? null;
      const shelfLifeDays = runtimeProduct?.shelf_life_days ?? null;
      const productionDateEnabled = Boolean(attrs.production_date_enabled);
      const remainingDays = expireAt?.isValid()
        ? expireAt.startOf('day').diff(dayjs().startOf('day'), 'day')
        : Number.POSITIVE_INFINITY;

      let status: BatchGovernanceStatus = 'NORMAL';
      if (typeof forbidSaleDays === 'number' && remainingDays <= forbidSaleDays) {
        status = 'FORBID_SALE';
      } else if (remainingDays <= nearExpiryWarningDays) {
        status = 'NEAR_EXPIRY';
      }

      const storedProductionDate = batch.production_date ? dayjs(batch.production_date) : null;
      const productionDateText = storedProductionDate?.isValid()
        ? storedProductionDate.format('YYYY-MM-DD')
        : productionDateEnabled && expireAt?.isValid() && typeof shelfLifeDays === 'number'
          ? expireAt.subtract(shelfLifeDays, 'day').format('YYYY-MM-DD')
          : productionDateEnabled
            ? '待补保质期规则'
            : '-';
      const governanceAction =
        status === 'FORBID_SALE'
          ? '冻结库存并复核'
          : status === 'NEAR_EXPIRY'
            ? '优先出库周转'
            : '按先进先出周转';

      map.set(batch.batch_id, {
        status,
        label: batchGovernanceStatusMeta[status].label,
        color: batchGovernanceStatusMeta[status].color,
        isFood: true,
        remainingDays: Number.isFinite(remainingDays) ? remainingDays : null,
        productionDateText,
        shelfLifeText: formatDayCount(typeof shelfLifeDays === 'number' ? shelfLifeDays : null),
        governanceAction
      });
    });
    return map;
  }, [data?.data, productMap, skuRuntimeMap]);
  const batchGovernanceSummary = useMemo(() => {
    const summary = {
      NORMAL: 0,
      NEAR_EXPIRY: 0,
      FORBID_SALE: 0
    } satisfies Record<BatchGovernanceStatus, number>;
    batchGovernanceMetaMap.forEach((meta) => {
      if (meta.status) {
        summary[meta.status] += 1;
      }
    });
    return summary;
  }, [batchGovernanceMetaMap]);
  const filteredBatches = useMemo(() => {
    const currentRows = data?.data ?? [];
    if (!filters.governance_status) {
      return currentRows;
    }
    return currentRows.filter((batch) => batchGovernanceMetaMap.get(batch.batch_id)?.status === filters.governance_status);
  }, [batchGovernanceMetaMap, data?.data, filters.governance_status]);
  const batchBlockReason = useMemo(() => {
    if (editingBatch) {
      return null;
    }
    if (!selectedSkuId) {
      return '请选择 SKU 后再保存批次。';
    }
    if (!selectedRuntimeSku) {
      return '当前 SKU 运行期主数据未加载完成，请稍后重试。';
    }
    if (!selectedRuntimeSku.batch_enabled) {
      return '当前 SKU 未启用批次管理，不能新建批次。';
    }
    if (isFoodGovernedSku && !selectedExpireAt) {
      return '食品 SKU 需先填写失效日期后才可保存。';
    }
    return null;
  }, [editingBatch, isFoodGovernedSku, selectedExpireAt, selectedRuntimeSku, selectedSkuId]);
  const canSaveBatch = canWrite && !batchBlockReason;
  const productionDatePreview = useMemo(() => {
    if (!isFoodGovernedSku || !selectedExpireAt) {
      return null;
    }
    if (!selectedProductionDateEnabled) {
      return null;
    }
    if (typeof selectedShelfLifeDays !== 'number') {
      return {
        type: 'warning' as const,
        message: '当前商品未配置保质期规则',
        description: '该食品 SKU 已启用生产日期管理，但缺少保质期(天)配置，暂时无法推算生产日期。'
      };
    }
    return {
      type: 'info' as const,
      message: '推算生产日期',
      description: `按保质期 ${selectedShelfLifeDays} 天推算，当前批次生产日期约为 ${selectedExpireAt
        .subtract(selectedShelfLifeDays, 'day')
        .format('YYYY-MM-DD')}，剩余保质期 ${selectedExpireAt
        .startOf('day')
        .diff(dayjs().startOf('day'), 'day')} 天。`
    };
  }, [
    isFoodGovernedSku,
    selectedExpireAt,
    selectedProductionDateEnabled,
    selectedShelfLifeDays
  ]);

  useEffect(() => {
    if (!drawerOpen || !isFoodGovernedSku || !selectedProductionDateEnabled) {
      return;
    }
    if (selectedProductionDate || !selectedExpireAt || typeof selectedShelfLifeDays !== 'number') {
      return;
    }
    drawerForm.setFieldValue('production_date', selectedExpireAt.subtract(selectedShelfLifeDays, 'day'));
  }, [
    drawerForm,
    drawerOpen,
    isFoodGovernedSku,
    selectedExpireAt,
    selectedProductionDate,
    selectedProductionDateEnabled,
    selectedShelfLifeDays
  ]);

  const loadSkuOptions = async (keyword?: string) => {
    setSkuLoading(true);
    try {
      const response = await searchSkus(tenantId, keyword ?? '');
      setSkuOptions(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'SKU ????');
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
        title: '批次号',
        dataIndex: 'batch_no'
      },
      {
        title: '失效日期',
        dataIndex: 'expire_at',
        render: (_, record) => formatDateTime(record.expire_at)
      },
      {
        title: '推算生产日期',
        key: 'production_date',
        render: (_, record) => {
          const meta = batchGovernanceMetaMap.get(record.batch_id);
          return (
            <span data-testid="sku-batches-production-date-cell">
              {meta?.productionDateText ?? '-'}
            </span>
          );
        }
      },
      {
        title: '剩余保质期',
        key: 'remaining_days',
        render: (_, record) => {
          const meta = batchGovernanceMetaMap.get(record.batch_id);
          return (
            <span data-testid="sku-batches-shelf-life-cell">
              {formatDayCount(meta?.remainingDays ?? null)}
            </span>
          );
        }
      },
      {
        title: '治理状态',
        key: 'governance_status',
        render: (_, record) => {
          const meta = batchGovernanceMetaMap.get(record.batch_id);
          if (!meta?.status) {
            return '-';
          }
          return (
            <Space direction="vertical" size={0}>
              <Tag color={meta.color} data-testid="sku-batches-status-tag">
                {meta.label}
              </Tag>
              <Typography.Text type="secondary" data-testid="sku-batches-governance-action">
                {meta.governanceAction}
              </Typography.Text>
            </Space>
          );
        }
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      },
      {
        title: '操作',
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
                production_date: record.production_date ? dayjs(record.production_date) : undefined,
                expire_at: record.expire_at ? dayjs(record.expire_at) : undefined
              });
            }}
          >
            编辑
          </Button>
        )
      }
    ],
    [batchGovernanceMetaMap, canWrite, drawerForm, skuMap]
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
      const productionDate = values.production_date ? values.production_date.toISOString() : undefined;
      const expireAt = values.expire_at ? values.expire_at.toISOString() : undefined;
      if (editingBatch) {
        setSaving(true);
        const payload: SkuBatchUpdateRequest = {
          batch_no: values.batch_no?.trim() || undefined,
          production_date: productionDate,
          expire_at: expireAt
        };
        await updateSkuBatch(tenantId, editingBatch.batch_id, payload);
        message.success('?????');
      } else {
        if (!values.sku_id) {
          message.error('??? SKU');
          return;
        }
        const runtimeSku = skuRuntimeMap.get(values.sku_id);
        const runtimeProduct = runtimeSku ? productMap.get(runtimeSku.product_id) : undefined;
        if (!runtimeSku) {
          message.error('SKU ??????????????');
          return;
        }
        if (!runtimeSku.batch_enabled) {
          message.error('?? SKU ??????????????');
          return;
        }
        if (getProductIndustryProfile(runtimeProduct) === 'FOOD' && !expireAt) {
          message.error('?? SKU ?????????????');
          return;
        }
        setSaving(true);
        const payload: SkuBatchCreateRequest = {
          sku_id: values.sku_id,
          batch_no: values.batch_no.trim(),
          production_date: productionDate,
          expire_at: expireAt
        };
        await createSkuBatch(tenantId, payload);
        message.success('?????');
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
          SKU 批次台账
        </Typography.Title>
        <Button
          type="primary"
          onClick={openCreateDrawer}
          disabled={!canWrite}
          data-testid="sku-batches-create-button"
        >
          新建批次
        </Button>
      </Space>
      <Alert
        type="info"
        showIcon
        data-testid="sku-batches-runtime-governance-summary"
        message="运行治理摘要"
        description="食品 SKU 在批次页会前置校验批次管理能力与失效日期，并把推算生产日期、剩余保质期、正常/临期/禁售状态与治理动作前台化展示。"
      />
      <Row gutter={16}>
        <Col xs={24} md={8}>
          <Card size="small" data-testid="sku-batches-summary-normal">
            <Typography.Text type="secondary">正常批次</Typography.Text>
            <Typography.Title level={4} style={{ margin: '8px 0 0' }}>
              {batchGovernanceSummary.NORMAL}
            </Typography.Title>
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card size="small" data-testid="sku-batches-summary-near-expiry">
            <Typography.Text type="secondary">临期批次</Typography.Text>
            <Typography.Title level={4} style={{ margin: '8px 0 0' }}>
              {batchGovernanceSummary.NEAR_EXPIRY}
            </Typography.Title>
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card size="small" data-testid="sku-batches-summary-forbid-sale">
            <Typography.Text type="secondary">禁售批次</Typography.Text>
            <Typography.Title level={4} style={{ margin: '8px 0 0' }}>
              {batchGovernanceSummary.FORBID_SALE}
            </Typography.Title>
          </Card>
        </Col>
      </Row>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="sku_id" label="SKU">
                <Select
                  showSearch
                  allowClear
                  placeholder="搜索 SKU"
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
              <Form.Item name="batch_no" label="批次号">
                <Input allowClear placeholder="批次号" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="governance_status" label="治理状态">
                <Select
                  allowClear
                  placeholder="全部"
                  data-testid="sku-batches-governance-status-filter"
                  onChange={(value) => {
                    setFilters((prev) => ({ ...prev, governance_status: value ?? undefined }));
                    setPagination((prev) => ({ ...prev, current: 1 }));
                  }}
                  options={[
                    { value: 'NORMAL', label: '正常' },
                    { value: 'NEAR_EXPIRY', label: '临期' },
                    { value: 'FORBID_SALE', label: '禁售' }
                  ]}
                />
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
      </Card>
      <UiModule03MainTableSection<SkuBatch>
        rowKey="batch_id"
        columns={columns}
        data={filteredBatches}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="批次台账加载失败"
        emptyText="暂无批次数据"
        total={filters.governance_status ? filteredBatches.length : total}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="sku-batches.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editingBatch ? '编辑批次' : '新建批次'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button
              type="primary"
              loading={saving}
              onClick={handleSave}
              disabled={!editingBatch && !canSaveBatch}
              data-testid="sku-batches-save-button"
            >
              保存批次
            </Button>
          </Space>
        }
      >
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          {isFoodGovernedSku ? (
            <Alert
              type="info"
              showIcon
              data-testid="sku-batches-food-governance-alert"
              message="食品治理策略"
              description={`${foodPolicySummary}。食品 SKU 新建批次时必须补齐失效日期，并按保质期规则推算生产日期。`}
            />
          ) : null}
          {productionDatePreview ? (
            <Alert
              type={productionDatePreview.type}
              showIcon
              data-testid="sku-batches-production-date-preview"
              message={productionDatePreview.message}
              description={productionDatePreview.description}
            />
          ) : null}
          {selectedSkuId && selectedRuntimeSku && !selectedRuntimeSku.batch_enabled ? (
            <Alert
              type="error"
              showIcon
              data-testid="sku-batches-batch-disabled-alert"
              message="批次能力已关闭"
              description="当前 SKU 未启用批次管理，本页将阻止继续保存。"
            />
          ) : null}
          {!editingBatch && batchBlockReason ? (
            <Alert
              type={selectedSkuId && selectedRuntimeSku && !selectedRuntimeSku.batch_enabled ? 'error' : 'warning'}
              showIcon
              data-testid="sku-batches-save-reason-alert"
              message="当前无法保存"
              description={batchBlockReason}
            />
          ) : null}
          <Form form={drawerForm} layout="vertical">
            <Form.Item
              name="sku_id"
              label="SKU"
              rules={[{ required: true, message: '请选择 SKU' }]}
            >
              <Select
                showSearch
                allowClear
                data-testid="sku-batches-drawer-sku"
                disabled={Boolean(editingBatch)}
                placeholder="搜索 SKU"
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
              label="批次号"
              rules={[{ required: true, message: '请输入批次号' }]}
            >
              <Input placeholder="请输入批次号" data-testid="sku-batches-drawer-batch-no" />
            </Form.Item>
            <Form.Item name="expire_at" label="失效日期">
              <DatePicker showTime style={{ width: '100%' }} data-testid="sku-batches-drawer-expire-at" />
            </Form.Item>
            <Form.Item name="production_date" label="生产日期">
              <DatePicker
                showTime
                style={{ width: '100%' }}
                data-testid="sku-batches-drawer-production-date"
              />
            </Form.Item>
          </Form>
        </Space>
      </Drawer>
    </Space>
  );

  return (
    <Tabs
      defaultActiveKey="batches"
      items={[
        { key: 'batches', label: '批次台账', children: batchListContent },
        { key: 'strategies', label: '治理策略', children: <SkuBatchStrategiesPanel /> }
      ]}
    />
  );
};
