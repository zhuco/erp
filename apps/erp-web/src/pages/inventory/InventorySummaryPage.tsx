import {
  Alert,
  Button,
  Card,
  Checkbox,
  Col,
  Descriptions,
  Divider,
  Empty,
  Form,
  Input,
  InputNumber,
  Modal,
  Popover,
  Row,
  Select,
  Space,
  Tabs,
  Tag,
  Tooltip,
  Tree,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { DataNode } from 'antd/es/tree';
import {
  ExportOutlined,
  PlusOutlined,
  QuestionCircleOutlined,
  ReloadOutlined,
  SettingOutlined,
  ShareAltOutlined
} from '@ant-design/icons';
import dayjs from 'dayjs';
import type { ReactNode } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type { InventorySummary, InventorySummaryListResponse } from '@/api/inventory';
import { listInventory, listInventorySummary } from '@/api/inventory';
import type { InventoryReconciliationResult } from '@/api/inventoryGovernance';
import { checkInventoryReconciliation, getInventoryAtp } from '@/api/inventoryGovernance';
import { createExportJob } from '@/api/exports';
import { addOrderNote, getOrderDetail } from '@/api/orders';
import { getSkuDetail, updateSku } from '@/api/skus';
import { listWarehouses } from '@/api/warehouses';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { WorkbenchListPage } from '@/components/workbench/WorkbenchListPage';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

import './inventorySummaryPage.css';

type InventorySummaryRow = InventorySummary & {
  sku_name?: string | null;
  spec?: string | null;
  brand?: string | null;
  sales_yesterday?: number | null;
  sales_7d?: number | null;
  sales_30d?: number | null;
};

type FilterValues = {
  keyword?: string;
};

type NumericFilters = {
  available?: number | null;
  sales_yesterday?: number | null;
  sales_7d?: number | null;
  sales_30d?: number | null;
};

type DetailTabKey =
  | 'warehouse'
  | 'sales_active'
  | 'inbound_apply'
  | 'outbound_apply'
  | 'purchase_plan'
  | 'purchase_in_transit'
  | 'return_in_transit'
  | 'transfer_in_transit'
  | 'return_sale_in_transit'
  | 'audit_docs'
  | 'return_reserve'
  | 'assembly'
  | 'batch'
  | 'barcode';

const exportThreshold = 5000;

const detailTabs: { key: DetailTabKey; label: string }[] = [
  { key: 'warehouse', label: '仓库' },
  { key: 'sales_active', label: '执行中的销售' },
  { key: 'inbound_apply', label: '入库申请' },
  { key: 'outbound_apply', label: '出库申请' },
  { key: 'purchase_plan', label: '采购计划' },
  { key: 'purchase_in_transit', label: '采购在途' },
  { key: 'return_in_transit', label: '退货在途' },
  { key: 'transfer_in_transit', label: '调拨在途' },
  { key: 'return_sale_in_transit', label: '退货在途（卖货）' },
  { key: 'audit_docs', label: '审核中单据' },
  { key: 'return_reserve', label: '退货预留' },
  { key: 'assembly', label: '组装库存' },
  { key: 'batch', label: '批次' },
  { key: 'barcode', label: '辅助条码' }
];

const specModeOptions = [
  { value: 'SKU', label: '规格模式(SKU)' },
  { value: 'SPU', label: '规格模式(货品)' }
];

const inventoryTreeData: DataNode[] = [
  { key: 'all', title: '全部' },
  { key: 'category', title: '货品分类' },
  { key: 'brand', title: '品牌' },
  { key: 'type', title: '货品类型' },
  { key: 'supplier_quote', title: '供应商（报价）（只展示前…）' },
  { key: 'supplier_default', title: '默认供应商（只展示前50条…）' },
  { key: 'tag', title: '货品标记' },
  { key: 'inventory_level', title: '库存量' },
  { key: 'other', title: '其他' },
  { key: 'manage_tag', title: '管理标签' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const resolveSalesValue = (value?: number | null) => (value == null ? '-' : value);

const normalizeSummaryRow = (row: InventorySummary): InventorySummaryRow => ({
  ...row,
  sku_name: row.sku_code ?? row.sku_id,
  spec: row.barcode ?? '-',
  brand: '-',
  sales_yesterday: 0,
  sales_7d: 0,
  sales_30d: 0
});

const applyNumericFilters = (rows: InventorySummaryRow[], filters: NumericFilters) =>
  rows.filter((row) => {
    if (filters.available != null && row.available !== filters.available) {
      return false;
    }
    if (filters.sales_yesterday != null && row.sales_yesterday !== filters.sales_yesterday) {
      return false;
    }
    if (filters.sales_7d != null && row.sales_7d !== filters.sales_7d) {
      return false;
    }
    if (filters.sales_30d != null && row.sales_30d !== filters.sales_30d) {
      return false;
    }
    return true;
  });

const buildColumnSettings = (columns: ColumnsType<InventorySummaryRow>) => {
  const keys = columns.map((column, index) => column.key?.toString() ?? `${index}`);
  return keys;
};

const resolveDetailRowKey = (record: Record<string, unknown>, index?: number) => {
  const key = record.id ?? record.order_id ?? record.orderId ?? record.reserve_no ?? record.reserveNo;
  if (key) {
    return String(key);
  }
  return String(index ?? 0);
};

const resolveOrderId = (record?: Record<string, unknown> | null) => {
  if (!record) {
    return null;
  }
  const raw = record.order_id ?? record.orderId;
  if (!raw) {
    return null;
  }
  return String(raw);
};

export const InventorySummaryPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const canExport = useCan('exports.write');
  const canRead = useCan('inventory.read');
  const canReadOrders = useCan('orders.read');
  const canWriteOrders = useCan('orders.write');
  const canWriteSkus = useCan('skus.write');
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [treeSelection, setTreeSelection] = useState<string[]>(['all']);
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [specMode, setSpecMode] = useState<string>('SKU');
  const [selected, setSelected] = useState<InventorySummaryRow | null>(null);
  const [activeTab, setActiveTab] = useState<DetailTabKey>('warehouse');
  const [numericFilters, setNumericFilters] = useState<NumericFilters>({});
  const [visibleColumnKeys, setVisibleColumnKeys] = useState<string[] | null>(null);
  const [exporting, setExporting] = useState(false);
  const [skuNoteModalOpen, setSkuNoteModalOpen] = useState(false);
  const [skuNoteInput, setSkuNoteInput] = useState('');
  const [skuNoteSubmitting, setSkuNoteSubmitting] = useState(false);
  const [orderNoteModalOpen, setOrderNoteModalOpen] = useState(false);
  const [orderNoteInput, setOrderNoteInput] = useState('');
  const [orderNoteSubmitting, setOrderNoteSubmitting] = useState(false);
  const [selectedDetailKey, setSelectedDetailKey] = useState<string | null>(null);
  const [activeOrderId, setActiveOrderId] = useState<string | null>(null);
  const [atpWarehouseId, setAtpWarehouseId] = useState<string | undefined>(undefined);
  const [atpRequestQty, setAtpRequestQty] = useState<number>(1);
  const [reconcileExpected, setReconcileExpected] = useState<number | null>(null);
  const [reconcileLoading, setReconcileLoading] = useState(false);
  const [reconcileResult, setReconcileResult] = useState<InventoryReconciliationResult | null>(
    null
  );

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'summary'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const warehouseOptions = useMemo(
    () =>
      (warehouseQuery.data?.items ?? []).map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })),
    [warehouseQuery.data?.items]
  );

  const searchQuery = useMemo(() => {
    const keyword = filters.keyword?.trim();
    return {
      keyword,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20,
      spec_mode: specMode,
      tree_key: treeSelection[0] ?? 'all'
    };
  }, [filters.keyword, pagination.current, pagination.pageSize, specMode, treeSelection]);

  const summaryQuery = useQuery({
    queryKey: ['inventory-summary', tenantId, searchQuery],
    queryFn: async (): Promise<InventorySummaryListResponse> => {
      const keyword = searchQuery.keyword;
      const isBarcode = Boolean(keyword && /^[0-9]{6,}$/.test(keyword));
      const response = await listInventorySummary(tenantId, {
        keyword: searchQuery.keyword,
        sku_code: keyword && !isBarcode ? keyword : undefined,
        barcode: keyword && isBarcode ? keyword : undefined,
        offset: searchQuery.offset,
        limit: searchQuery.limit,
        spec_mode: searchQuery.spec_mode,
        tree_key: searchQuery.tree_key
      });
      return response;
    },
    enabled: tenantReady && inventoryFeature.enabled,
    keepPreviousData: true,
    retry: false
  });

  useEffect(() => {
    if (atpWarehouseId) {
      return;
    }
    const defaultWarehouseId = warehouseQuery.data?.items?.[0]?.warehouse_id;
    if (defaultWarehouseId) {
      setAtpWarehouseId(defaultWarehouseId);
    }
  }, [atpWarehouseId, warehouseQuery.data?.items]);

  useEffect(() => {
    if (!selected) {
      setReconcileExpected(null);
      setReconcileResult(null);
      return;
    }
    setReconcileExpected(selected.on_hand ?? null);
    setReconcileResult(null);
  }, [selected]);

  const atpQuery = useQuery({
    queryKey: ['inventory-atp', tenantId, selected?.sku_id, atpWarehouseId, atpRequestQty],
    queryFn: () =>
      getInventoryAtp(tenantId, {
        warehouse_id: atpWarehouseId ?? '',
        sku_id: selected?.sku_id ?? '',
        request_qty: atpRequestQty
      }),
    enabled:
      tenantReady &&
      inventoryFeature.enabled &&
      Boolean(selected?.sku_id) &&
      Boolean(atpWarehouseId),
    retry: false
  });

  const summaryRows = useMemo(() => {
    const rows = (summaryQuery.data?.items ?? []).map(normalizeSummaryRow);
    return applyNumericFilters(rows, numericFilters);
  }, [summaryQuery.data?.items, numericFilters]);

  useEffect(() => {
    if (!selected) {
      return;
    }
    const exists = summaryRows.some((row) => row.sku_id === selected.sku_id);
    if (!exists) {
      setSelected(null);
    }
  }, [selected, summaryRows]);

  const mainColumns = useMemo<ColumnsType<InventorySummaryRow>>(
    () => [
      { title: '货品编号', dataIndex: 'sku_code', key: 'sku_code' },
      { title: '货品名称', dataIndex: 'sku_name', key: 'sku_name' },
      {
        title: '可用库存',
        dataIndex: 'available',
        key: 'available',
        align: 'right'
      },
      {
        title: '昨天销量',
        dataIndex: 'sales_yesterday',
        key: 'sales_yesterday',
        align: 'right',
        render: (value) => resolveSalesValue(value)
      },
      {
        title: '近7天销量',
        dataIndex: 'sales_7d',
        key: 'sales_7d',
        align: 'right',
        render: (value) => resolveSalesValue(value)
      },
      {
        title: '近30天销量',
        dataIndex: 'sales_30d',
        key: 'sales_30d',
        align: 'right',
        render: (value) => resolveSalesValue(value)
      },
      { title: '规格', dataIndex: 'spec', key: 'spec' },
      { title: '品牌', dataIndex: 'brand', key: 'brand' }
    ],
    []
  );

  const columnKeys = useMemo(() => buildColumnSettings(mainColumns), [mainColumns]);
  const resolvedVisibleKeys = visibleColumnKeys ?? columnKeys;
  const visibleColumns = useMemo(
    () =>
      mainColumns.filter((column, index) =>
        resolvedVisibleKeys.includes(column.key?.toString() ?? `${index}`)
      ),
    [mainColumns, resolvedVisibleKeys]
  );

  const summaryFields = useMemo(
    () => [
      { key: 'available', label: '可用库存', field: 'available' },
      { key: 'sales_yesterday', label: '昨日销量', field: 'sales_yesterday' },
      { key: 'sales_7d', label: '近7天销量', field: 'sales_7d' },
      { key: 'sales_30d', label: '近30天销量', field: 'sales_30d' }
    ],
    []
  );


  useEffect(() => {
    if (!columnKeys.length) {
      return;
    }
    if (visibleColumnKeys && visibleColumnKeys.length > 0) {
      return;
    }
    const stored = localStorage.getItem('inventory.summary.table.columns');
    if (stored) {
      try {
        const parsed = JSON.parse(stored) as string[];
        if (Array.isArray(parsed) && parsed.length > 0) {
          setVisibleColumnKeys(parsed.filter((key) => columnKeys.includes(key)));
          return;
        }
      } catch {
        setVisibleColumnKeys(columnKeys);
        return;
      }
    }
    setVisibleColumnKeys(columnKeys);
  }, [columnKeys, visibleColumnKeys]);

  useEffect(() => {
    if (!visibleColumnKeys || visibleColumnKeys.length === 0) {
      return;
    }
    localStorage.setItem(
      'inventory.summary.table.columns',
      JSON.stringify(visibleColumnKeys)
    );
  }, [visibleColumnKeys]);

  const columnSettingContent = (
    <Checkbox.Group
      value={resolvedVisibleKeys}
      onChange={(values) => {
        if (values.length === 0) {
          return;
        }
        setVisibleColumnKeys(values as string[]);
      }}
    >
      <Space direction="vertical" size={8}>
        {mainColumns.map((column, index) => {
          const columnKey = column.key?.toString() ?? `${index}`;
          return <Checkbox key={columnKey} value={columnKey}>{column.title}</Checkbox>;
        })}
      </Space>
    </Checkbox.Group>
  );

  const detailQuery = useQuery({
    queryKey: ['inventory-summary-detail', tenantId, selected?.sku_id, activeTab],
    queryFn: async () => {
      if (!selected?.sku_id) {
        return { items: [], total: 0 };
      }
      if (activeTab === 'warehouse') {
        return listInventory(tenantId, {
          sku_id: selected.sku_id,
          offset: 0,
          limit: 200
        });
      }
      return { items: [], total: 0 };
    },
    enabled: tenantReady && inventoryFeature.enabled && Boolean(selected?.sku_id),
    keepPreviousData: true,
    retry: false
  });

  const skuDetailQuery = useQuery({
    queryKey: ['inventory-summary-sku-detail', tenantId, selected?.sku_id],
    queryFn: () => getSkuDetail(tenantId, selected?.sku_id ?? ''),
    enabled: tenantReady && inventoryFeature.enabled && Boolean(selected?.sku_id),
    retry: false
  });

  const orderNoteQuery = useQuery({
    queryKey: ['inventory-summary-order-detail', tenantId, activeOrderId],
    queryFn: () => getOrderDetail(tenantId, activeOrderId ?? ''),
    enabled:
      tenantReady && inventoryFeature.enabled && Boolean(activeOrderId) && canReadOrders,
    retry: false
  });

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setSelected(null);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setSelected(null);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleTreeSelect = (keys: React.Key[]) => {
    const key = String(keys[0] ?? 'all');
    setTreeSelection([key]);
    setSelected(null);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  useEffect(() => {
    setSelectedDetailKey(null);
    setActiveOrderId(null);
  }, [activeTab, selected?.sku_id]);

  const handleRefresh = () => {
    summaryQuery.refetch();
    detailQuery.refetch();
    skuDetailQuery.refetch();
    warehouseQuery.refetch();
    atpQuery.refetch();
    if (activeOrderId) {
      orderNoteQuery.refetch();
    }
  };

  const handleCheckReconciliation = async () => {
    if (!tenantId || !selected?.sku_id || !atpWarehouseId) {
      message.warning('Please select SKU and warehouse first');
      return;
    }
    if (reconcileExpected == null) {
      message.warning('Please input expected on-hand qty');
      return;
    }
    setReconcileLoading(true);
    try {
      const result = await checkInventoryReconciliation(tenantId, {
        warehouse_id: atpWarehouseId,
        sku_id: selected.sku_id,
        expected_on_hand: reconcileExpected,
        source_ref: `INV-SUMMARY-${dayjs().format('YYYYMMDDHHmmss')}`
      });
      setReconcileResult(result);
      if (result.status === 'MATCH') {
        message.success('Reconciliation passed');
      } else {
        message.warning('Reconciliation found mismatch');
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'Failed to run reconciliation');
    } finally {
      setReconcileLoading(false);
    }
  };

  const handleExport = async () => {
    if (!canExport) {
      message.warning('暂无导出权限');
      return;
    }
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    setExporting(true);
    try {
      const total = summaryQuery.data?.total ?? 0;
      const job = await createExportJob({
        type: 'INVENTORY_EXPORT',
        params: {
          keyword: searchQuery.keyword,
          spec_mode: searchQuery.spec_mode,
          tree_key: searchQuery.tree_key
        }
      });
      if (total > exportThreshold) {
        message.success(`导出任务已创建：${job.job_id}，请到导出中心下载`);
      } else {
        message.success(`导出任务已提交：${job.job_id}`);
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '导出失败');
    } finally {
      setExporting(false);
    }
  };

  const handleOpenSkuNoteModal = () => {
    if (!selected) {
      message.warning('请先选择货品');
      return;
    }
    setSkuNoteInput(
      (skuDetailQuery.data?.sku as { note?: string | null } | undefined)?.note ?? ''
    );
    setSkuNoteModalOpen(true);
  };

  const handleSubmitSkuNote = async () => {
    if (!tenantId || !selected) {
      message.warning('请先选择货品');
      return;
    }
    if (!canWriteSkus) {
      message.warning('暂无货品编辑权限');
      return;
    }
    const note = skuNoteInput.trim();
    if (!note) {
      message.warning('请输入备注内容');
      return;
    }
    setSkuNoteSubmitting(true);
    try {
      await updateSku(tenantId, selected.sku_id, { note });
      message.success('货品备注已更新');
      setSkuNoteModalOpen(false);
      skuDetailQuery.refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '货品备注更新失败');
    } finally {
      setSkuNoteSubmitting(false);
    }
  };

  const handleOpenOrderNoteModal = () => {
    if (!activeOrderId) {
      message.warning('请先选择订单明细');
      return;
    }
    setOrderNoteInput('');
    setOrderNoteModalOpen(true);
  };

  const handleSubmitOrderNote = async () => {
    if (!tenantId || !activeOrderId) {
      message.warning('请先选择订单明细');
      return;
    }
    if (!canWriteOrders) {
      message.warning('暂无订单备注权限');
      return;
    }
    const note = orderNoteInput.trim();
    if (!note) {
      message.warning('请输入备注内容');
      return;
    }
    setOrderNoteSubmitting(true);
    try {
      await addOrderNote(tenantId, activeOrderId, note);
      message.success('订单备注已更新');
      setOrderNoteModalOpen(false);
      orderNoteQuery.refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '订单备注更新失败');
    } finally {
      setOrderNoteSubmitting(false);
    }
  };

  const handleDetailRowClick = (record: Record<string, unknown>, index?: number) => {
    setSelectedDetailKey(resolveDetailRowKey(record, index));
    setActiveOrderId(resolveOrderId(record));
  };

  const queryCells = useMemo(
    () => ({
      available: (
        <InputNumber
          placeholder="="
          size="small"
          value={numericFilters.available ?? undefined}
          onChange={(value) =>
            setNumericFilters((prev) => ({ ...prev, available: value ?? null }))
          }
        />
      ),
      sales_yesterday: (
        <InputNumber
          placeholder="="
          size="small"
          value={numericFilters.sales_yesterday ?? undefined}
          onChange={(value) =>
            setNumericFilters((prev) => ({ ...prev, sales_yesterday: value ?? null }))
          }
        />
      ),
      sales_7d: (
        <InputNumber
          placeholder="="
          size="small"
          value={numericFilters.sales_7d ?? undefined}
          onChange={(value) =>
            setNumericFilters((prev) => ({ ...prev, sales_7d: value ?? null }))
          }
        />
      ),
      sales_30d: (
        <InputNumber
          placeholder="="
          size="small"
          value={numericFilters.sales_30d ?? undefined}
          onChange={(value) =>
            setNumericFilters((prev) => ({ ...prev, sales_30d: value ?? null }))
          }
        />
      )
    }),
    [numericFilters]
  );

  const detailColumns = useMemo(() => {
    if (activeTab === 'warehouse') {
      return [
        {
          title: '仓库',
          dataIndex: 'warehouse_id',
          render: (value?: string | null) => warehouseMap.get(value ?? '') ?? value ?? '-'
        },
        { title: '可用', dataIndex: 'available' },
        { title: '预留', dataIndex: 'reserved' },
        { title: '在途', dataIndex: 'in_transit' },
        { title: '冻结', dataIndex: 'frozen', render: (value: number | null) => value ?? 0 }
      ];
    }
    return [
      { title: '仓库', dataIndex: 'warehouse_name' },
      { title: '预留对象', dataIndex: 'reserve_target' },
      { title: '预留类型', dataIndex: 'reserve_type' },
      { title: '预留单号', dataIndex: 'reserve_no' },
      { title: '预留原因', dataIndex: 'reserve_reason' },
      { title: '到期释放', dataIndex: 'release_at' },
      { title: '开始时间', dataIndex: 'start_at' },
      { title: '截止时间', dataIndex: 'end_at' },
      { title: '可用数量', dataIndex: 'available_qty' }
    ];
  }, [activeTab, warehouseMap]);

  const skuDetail = skuDetailQuery.data?.sku;
  const skuTags = skuDetail?.tags ?? [];
  const skuImages = skuDetail?.image_urls ?? [];
  const skuNote =
    (skuDetail as { note?: string | null } | undefined)?.note ?? '';
  const orderNotes = orderNoteQuery.data?.notes ?? [];

  const renderSkuDetailContent = useCallback(
    (content: ReactNode) => {
      if (!selected) {
        return <Empty description="请先选择货品查看详情" />;
      }
      if (skuDetailQuery.isFetching) {
        return <Typography.Text type="secondary">货品信息加载中...</Typography.Text>;
      }
      if (skuDetailQuery.error) {
        return (
          <Space direction="vertical">
            <Typography.Text type="secondary">货品信息加载失败</Typography.Text>
            <Button size="small" onClick={() => skuDetailQuery.refetch()}>
              重试
            </Button>
          </Space>
        );
      }
      return content;
    },
    [selected, skuDetailQuery]
  );

  const orderNoteContent = useCallback(() => {
    if (!activeOrderId) {
      return <Typography.Text type="secondary">请选择订单明细查看备注</Typography.Text>;
    }
    if (!canReadOrders) {
      return <Typography.Text type="secondary">暂无查看订单备注权限</Typography.Text>;
    }
    if (orderNoteQuery.isFetching) {
      return <Typography.Text type="secondary">订单备注加载中...</Typography.Text>;
    }
    if (orderNoteQuery.error) {
      return (
        <Space direction="vertical">
          <Typography.Text type="secondary">订单备注加载失败</Typography.Text>
          <Button size="small" onClick={() => orderNoteQuery.refetch()}>
            重试
          </Button>
        </Space>
      );
    }
    if (!orderNotes.length) {
      return <Typography.Text type="secondary">暂无备注</Typography.Text>;
    }
    return (
      <Space direction="vertical" size={4} className="inventory-summary-detail-notes">
        {orderNotes.map((note) => (
          <div key={note.note_id}>
            <div>{note.note}</div>
            <Typography.Text type="secondary">
              {formatDateTime(note.created_at)}
            </Typography.Text>
          </div>
        ))}
      </Space>
    );
  }, [activeOrderId, canReadOrders, orderNoteQuery, orderNotes]);

  const filterPanel = (
    <Card size="small">
      <Space direction="vertical" size={8} className="inventory-summary-tree-panel">
        <Typography.Text strong>筛选</Typography.Text>
        <Tree
          blockNode
          defaultExpandAll
          selectedKeys={treeSelection}
          onSelect={handleTreeSelect}
          treeData={inventoryTreeData}
        />
      </Space>
    </Card>
  );

  const detailGroups = useMemo(
    () => [
      {
        key: 'tags',
        label: '标记',
        children: renderSkuDetailContent(
          <Space direction="vertical" size={8}>
            <div className="inventory-summary-detail-tags">
              <span>货品标记</span>
              {skuTags.length ? (
                skuTags.map((tag) => <Tag key={tag}>{tag}</Tag>)
              ) : (
                <Typography.Text type="secondary">暂无</Typography.Text>
              )}
              <Tooltip title="功能建设中">
                <Button size="small" type="text" icon={<PlusOutlined />} aria-label="Add product tag" />
              </Tooltip>
            </div>
            <div className="inventory-summary-detail-tags">
              <span>规格标记</span>
              <Tag>SKU</Tag>
              <Tooltip title="功能建设中">
                <Button size="small" type="text" icon={<PlusOutlined />} aria-label="Add spec tag" />
              </Tooltip>
            </div>
          </Space>
        )
      },
      {
        key: 'common',
        label: '快捷功能',
        children: (
          <Space size={8} wrap>
            <Button size="small" icon={<ReloadOutlined />} onClick={handleRefresh}>
              刷新 F5
            </Button>
            <Tooltip title={!canRead ? '无权限' : undefined}>
              <Button
                size="small"
                disabled={!canRead}
                onClick={() => navigate('/inventory/ledgers')}
              >
                库存变动日志
              </Button>
            </Tooltip>
            <Button size="small" onClick={() => navigate('/products')}>
              货品管理
            </Button>
          </Space>
        )
      },
      {
        key: 'governance',
        label: 'Inventory Governance',
        children: (
          <Space
            direction="vertical"
            size={12}
            style={{ width: '100%' }}
            data-testid="inventory-governance-panel"
          >
            {!selected ? (
              <Typography.Text type="secondary" data-testid="inventory-governance-empty">
                Select one SKU from table to run ATP and reconciliation
              </Typography.Text>
            ) : (
              <>
                <Space wrap>
                  <Select
                    data-testid="inventory-governance-warehouse-select"
                    value={atpWarehouseId}
                    options={warehouseOptions}
                    onChange={setAtpWarehouseId}
                    style={{ width: 220 }}
                    placeholder="Select warehouse"
                  />
                  <InputNumber
                    data-testid="inventory-governance-request-qty"
                    value={atpRequestQty}
                    min={1}
                    onChange={(value) => setAtpRequestQty(value ?? 1)}
                    placeholder="Request qty"
                  />
                  <Button
                    data-testid="inventory-governance-recalculate"
                    onClick={() => atpQuery.refetch()}
                    loading={atpQuery.isFetching}
                  >
                    Recalculate ATP
                  </Button>
                </Space>
                {atpQuery.error ? (
                  <Typography.Text type="danger">
                    {atpQuery.error instanceof Error
                      ? atpQuery.error.message
                      : 'ATP calculation failed'}
                  </Typography.Text>
                ) : null}
                {atpQuery.data ? (
                  <Descriptions size="small" column={2} data-testid="inventory-governance-atp">
                    <Descriptions.Item label="Available">{atpQuery.data.available}</Descriptions.Item>
                    <Descriptions.Item label="ATP Qty">{atpQuery.data.atp_qty}</Descriptions.Item>
                    <Descriptions.Item label="Shortage">{atpQuery.data.shortage_qty}</Descriptions.Item>
                    <Descriptions.Item label="Alloc Batches">
                      {atpQuery.data.allocations.length}
                    </Descriptions.Item>
                  </Descriptions>
                ) : null}
                <Divider style={{ margin: '8px 0' }} />
                <Space wrap>
                  <InputNumber
                    data-testid="inventory-governance-expected-qty"
                    value={reconcileExpected ?? undefined}
                    onChange={(value) => setReconcileExpected(value ?? null)}
                    placeholder="Expected on-hand qty"
                  />
                  <Button
                    data-testid="inventory-governance-check"
                    loading={reconcileLoading}
                    onClick={handleCheckReconciliation}
                  >
                    Check Reconciliation
                  </Button>
                </Space>
                {reconcileResult ? (
                  <Descriptions
                    size="small"
                    column={2}
                    data-testid="inventory-governance-reconcile-result"
                  >
                    <Descriptions.Item label="Status">
                      <Tag
                        color={reconcileResult.status === 'MATCH' ? 'green' : 'red'}
                        data-testid="inventory-governance-status"
                      >
                        {reconcileResult.status}
                      </Tag>
                    </Descriptions.Item>
                    <Descriptions.Item label="Diff">{reconcileResult.diff_qty}</Descriptions.Item>
                    <Descriptions.Item label="Expected">
                      {reconcileResult.expected_on_hand}
                    </Descriptions.Item>
                    <Descriptions.Item label="Actual">{reconcileResult.actual_on_hand}</Descriptions.Item>
                  </Descriptions>
                ) : null}
              </>
            )}
          </Space>
        )
      },
      {
        key: 'detail',
        label: '详情',
        children: renderSkuDetailContent(
          <Descriptions size="small" column={1}>
            <Descriptions.Item label="货品编号">
              {selected?.sku_code ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="SKU ID">{selected?.sku_id ?? '-'}</Descriptions.Item>
            <Descriptions.Item label="货品名称">
              {skuDetail?.name ?? selected?.sku_name ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="规格">
              {skuDetail?.spec ? JSON.stringify(skuDetail.spec) : selected?.spec ?? '-'}
            </Descriptions.Item>
          </Descriptions>
        )
      },
      {
        key: 'sku-note',
        label: '货品备注',
        children: renderSkuDetailContent(
          <Space direction="vertical" size={8}>
            <Typography.Text type="secondary">
              {skuNote || '暂无备注'}
            </Typography.Text>
            <Button
              size="small"
              onClick={handleOpenSkuNoteModal}
              disabled={!canWriteSkus}
            >
              编辑备注
            </Button>
          </Space>
        )
      },
      {
        key: 'order-note',
        label: '订单备注',
        children: (
          <Space direction="vertical" size={8} style={{ width: '100%' }}>
            {orderNoteContent()}
            <Button
              size="small"
              onClick={handleOpenOrderNoteModal}
              disabled={!canWriteOrders || !activeOrderId}
            >
              新增备注
            </Button>
          </Space>
        )
      },
      {
        key: 'image',
        label: '图片',
        children: renderSkuDetailContent(
          skuImages.length ? (
            <Space direction="vertical" size={8}>
              {skuImages.map((url) => (
                <img key={url} src={url} alt="sku" className="inventory-summary-image" />
              ))}
            </Space>
          ) : (
            <Typography.Text type="secondary">暂无图片</Typography.Text>
          )
        )
      }
    ],
    [
      activeOrderId,
      canRead,
      canReadOrders,
      canWriteOrders,
      canWriteSkus,
      atpQuery,
      atpRequestQty,
      atpWarehouseId,
      handleRefresh,
      handleCheckReconciliation,
      navigate,
      orderNoteContent,
      reconcileExpected,
      reconcileLoading,
      reconcileResult,
      renderSkuDetailContent,
      selected?.sku_code,
      selected?.sku_id,
      selected?.sku_name,
      selected?.spec,
      skuDetail?.name,
      skuDetail?.spec,
      skuImages,
      skuNote,
      skuTags,
      warehouseOptions
    ]
  );

  const detailRows = useMemo(
    () => (detailQuery.data?.items ?? []) as Record<string, unknown>[],
    [detailQuery.data?.items]
  );

  const bottomTabs = useMemo(
    () => (
      <div className="inventory-summary-bottom-panel">
        <Tabs
          size="small"
          type="card"
          activeKey={activeTab}
          items={detailTabs}
          onChange={(key) => setActiveTab(key as DetailTabKey)}
        />
        {!selected ? (
          <EmptyState title="请先选择货品查看明细" />
        ) : (
          <UiModule03MainTableSection
            rowKey={resolveDetailRowKey}
            columns={detailColumns}
            data={detailRows}
            loading={detailQuery.isFetching}
            error={detailQuery.error}
            onRetry={() => detailQuery.refetch()}
            errorTitle="明细加载失败"
            emptyText="暂无明细"
            total={detailQuery.data?.total ?? 0}
            pagination={{ current: 1, pageSize: 10, total: detailQuery.data?.total ?? 0 }}
            onPaginationChange={() => {}}
            showColumnSettings={false}
            showQueryRow={false}
            rowClassName={(record, index) =>
              resolveDetailRowKey(record, index) === selectedDetailKey
                ? 'inventory-summary-row-active'
                : ''
            }
            onRow={(record, index) => ({
              onClick: () => handleDetailRowClick(record as Record<string, unknown>, index)
            })}
            enablePanels={false}
          />
        )}
      </div>
    ),
    [
      activeTab,
      detailColumns,
      detailQuery,
      detailRows,
      handleDetailRowClick,
      selected,
      selectedDetailKey
    ]
  );

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.key === 'F5') {
        event.preventDefault();
        handleRefresh();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [handleRefresh]);

  const renderActionButtons = (size: 'small' | 'middle' = 'small') => (
    <Space size={8} wrap>
      <Popover placement="bottomRight" trigger="click" content={columnSettingContent}>
        <Button size={size} icon={<SettingOutlined />}>
          设置
        </Button>
      </Popover>
      <Button
        size={size}
        icon={<ExportOutlined />}
        onClick={handleExport}
        loading={exporting}
        disabled={!canExport}
      >
        导出（大于5000行）
      </Button>
      <Button
        size={size}
        icon={<ShareAltOutlined />}
        onClick={() => message.info('分享链接功能建设中')}
      >
        分享
      </Button>
      <Select
        size={size}
        value={specMode}
        options={specModeOptions}
        onChange={(value) => {
          setSpecMode(value);
          setPagination((prev) => ({ ...prev, current: 1 }));
        }}
        style={{ width: 150 }}
      />
      <Button
        size={size}
        icon={<QuestionCircleOutlined />}
        onClick={() => message.info('查看帮助文档')}
      >
        帮助
      </Button>
    </Space>
  );

  const queryPanel = (
    <Card size="small">
      <Form form={form} layout="vertical" onFinish={handleSearch}>
        <Row gutter={12}>
          <Col xs={24}>
            <Form.Item name="keyword" label="关键词">
              <Input
                placeholder="编号/名称/条码（含辅助条码）"
                allowClear
                data-testid="inventory-summary-search"
              />
            </Form.Item>
          </Col>
          <Col xs={24}>
            <Space>
              <Button type="primary" htmlType="submit" size="small">
                查询
              </Button>
              <Button onClick={handleReset} size="small">
                重置
              </Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
  );

  const tableContent = (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      {!inventoryFeature.enabled ? (
        <Alert
          type="warning"
          showIcon
          message="库存管理已关闭"
          description="库存查询已暂停，如需使用请先在设置中开启。"
        />
      ) : null}
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
      <div className="inventory-summary-table">
        <UiModule03MainTableSection<InventorySummaryRow>
          dataTestId="inventory-summary-table"
          rowKey={(record) => record.sku_id}
          columns={visibleColumns}
          data={summaryRows}
          loading={summaryQuery.isFetching && summaryRows.length === 0}
          error={summaryQuery.error}
          onRetry={() => summaryQuery.refetch()}
          errorTitle="库存概况加载失败"
          emptyText="暂无库存"
          total={summaryQuery.data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          showColumnSettings={false}
          queryCells={queryCells}
          summaryFields={summaryFields}
          rowClassName={(record) =>
            record.sku_id === selected?.sku_id ? 'inventory-summary-row-active' : ''
          }
          onRow={(record) => ({
            onClick: () => setSelected(record)
          })}
          enablePanels={false}
        />
      </div>
    </Space>
  );

  return (
    <>
      <WorkbenchListPage
        className="inventory-summary-page"
        filter={filterPanel}
        query={queryPanel}
        toolbarLeft={
          <Typography.Title level={4} data-testid="inventory-summary-title" style={{ margin: 0 }}>
            总库存查询
          </Typography.Title>
        }
        toolbarRight={renderActionButtons('small')}
        table={tableContent}
        detailGroups={detailGroups}
        bottomTabs={bottomTabs}
      />
      <Modal
        title="编辑货品备注"
        open={skuNoteModalOpen}
        onCancel={() => setSkuNoteModalOpen(false)}
        onOk={handleSubmitSkuNote}
        okButtonProps={{ disabled: !canWriteSkus }}
        confirmLoading={skuNoteSubmitting}
      >
        <Form layout="vertical">
          <Form.Item label="备注内容">
            <Input.TextArea
              rows={3}
              value={skuNoteInput}
              onChange={(event) => setSkuNoteInput(event.target.value)}
              placeholder="请输入货品备注"
            />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="新增订单备注"
        open={orderNoteModalOpen}
        onCancel={() => setOrderNoteModalOpen(false)}
        onOk={handleSubmitOrderNote}
        okButtonProps={{ disabled: !canWriteOrders }}
        confirmLoading={orderNoteSubmitting}
      >
        <Form layout="vertical">
          <Form.Item label="备注内容">
            <Input.TextArea
              rows={3}
              value={orderNoteInput}
              onChange={(event) => setOrderNoteInput(event.target.value)}
              placeholder="请输入订单备注"
            />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};

InventorySummaryPage.displayName = 'InventorySummaryPage';
