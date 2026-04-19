import {
  Alert,
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
  Popconfirm,
  Row,
  Select,
  Space,
  Switch,
  Table,
  Tabs,
  Tag,
  Typography,
  Upload,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import type { UploadFile } from 'antd/es/upload/interface';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as XLSX from 'xlsx';

import {
  applyStockCount,
  cancelStockCount,
  confirmStockCountDiff,
  createStockCount,
  getStockCount,
  importStockCount,
  listStockCounts,
  updateStockCountItems
} from '@/api/stockCounts';
import type {
  StockCountCreatePayload,
  StockCountFilters,
  StockCountImportItem,
  StockCountItem,
  StockCountListQuery,
  StockCountSummary
} from '@/api/stockCounts';
import type { SkuSearchItem } from '@/api/skus';
import { searchSkus } from '@/api/skus';
import type { OperationLog } from '@/api/system';
import { listOperationLogs } from '@/api/system';
import { listWarehouseAreas } from '@/api/warehouseAreas';
import { listWarehouseLocations } from '@/api/warehouseLocations';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { ErrorState } from '@/components/QueryState';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const statusMeta: Record<string, { label: string; color: string }> = {
  COUNTING: { label: '盘点中', color: 'blue' },
  DIFF_CONFIRMED: { label: '待复核', color: 'orange' },
  APPLIED: { label: '已完成', color: 'green' },
  CANCELLED: { label: '已作废', color: 'default' }
};

const scopeTypeLabels: Record<string, string> = {
  FULL: '全仓',
  AREA: '库区',
  LOCATION: '货位',
  SKU: '指定货品'
};

const countModeLabels: Record<string, string> = {
  OPEN: '明盘',
  BLIND: '暗盘'
};

const countTypeLabels: Record<string, string> = {
  DOCUMENT: '有盘点单',
  IMPORT: '导入盘点'
};

const diffReasonOptions = [
  { label: '盘点差异', value: '盘点差异' },
  { label: '库位调整', value: '库位调整' },
  { label: '损耗', value: '损耗' },
  { label: '其他', value: '其他' }
];

type FilterValues = {
  warehouse_id?: string;
  status?: string;
  count_type?: string;
  scope_type?: string;
  keyword?: string;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type CreateFormValues = {
  warehouse_id: string;
  scope_type: string;
  scope_ids?: string[];
  count_mode: string;
  count_by_location?: boolean;
  freeze_stock?: boolean;
};

type EntryFormValues = {
  sku_id: string;
  location_id?: string;
  counted_qty: number;
};

type ImportFormValues = {
  warehouse_id: string;
  scope_type?: string;
  count_mode?: string;
  freeze_stock?: boolean;
};

const normalizeScopeType = (filters: StockCountFilters, dimension?: string): string => {
  if (filters.scope_type) {
    return filters.scope_type;
  }
  if (dimension === 'LOCATION') {
    return 'LOCATION';
  }
  return 'FULL';
};

const resolveScopeLabel = (filters: StockCountFilters, dimension?: string) => {
  const scope = normalizeScopeType(filters, dimension);
  return scopeTypeLabels[scope] ?? scope;
};

const resolveCountMode = (filters: StockCountFilters) =>
  countModeLabels[filters.count_mode ?? 'OPEN'] ?? filters.count_mode ?? '-';

const resolveCountType = (filters: StockCountFilters, strategy?: string) => {
  const type = filters.count_type ?? (strategy === 'IMPORT' ? 'IMPORT' : 'DOCUMENT');
  return countTypeLabels[type] ?? type;
};

const resolveStatusTag = (status?: string) => {
  const meta = status ? statusMeta[status] : undefined;
  return <Tag color={meta?.color ?? 'default'}>{meta?.label ?? status ?? '-'}</Tag>;
};

const normalizeHeader = (value: string) => value.trim().toLowerCase();

const importHeaderAliases: Record<string, keyof StockCountImportItem> = {
  sku_id: 'sku_id',
  sku: 'sku_id',
  'sku编码': 'sku_id',
  'sku id': 'sku_id',
  'sku code': 'sku_id',
  货品: 'sku_id',
  商品: 'sku_id',
  location_id: 'location_id',
  location: 'location_id',
  'location code': 'location_id',
  货位: 'location_id',
  库位: 'location_id',
  货位编码: 'location_id',
  库位编码: 'location_id',
  counted_qty: 'counted_qty',
  count_qty: 'counted_qty',
  'counted qty': 'counted_qty',
  盘点数量: 'counted_qty',
  实盘数量: 'counted_qty',
  数量: 'counted_qty',
  diff_reason: 'diff_reason',
  reason: 'diff_reason',
  差异原因: 'diff_reason',
  备注: 'diff_reason'
};

const parseImportRows = async (file: File) => {
  const data = await file.arrayBuffer();
  const workbook = XLSX.read(data, { type: 'array' });
  const sheetName = workbook.SheetNames[0];
  if (!sheetName) {
    throw new Error('未找到可用工作表');
  }
  const sheet = workbook.Sheets[sheetName];
  const rows = XLSX.utils.sheet_to_json<Record<string, unknown>>(sheet, { defval: '' });
  const items: StockCountImportItem[] = [];
  const errors: string[] = [];

  rows.forEach((row, index) => {
    const record: Partial<StockCountImportItem> = {};
    Object.entries(row).forEach(([key, value]) => {
      const mapped = importHeaderAliases[normalizeHeader(key)];
      if (!mapped) {
        return;
      }
      if (mapped === 'counted_qty') {
        record.counted_qty = Number(value);
        return;
      }
      record[mapped] = typeof value === 'string' ? value.trim() : String(value).trim();
    });

    const rowLabel = `第 ${index + 2} 行`;
    if (!record.sku_id) {
      errors.push(`${rowLabel} SKU 不能为空`);
      return;
    }
    if (record.counted_qty == null || Number.isNaN(record.counted_qty)) {
      errors.push(`${rowLabel} 盘点数量不能为空`);
      return;
    }
    if ((record.counted_qty ?? 0) < 0) {
      errors.push(`${rowLabel} 盘点数量需为非负`);
      return;
    }
    if (record.diff_reason && !record.diff_reason.trim()) {
      record.diff_reason = undefined;
    }
    items.push({
      sku_id: record.sku_id,
      location_id: record.location_id,
      counted_qty: record.counted_qty ?? 0,
      diff_reason: record.diff_reason
    });
  });

  return { items, errors };
};

export const StocktakesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);
  const [importOpen, setImportOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [reviewOpen, setReviewOpen] = useState(false);
  const [applyOpen, setApplyOpen] = useState(false);
  const [activeCountId, setActiveCountId] = useState<string | null>(null);
  const [countedMap, setCountedMap] = useState<Record<string, number>>({});
  const [reasonMap, setReasonMap] = useState<Record<string, string>>({});
  const [bulkCounted, setBulkCounted] = useState<number | null>(null);
  const [importFileList, setImportFileList] = useState<UploadFile[]>([]);
  const [importItems, setImportItems] = useState<StockCountImportItem[]>([]);
  const [importErrors, setImportErrors] = useState<string[]>([]);
  const [skuOptions, setSkuOptions] = useState<{ label: string; value: string }[]>([]);
  const [detailTab, setDetailTab] = useState('items');
  const [filterForm] = Form.useForm<FilterValues>();
  const [createForm] = Form.useForm<CreateFormValues>();
  const [entryForm] = Form.useForm<EntryFormValues>();
  const [importForm] = Form.useForm<ImportFormValues>();

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'stocktakes'],
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

  const createWarehouseId = Form.useWatch('warehouse_id', createForm);
  const createScopeType = Form.useWatch('scope_type', createForm);

  const areaQuery = useQuery({
    queryKey: ['warehouse-areas', tenantId, createWarehouseId, createScopeType],
    queryFn: () =>
      listWarehouseAreas(tenantId, {
        warehouse_id: createWarehouseId,
        status: 'ACTIVE',
        offset: 0,
        limit: 200
      }),
    enabled: tenantReady && createOpen && Boolean(createWarehouseId) && createScopeType === 'AREA'
  });

  const locationQuery = useQuery({
    queryKey: ['warehouse-locations', tenantId, createWarehouseId, createScopeType],
    queryFn: () =>
      listWarehouseLocations(tenantId, {
        warehouse_id: createWarehouseId,
        offset: 0,
        limit: 200
      }),
    enabled: tenantReady && createOpen && Boolean(createWarehouseId) && createScopeType === 'LOCATION'
  });

  const areaOptions = useMemo(
    () =>
      areaQuery.data?.items.map((area) => ({
        label: `${area.code}-${area.name}`,
        value: area.area_id
      })) ?? [],
    [areaQuery.data?.items]
  );

  const locationOptions = useMemo(
    () =>
      locationQuery.data?.items.map((location) => ({
        label: `${location.code}${location.name ? `-${location.name}` : ''}`,
        value: location.location_id
      })) ?? [],
    [locationQuery.data?.items]
  );

  const listQuery = useMemo<StockCountListQuery>(() => {
    const range = filters.created_range;
    return {
      warehouse_id: filters.warehouse_id,
      status: filters.status,
      count_type: filters.count_type,
      scope_type: filters.scope_type,
      keyword: filters.keyword,
      created_from: range?.[0]?.startOf('day').toISOString(),
      created_to: range?.[1]?.endOf('day').toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const listQueryState = useQuery({
    queryKey: ['stocktakes', tenantId, listQuery],
    queryFn: () => listStockCounts(tenantId, listQuery),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const detailQuery = useQuery({
    queryKey: ['stocktakes', tenantId, activeCountId],
    queryFn: () => getStockCount(tenantId, activeCountId ?? ''),
    enabled: tenantReady && Boolean(activeCountId),
    onSuccess: (detail) => {
      const nextCounted: Record<string, number> = {};
      const nextReasons: Record<string, string> = {};
      detail.items.forEach((item) => {
        nextCounted[item.count_item_id] = item.counted_qty;
        if (item.diff_reason) {
          nextReasons[item.count_item_id] = item.diff_reason;
        }
      });
      setCountedMap(nextCounted);
      setReasonMap(nextReasons);
    }
  });

  const logsQuery = useQuery({
    queryKey: ['stocktakes', tenantId, activeCountId, 'logs'],
    queryFn: () =>
      listOperationLogs(tenantId, {
        ref_type: 'STOCK_COUNT',
        ref_id: activeCountId ?? undefined,
        limit: 50
      }),
    enabled: tenantReady && Boolean(activeCountId) && detailOpen
  });

  const createMutation = useMutation({
    mutationFn: async () => {
      const values = await createForm.validateFields();
      const scopeType = values.scope_type;
      let dimension = values.count_by_location ? 'LOCATION' : 'SKU';
      if (scopeType === 'AREA' || scopeType === 'LOCATION') {
        dimension = 'LOCATION';
      }
      if (scopeType === 'SKU') {
        dimension = 'SKU';
      }
      const payload: StockCountCreatePayload = {
        warehouse_id: values.warehouse_id,
        strategy: 'FULL',
        dimension,
        scope_type: scopeType,
        scope_ids: values.scope_ids,
        count_mode: values.count_mode,
        count_type: 'DOCUMENT',
        freeze_stock: values.freeze_stock ?? false
      };
      return createStockCount(tenantId, payload);
    },
    onSuccess: async () => {
      message.success('盘点单已创建');
      setCreateOpen(false);
      createForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['stocktakes', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '创建盘点单失败');
    }
  });

  const updateMutation = useMutation({
    mutationFn: async (items: StockCountItem[]) => {
      const updates = items.map((item) => ({
        count_item_id: item.count_item_id,
        counted_qty: countedMap[item.count_item_id] ?? item.counted_qty,
        diff_reason: reasonMap[item.count_item_id]
      }));
      return updateStockCountItems(tenantId, activeCountId ?? '', updates);
    },
    onSuccess: (detail) => {
      message.success('盘点数据已保存');
      queryClient.setQueryData(['stocktakes', tenantId, activeCountId], detail);
      const nextCounted: Record<string, number> = {};
      const nextReasons: Record<string, string> = {};
      detail.items.forEach((item) => {
        nextCounted[item.count_item_id] = item.counted_qty;
        if (item.diff_reason) {
          nextReasons[item.count_item_id] = item.diff_reason;
        }
      });
      setCountedMap(nextCounted);
      setReasonMap(nextReasons);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存盘点数据失败');
    }
  });

  const entryMutation = useMutation({
    mutationFn: async (values: EntryFormValues) => {
      if (!activeCountId) {
        throw new Error('请先选择盘点单');
      }
      const detail = detailQuery.data;
      if (!detail) {
        throw new Error('盘点明细未加载');
      }
      const skuId = values.sku_id.trim();
      if (!skuId) {
        throw new Error('请输入 SKU');
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
      queryClient.setQueryData(['stocktakes', tenantId, activeCountId], detail);
      const nextCounted: Record<string, number> = {};
      const nextReasons: Record<string, string> = {};
      detail.items.forEach((item) => {
        nextCounted[item.count_item_id] = item.counted_qty;
        if (item.diff_reason) {
          nextReasons[item.count_item_id] = item.diff_reason;
        }
      });
      setCountedMap(nextCounted);
      setReasonMap(nextReasons);
      entryForm.resetFields();
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '盘点录入失败');
    }
  });

  const confirmMutation = useMutation({
    mutationFn: () => confirmStockCountDiff(tenantId, activeCountId ?? ''),
    onSuccess: (detail) => {
      message.success('复核完成');
      queryClient.setQueryData(['stocktakes', tenantId, activeCountId], detail);
      queryClient.invalidateQueries({ queryKey: ['stocktakes', tenantId] });
      setReviewOpen(false);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '复核失败');
    }
  });

  const applyMutation = useMutation({
    mutationFn: () => applyStockCount(tenantId, activeCountId ?? ''),
    onSuccess: (detail) => {
      message.success('库存调整已生成');
      queryClient.setQueryData(['stocktakes', tenantId, activeCountId], detail);
      queryClient.invalidateQueries({ queryKey: ['stocktakes', tenantId] });
      setApplyOpen(false);
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '生成调整失败');
    }
  });

  const cancelMutation = useMutation({
    mutationFn: (countId: string) => cancelStockCount(tenantId, countId),
    onSuccess: (detail, countId) => {
      message.success('盘点单已作废');
      queryClient.setQueryData(['stocktakes', tenantId, countId], detail);
      queryClient.invalidateQueries({ queryKey: ['stocktakes', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '作废失败');
    }
  });

  const importMutation = useMutation({
    mutationFn: async (payload: StockCountImportItem[]) => {
      const values = await importForm.validateFields();
      const dimension = payload.some((item) => item.location_id) ? 'LOCATION' : 'SKU';
      const scopeType = values.scope_type ?? (dimension === 'LOCATION' ? 'LOCATION' : 'SKU');
      return importStockCount(tenantId, {
        warehouse_id: values.warehouse_id,
        dimension,
        scope_type: scopeType,
        count_mode: values.count_mode ?? 'OPEN',
        count_type: 'IMPORT',
        freeze_stock: values.freeze_stock ?? false,
        items: payload
      });
    },
    onSuccess: async () => {
      message.success('盘点数据已导入');
      setImportOpen(false);
      setImportFileList([]);
      setImportItems([]);
      setImportErrors([]);
      await queryClient.invalidateQueries({ queryKey: ['stocktakes', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '导入失败');
    }
  });

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    filterForm.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleSkuSearch = async (keyword: string) => {
    if (!tenantId) {
      return;
    }
    const result = await searchSkus(tenantId, keyword);
    const options = result.items.map((sku: SkuSearchItem) => ({
      label: `${sku.sku_code ?? sku.sku_id} ${sku.name ?? ''}`.trim(),
      value: sku.sku_id
    }));
    setSkuOptions(options);
  };

  const handleExportList = () => {
    const rows = listQueryState.data?.items ?? [];
    if (rows.length === 0) {
      message.info('暂无可导出的盘点单');
      return;
    }
    const header = [
      'count_id',
      'warehouse',
      'scope',
      'count_mode',
      'count_type',
      'status',
      'created_at'
    ];
    const csv = [
      header.join(','),
      ...rows.map((row) => {
        const scope = resolveScopeLabel(row.filters, row.dimension);
        const countMode = resolveCountMode(row.filters);
        const countType = resolveCountType(row.filters, row.strategy);
        const status = statusMeta[row.status]?.label ?? row.status;
        return [
          row.count_id,
          warehouseMap.get(row.warehouse_id) ?? row.warehouse_id,
          scope,
          countMode,
          countType,
          status,
          formatDateTime(row.created_at)
        ].join(',');
      })
    ].join('\n');
    const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `stocktakes-${dayjs().format('YYYYMMDDHHmmss')}.csv`;
    link.click();
    URL.revokeObjectURL(url);
  };

  const handleImportFile = async (file: File) => {
    const { items, errors } = await parseImportRows(file);
    setImportItems(items);
    setImportErrors(errors);
  };

  const handleBulkFill = () => {
    if (bulkCounted == null) {
      message.warning('请输入批量填充值');
      return;
    }
    const next: Record<string, number> = {};
    detailQuery.data?.items.forEach((item) => {
      next[item.count_item_id] = bulkCounted;
    });
    setCountedMap(next);
  };

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
        title: '盘点范围',
        render: (_, record) => resolveScopeLabel(record.filters, record.dimension)
      },
      {
        title: '盘点方式',
        render: (_, record) => resolveCountMode(record.filters)
      },
      {
        title: '盘点类型',
        render: (_, record) => resolveCountType(record.filters, record.strategy)
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) => resolveStatusTag(value)
      },
      {
        title: '创建人',
        render: (_, record) => record.filters.created_by ?? '-'
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
            <Popconfirm
              title="确认作废盘点单？"
              okText="作废"
              cancelText="取消"
              disabled={record.status === 'COUNTING' || record.status === 'APPLIED'}
              onConfirm={() => {
                setActiveCountId(record.count_id);
                cancelMutation.mutate(record.count_id);
              }}
            >
              <Button
                size="small"
                danger
                disabled={
                  !canWrite ||
                  record.status === 'COUNTING' ||
                  record.status === 'APPLIED' ||
                  record.status === 'CANCELLED'
                }
              >
                作废
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [cancelMutation, canWrite, warehouseMap]
  );

  const detail = detailQuery.data;
  const detailItems = detail?.items ?? [];
  const detailFilters = detail?.count.filters ?? {};
  const countMode = resolveCountMode(detailFilters);
  const scopeLabel = detail ? resolveScopeLabel(detailFilters, detail.count.dimension) : '-';
  const detailStatus = detail?.count.status ?? '-';
  const showLocation = detail?.count.dimension === 'LOCATION';
  const canConfirm = detail?.count.status === 'COUNTING';
  const canApply = detail?.count.status === 'DIFF_CONFIRMED';

  const detailColumns = useMemo<ColumnsType<StockCountItem>>(
    () => {
      const cols: ColumnsType<StockCountItem> = [
        { title: 'SKU/SKU ID', dataIndex: 'sku_id' }
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
          title: '账面数量',
          dataIndex: 'system_qty',
          render: (value) => value ?? 0
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
          title: '差异数量',
          dataIndex: 'diff_qty',
          render: (_, record) => {
            const counted = countedMap[record.count_item_id] ?? record.counted_qty;
            return counted - record.system_qty;
          }
        },
        {
          title: '差异原因',
          dataIndex: 'diff_reason',
          render: (_, record) => (
            <Select
              allowClear
              options={diffReasonOptions}
              value={reasonMap[record.count_item_id] ?? record.diff_reason ?? undefined}
              onChange={(value) =>
                setReasonMap((prev) => ({
                  ...prev,
                  [record.count_item_id]: value ?? ''
                }))
              }
              style={{ minWidth: 140 }}
            />
          )
        }
      );
      return cols;
    },
    [countedMap, reasonMap, showLocation]
  );

  const diffRows = useMemo(
    () =>
      detailItems.filter((item) => {
        const counted = countedMap[item.count_item_id] ?? item.counted_qty;
        return counted - item.system_qty !== 0;
      }),
    [countedMap, detailItems]
  );

  const diffColumns = useMemo<ColumnsType<StockCountItem>>(
    () =>
      detailColumns.filter((column) =>
        ['SKU/SKU ID', '货位', '账面数量', '盘点数量', '差异数量', '差异原因'].includes(
          typeof column.title === 'string' ? column.title : ''
        )
      ),
    [detailColumns]
  );

  const logColumns = useMemo<ColumnsType<OperationLog>>(
    () => [
      { title: '动作', dataIndex: 'action' },
      { title: '操作人', dataIndex: 'actor_user_id', render: (value) => value ?? '-' },
      { title: '类型', dataIndex: 'ref_type', render: (value) => value ?? '-' },
      { title: '关联ID', dataIndex: 'ref_id', render: (value) => value ?? '-' },
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      }
    ],
    []
  );

  useEffect(() => {
    if (!detailOpen) {
      setActiveCountId(null);
      entryForm.resetFields();
      setDetailTab('items');
    }
  }, [detailOpen, entryForm]);
  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Row justify="space-between" align="middle">
        <Typography.Title level={3} style={{ margin: 0 }}>
          盘点管理
        </Typography.Title>
        <Space>
          <Button type="primary" onClick={() => setCreateOpen(true)} disabled={!canWrite}>
            新建盘点单
          </Button>
          <Button onClick={() => setImportOpen(true)} disabled={!canWrite}>
            导入盘点数据
          </Button>
          <Button onClick={handleExportList}>导出</Button>
        </Space>
      </Row>
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
      <Card title="筛选">
        <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="仓库">
                <Select allowClear placeholder="全部仓库" options={warehouseOptions} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select
                  allowClear
                  placeholder="全部状态"
                  options={[
                    { label: '盘点中', value: 'COUNTING' },
                    { label: '待复核', value: 'DIFF_CONFIRMED' },
                    { label: '已完成', value: 'APPLIED' },
                    { label: '已作废', value: 'CANCELLED' }
                  ]}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="count_type" label="盘点类型">
                <Select
                  allowClear
                  placeholder="全部类型"
                  options={[
                    { label: '有盘点单', value: 'DOCUMENT' },
                    { label: '导入盘点', value: 'IMPORT' }
                  ]}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="scope_type" label="盘点范围">
                <Select
                  allowClear
                  placeholder="全部范围"
                  options={[
                    { label: '全仓', value: 'FULL' },
                    { label: '库区', value: 'AREA' },
                    { label: '货位', value: 'LOCATION' },
                    { label: '指定货品', value: 'SKU' }
                  ]}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="created_range" label="日期范围">
                <DatePicker.RangePicker allowClear style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="keyword" label="关键字">
                <Input allowClear placeholder="盘点单号" />
              </Form.Item>
            </Col>
            <Col
              xs={24}
              md={10}
              style={{ display: 'flex', alignItems: 'end', justifyContent: 'flex-end' }}
            >
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
      <Card title="盘点单列表">
        <UiModule03MainTableSection<StockCountSummary>
          rowKey="count_id"
          columns={columns}
          data={listQueryState.data?.items ?? []}
          loading={listQueryState.isLoading || listQueryState.isFetching}
          error={listQueryState.error}
          onRetry={() => listQueryState.refetch()}
          errorTitle="盘点单列表加载失败"
          emptyText="暂无盘点单"
          total={listQueryState.data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="stocktakes.table.columns"
          enablePanels={false}
        />
      </Card>
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
          initialValues={{ scope_type: 'FULL', count_mode: 'OPEN', freeze_stock: false }}
        >
          <Form.Item
            name="warehouse_id"
            label="仓库"
            rules={[{ required: true, message: '请选择仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
          <Form.Item
            name="scope_type"
            label="盘点范围"
            rules={[{ required: true, message: '请选择盘点范围' }]}
          >
            <Select
              options={[
                { label: '全仓', value: 'FULL' },
                { label: '库区', value: 'AREA' },
                { label: '货位', value: 'LOCATION' },
                { label: '指定货品', value: 'SKU' }
              ]}
            />
          </Form.Item>
          <Form.Item shouldUpdate={(prev, next) => prev.scope_type !== next.scope_type} noStyle>
            {({ getFieldValue }) => {
              const scopeType = getFieldValue('scope_type');
              if (scopeType === 'AREA') {
                return (
                  <Form.Item
                    name="scope_ids"
                    label="选择库区"
                    rules={[{ required: true, message: '请选择库区' }]}
                  >
                    <Select mode="multiple" options={areaOptions} placeholder="选择库区" />
                  </Form.Item>
                );
              }
              if (scopeType === 'LOCATION') {
                return (
                  <Form.Item
                    name="scope_ids"
                    label="选择货位"
                    rules={[{ required: true, message: '请选择货位' }]}
                  >
                    <Select mode="multiple" options={locationOptions} placeholder="选择货位" />
                  </Form.Item>
                );
              }
              if (scopeType === 'SKU') {
                return (
                  <Form.Item
                    name="scope_ids"
                    label="选择货品"
                    rules={[{ required: true, message: '请选择货品' }]}
                  >
                    <Select
                      mode="multiple"
                      options={skuOptions}
                      placeholder="搜索并选择 SKU"
                      showSearch
                      filterOption={false}
                      onSearch={handleSkuSearch}
                    />
                  </Form.Item>
                );
              }
              return null;
            }}
          </Form.Item>
          <Form.Item
            name="count_mode"
            label="盘点方式"
            rules={[{ required: true, message: '请选择盘点方式' }]}
          >
            <Select
              options={[
                { label: '明盘', value: 'OPEN' },
                { label: '暗盘', value: 'BLIND' }
              ]}
            />
          </Form.Item>
          <Form.Item shouldUpdate={(prev, next) => prev.scope_type !== next.scope_type} noStyle>
            {({ getFieldValue }) =>
              getFieldValue('scope_type') === 'FULL' ? (
                <Form.Item name="count_by_location" label="是否按库位" valuePropName="checked">
                  <Switch />
                </Form.Item>
              ) : null
            }
          </Form.Item>
          <Form.Item name="freeze_stock" label="冻结出入库" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="导入盘点数据"
        open={importOpen}
        onCancel={() => setImportOpen(false)}
        footer={null}
        width={720}
        destroyOnHidden
      >
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Alert
            type="info"
            showIcon
            message="支持 Excel/CSV 文件"
            description="字段示例：sku_id、location_id(可选)、counted_qty、diff_reason。SKU 需使用系统内 SKU ID。"
          />
          <Form
            form={importForm}
            layout="vertical"
            initialValues={{ count_mode: 'OPEN', scope_type: 'SKU', freeze_stock: false }}
          >
            <Form.Item
              name="warehouse_id"
              label="仓库"
              rules={[{ required: true, message: '请选择仓库' }]}
            >
              <Select options={warehouseOptions} placeholder="选择仓库" />
            </Form.Item>
            <Form.Item name="scope_type" label="盘点范围">
              <Select
                options={[
                  { label: '指定货品', value: 'SKU' },
                  { label: '货位', value: 'LOCATION' }
                ]}
              />
            </Form.Item>
            <Form.Item name="count_mode" label="盘点方式">
              <Select
                options={[
                  { label: '明盘', value: 'OPEN' },
                  { label: '暗盘', value: 'BLIND' }
                ]}
              />
            </Form.Item>
            <Form.Item name="freeze_stock" label="冻结出入库" valuePropName="checked">
              <Switch />
            </Form.Item>
          </Form>
          <Card size="small">
            <Space direction="vertical" size={8} style={{ width: '100%' }}>
              <Typography.Text strong>1. 上传文件</Typography.Text>
              <Upload
                beforeUpload={() => false}
                fileList={importFileList}
                accept=".xlsx,.xls,.csv"
                onChange={async ({ fileList }) => {
                  const next = fileList.slice(-1);
                  setImportFileList(next);
                  setImportItems([]);
                  setImportErrors([]);
                  const file = next[0]?.originFileObj;
                  if (file instanceof File) {
                    try {
                      await handleImportFile(file);
                    } catch (err) {
                      message.error(err instanceof Error ? err.message : '解析文件失败');
                    }
                  }
                }}
              >
                <Button disabled={!canWrite}>选择文件</Button>
              </Upload>
              {importErrors.length ? (
                <Alert
                  type="error"
                  showIcon
                  message="导入校验未通过"
                  description={
                    <Space direction="vertical">
                      {importErrors.slice(0, 6).map((err) => (
                        <Typography.Text key={err}>{err}</Typography.Text>
                      ))}
                      {importErrors.length > 6 ? (
                        <Typography.Text>...</Typography.Text>
                      ) : null}
                    </Space>
                  }
                />
              ) : null}
            </Space>
          </Card>
          <Card size="small" title={`预览 (${importItems.length} 行)`}>
            <Table<StockCountImportItem>
              rowKey={(record, index) => `${record.sku_id}-${record.location_id ?? 'NA'}-${index}`}
              size="small"
              columns={[
                { title: 'SKU', dataIndex: 'sku_id' },
                { title: '货位', dataIndex: 'location_id', render: (value) => value ?? '-' },
                { title: '盘点数量', dataIndex: 'counted_qty' },
                { title: '差异原因', dataIndex: 'diff_reason', render: (value) => value ?? '-' }
              ]}
              dataSource={importItems}
              pagination={{ pageSize: 8 }}
            />
          </Card>
          <Space style={{ justifyContent: 'flex-end', width: '100%' }}>
            <Button onClick={() => setImportOpen(false)}>取消</Button>
            <Button
              type="primary"
              loading={importMutation.isPending}
              disabled={!canWrite || importItems.length === 0 || importErrors.length > 0}
              onClick={() => importMutation.mutate(importItems)}
            >
              确认导入
            </Button>
          </Space>
        </Space>
      </Modal>
      <Drawer
        title="盘点单详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={860}
        destroyOnHidden
      >
        {detailQuery.isLoading ? (
          <Card loading />
        ) : detail ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions bordered size="small" column={2}>
              <Descriptions.Item label="盘点单号">{detail.count.count_id}</Descriptions.Item>
              <Descriptions.Item label="仓库">
                {warehouseMap.get(detail.count.warehouse_id) ?? detail.count.warehouse_id}
              </Descriptions.Item>
              <Descriptions.Item label="盘点范围">{scopeLabel}</Descriptions.Item>
              <Descriptions.Item label="盘点方式">{countMode}</Descriptions.Item>
              <Descriptions.Item label="盘点类型">
                {resolveCountType(detailFilters, detail.count.strategy)}
              </Descriptions.Item>
              <Descriptions.Item label="状态">{resolveStatusTag(detailStatus)}</Descriptions.Item>
              <Descriptions.Item label="创建时间">
                {formatDateTime(detail.count.created_at)}
              </Descriptions.Item>
              <Descriptions.Item label="冻结出入库">
                {detailFilters.freeze_stock ? '是' : '否'}
              </Descriptions.Item>
            </Descriptions>
            <Alert
              type="info"
              showIcon
              message="提示"
              description="盘点数量不能为空且需为非负；如启用批次/序列号，请在明细中完善对应信息。"
            />
            <Space>
              <InputNumber
                min={0}
                value={bulkCounted}
                placeholder="批量填充"
                onChange={(value) => setBulkCounted(value)}
              />
              <Button onClick={handleBulkFill}>批量填充</Button>
              <Button
                onClick={() => updateMutation.mutate(detailItems)}
                loading={updateMutation.isPending}
                disabled={!canWrite}
              >
                保存盘点数据
              </Button>
              <Button
                onClick={() => setReviewOpen(true)}
                loading={confirmMutation.isPending}
                disabled={!canConfirm || !canWrite}
              >
                复核差异
              </Button>
              <Button
                type="primary"
                onClick={() => setApplyOpen(true)}
                loading={applyMutation.isPending}
                disabled={!canApply || !canWrite}
              >
                生成调整
              </Button>
            </Space>
            <Tabs
              activeKey={detailTab}
              onChange={(key) => setDetailTab(key)}
              items={[
                {
                  key: 'items',
                  label: '明细',
                  children: (
                    <Space direction="vertical" size={12} style={{ width: '100%' }}>
                      <Form
                        form={entryForm}
                        layout="inline"
                        onFinish={(values) => entryMutation.mutate(values)}
                        initialValues={{ counted_qty: 0 }}
                      >
                        <Form.Item
                          name="sku_id"
                          label="SKU"
                          rules={[{ required: true, message: '请输入 SKU' }]}
                        >
                          <Input placeholder="SKU ID" />
                        </Form.Item>
                        {showLocation ? (
                          <Form.Item
                            name="location_id"
                            label="货位"
                            rules={[{ required: true, message: '请输入货位' }]}
                          >
                            <Input placeholder="货位 ID" />
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
                      <Table
                        rowKey="count_item_id"
                        columns={detailColumns}
                        dataSource={detailItems}
                        pagination={{ pageSize: 12 }}
                        size="small"
                      />
                    </Space>
                  )
                },
                {
                  key: 'diff',
                  label: `差异 (${diffRows.length})`,
                  children: (
                    <Table
                      rowKey="count_item_id"
                      columns={diffColumns}
                      dataSource={diffRows}
                      pagination={{ pageSize: 10 }}
                      size="small"
                    />
                  )
                },
                {
                  key: 'logs',
                  label: '操作记录',
                  children: logsQuery.error ? (
                    <ErrorState
                      title="操作记录加载失败"
                      error={logsQuery.error}
                      onRetry={() => logsQuery.refetch()}
                    />
                  ) : (
                    <Table
                      rowKey="log_id"
                      columns={logColumns}
                      dataSource={logsQuery.data?.data ?? []}
                      pagination={{ pageSize: 8 }}
                      size="small"
                    />
                  )
                }
              ]}
            />
          </Space>
        ) : null}
      </Drawer>
      <Modal
        title="复核确认"
        open={reviewOpen}
        onCancel={() => setReviewOpen(false)}
        onOk={() => confirmMutation.mutate()}
        confirmLoading={confirmMutation.isPending}
        okButtonProps={{ disabled: !canConfirm }}
      >
        <Typography.Text>确认复核差异并进入生成调整阶段？</Typography.Text>
      </Modal>
      <Modal
        title="生成调整确认"
        open={applyOpen}
        onCancel={() => setApplyOpen(false)}
        onOk={() => applyMutation.mutate()}
        confirmLoading={applyMutation.isPending}
        okButtonProps={{ disabled: !canApply }}
      >
        <Typography.Text>生成库存调整单并完成盘点，确认继续？</Typography.Text>
      </Modal>
    </Space>
  );
};
