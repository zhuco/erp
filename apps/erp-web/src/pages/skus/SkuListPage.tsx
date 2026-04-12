import {
  Alert,
  Button,
  Card,
  Col,
  Descriptions,
  Drawer,
  Dropdown,
  Form,
  Image,
  Input,
  InputNumber,
  Modal,
  Progress,
  Row,
  Select,
  Space,
  Spin,
  Switch,
  Table,
  Tag,
  Tooltip,
  Typography,
  Upload,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { UploadFile } from 'antd/es/upload/interface';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';

import { listProducts, type Product } from '@/api/products';
import { listSuppliers } from '@/api/suppliers';
import { useSkusList } from '@/api/hooks';
import type {
  Sku,
  SkuCreatePayload,
  SkuImportDuplicateStrategy,
  SkuImportFailureRow,
  SkuImportJobResponse,
  SkuKind,
  SkuUpdatePayload
} from '@/api/skus';
import {
  copySku,
  createSku,
  createSkuImportJob,
  deleteSku,
  downloadSkuImportFailureLog,
  downloadSkuImportTemplate,
  getSkuDetail,
  getSkuImportJob,
  listSkuImportFailures,
  updateSku,
  uploadSkuImportFile
} from '@/api/skus';
import type { SkuDetailResponse } from '@/api/skus';
import { EmptyState } from '@/components/EmptyState';
import { UiModule03MainTableSection, UiModule03TableFooter } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  product_id?: string;
  keyword?: string;
  is_active?: boolean;
};

type SkuUnitForm = {
  name: string;
  ratio: number;
  barcode?: string;
  rounding?: string;
  is_base?: boolean;
};

type FormValues = {
  product_id?: string;
  sku_code: string;
  barcode?: string;
  name?: string;
  spec?: string;
  units?: SkuUnitForm[];
  tags?: string[];
  image_urls?: string[];
  supplier_ids?: string[];
  primary_supplier_id?: string;
  weight_gram?: number;
  length_mm?: number;
  width_mm?: number;
  height_mm?: number;
  volume_cm3?: number;
  sku_kind: SkuKind;
  cost?: number;
  is_active: boolean;
  batch_enabled: boolean;
  serial_enabled: boolean;
};

const skuKindOptions = [
  { value: 'NORMAL', label: '普通' },
  { value: 'KIT', label: '组合装' }
];

const activeOptions = [
  { value: true, label: '启用' },
  { value: false, label: '停用' }
];

const unitRoundingOptions = [
  { value: 'NONE', label: '不取整' },
  { value: 'FLOOR', label: '向下取整' },
  { value: 'CEIL', label: '向上取整' },
  { value: 'ROUND', label: '四舍五入' }
];

const skuKindLabels: Record<SkuKind, string> = {
  NORMAL: '普通',
  KIT: '组合装'
};

const statusLabels: Record<string, string> = {
  ACTIVE: '启用',
  INACTIVE: '停用'
};

const duplicateStrategyOptions: Array<{ value: SkuImportDuplicateStrategy; label: string }> = [
  { value: 'skip', label: '跳过重复 SKU' },
  { value: 'overwrite', label: '覆盖已有 SKU' }
];

const importStatusLabels: Record<string, string> = {
  PENDING: '待处理',
  RUNNING: '导入中',
  SUCCESS: '成功',
  FAILED: '失败'
};

const isImportFinished = (status?: string | null) =>
  status === 'SUCCESS' || status === 'FAILED';

const formatCost = (costCent?: number | null) =>
  costCent == null ? '-' : `¥${(costCent / 100).toFixed(2)}`;

const formatDimensions = (sku: Sku) => {
  const dims = [sku.length_mm, sku.width_mm, sku.height_mm].filter(
    (value): value is number => value != null
  );
  if (!dims.length) {
    return '-';
  }
  return `${dims.join('x')} mm`;
};

const formatWeightVolume = (sku: Sku) => {
  const weight = sku.weight_gram != null ? `${sku.weight_gram} g` : '-';
  const volume = sku.volume_cm3 != null ? `${sku.volume_cm3} cm³` : '-';
  return (
    <Space direction="vertical" size={0}>
      <span>重量：{weight}</span>
      <span>体积：{volume}</span>
    </Space>
  );
};

const formatStatus = (sku: Sku) => {
  const status = sku.status ?? (sku.is_active ? 'ACTIVE' : 'INACTIVE');
  return status;
};

const parseJson = (value?: string) => {
  if (!value) {
    return undefined;
  }
  return JSON.parse(value);
};

const normalizeUnits = (units?: SkuUnitForm[]) => {
  if (!units || units.length === 0) {
    return [];
  }
  let cleaned = units
    .map((unit) => ({
      name: unit.name?.trim(),
      ratio: Number.isFinite(unit.ratio) ? Math.max(1, Math.round(unit.ratio)) : 1,
      barcode: unit.barcode?.trim() || undefined,
      rounding: unit.rounding?.trim() || undefined,
      is_base: unit.is_base ?? false
    }))
    .filter((unit) => unit.name);
  if (!cleaned.length) {
    return [];
  }
  let baseIndex = cleaned.findIndex((unit) => unit.is_base);
  if (baseIndex < 0) {
    baseIndex = 0;
  }
  cleaned = cleaned.map((unit, index) => ({
    ...unit,
    is_base: index === baseIndex,
    ratio: index === baseIndex ? 1 : unit.ratio
  }));
  return cleaned;
};

const splitSkuSpec = (spec?: Record<string, unknown> | null) => {
  if (!spec || typeof spec !== 'object') {
    return { units: [] as SkuUnitForm[], extraSpec: undefined as string | undefined };
  }
  const { units, ...rest } = spec as Record<string, unknown> & { units?: SkuUnitForm[] };
  const normalizedUnits = normalizeUnits(Array.isArray(units) ? units : undefined);
  const extraKeys = Object.keys(rest);
  return {
    units: normalizedUnits,
    extraSpec: extraKeys.length ? JSON.stringify(rest, null, 2) : undefined
  };
};

const mergeSkuSpec = (extraSpec: Record<string, unknown> | undefined, units?: SkuUnitForm[]) => {
  const normalizedUnits = normalizeUnits(units);
  const payload = extraSpec && typeof extraSpec === 'object' ? { ...extraSpec } : {};
  if (normalizedUnits.length) {
    payload.units = normalizedUnits;
  }
  return Object.keys(payload).length ? payload : undefined;
};

const formatUnitSummary = (units?: SkuUnitForm[]) => {
  if (!units || units.length === 0) {
    return '-';
  }
  return units
    .map((unit) => {
      const ratio = unit.is_base ? 1 : unit.ratio;
      return unit.is_base ? unit.name : `${unit.name}=${ratio}`;
    })
    .join(' / ');
};

const normalizeStringList = (value?: string[] | string | null) => {
  if (!value) {
    return [];
  }
  if (Array.isArray(value)) {
    return value.map((item) => item.trim()).filter(Boolean);
  }
  return value
    .split(/[,\\n]/)
    .map((item) => item.trim())
    .filter(Boolean);
};

const renderTagList = (tags?: string[] | null) => {
  if (!tags || tags.length === 0) {
    return '-';
  }
  return (
    <Space size={[4, 4]} wrap>
      {tags.map((tag) => (
        <Tag key={tag}>{tag}</Tag>
      ))}
    </Space>
  );
};

const renderImageCount = (imageUrls?: string[] | null) =>
  imageUrls && imageUrls.length > 0 ? `${imageUrls.length} 张` : '-';

export const SkuListPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const costFeature = useFeatureFlag('cost.enabled');
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
  const [editingSku, setEditingSku] = useState<Sku | null>(null);
  const [saving, setSaving] = useState(false);
  const [productOptions, setProductOptions] = useState<Product[]>([]);
  const [productLoading, setProductLoading] = useState(false);
  const [supplierOptions, setSupplierOptions] = useState<
    Array<{ value: string; label: string }>
  >([]);
  const [supplierLoading, setSupplierLoading] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [detailLoading, setDetailLoading] = useState(false);
  const [detailData, setDetailData] = useState<SkuDetailResponse | null>(null);
  const [importOpen, setImportOpen] = useState(false);
  const [importFileList, setImportFileList] = useState<UploadFile[]>([]);
  const [duplicateStrategy, setDuplicateStrategy] =
    useState<SkuImportDuplicateStrategy>('skip');
  const [importing, setImporting] = useState(false);
  const [importJob, setImportJob] = useState<SkuImportJobResponse | null>(null);
  const [failureDrawerOpen, setFailureDrawerOpen] = useState(false);
  const [failureRows, setFailureRows] = useState<SkuImportFailureRow[]>([]);
  const [failureLoading, setFailureLoading] = useState(false);
  const [failureTotal, setFailureTotal] = useState(0);
  const [failurePagination, setFailurePagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const tableContainerRef = useRef<HTMLDivElement | null>(null);
  const importStatusRef = useRef<string | null>(null);

  const currentPage = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  const cursor = cursorMap[currentPage];
  const importProcessing = importing || (importJob ? !isImportFinished(importJob.status) : false);
  const failurePage = failurePagination.current ?? 1;
  const failurePageSize = failurePagination.pageSize ?? 10;

  const { data, isLoading, isFetching, error, refetch } = useSkusList({
    product_id: filters.product_id,
    keyword: filters.keyword?.trim() || undefined,
    is_active: filters.is_active,
    pageToken: cursor,
    pageSize
  });
  const listItems = data?.data ?? [];

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

  useEffect(() => {
    if (!tenantId || !importJob || isImportFinished(importJob.status)) {
      return;
    }
    const timer = window.setInterval(async () => {
      try {
        const latest = await getSkuImportJob(tenantId, importJob.job_id);
        setImportJob(latest);
      } catch (err) {
        message.error(err instanceof Error ? err.message : '获取导入状态失败');
      }
    }, 2000);
    return () => window.clearInterval(timer);
  }, [importJob, tenantId]);

  useEffect(() => {
    if (!importJob) {
      importStatusRef.current = null;
      return;
    }
    if (importJob.status === importStatusRef.current) {
      return;
    }
    if (isImportFinished(importJob.status)) {
      if (importJob.failed_rows > 0) {
        message.warning(`导入完成，失败 ${importJob.failed_rows} 条`);
      } else {
        message.success('导入成功');
      }
      if (tenantId && importJob.success_rows > 0) {
        void queryClient.invalidateQueries({ queryKey: ['skus', tenantId] });
      }
    }
    importStatusRef.current = importJob.status;
  }, [importJob, queryClient, tenantId]);

  const total = useMemo(() => {
    if (!data) {
      return 0;
    }
    if (data.next_cursor) {
      return currentPage * pageSize + pageSize;
    }
    return (currentPage - 1) * pageSize + data.data.length;
  }, [currentPage, data, pageSize]);

  const productMap = useMemo(() => {
    const map = new Map<string, Product>();
    productOptions.forEach((product) => map.set(product.product_id, product));
    if (editingSku?.product_id && !map.has(editingSku.product_id)) {
      map.set(editingSku.product_id, {
        product_id: editingSku.product_id,
        tenant_id: tenantId ?? '',
        name: editingSku.product_id,
        brand: null,
        category: null,
        attrs: {},
        created_at: '',
        updated_at: ''
      });
    }
    return map;
  }, [editingSku?.product_id, productOptions, tenantId]);

  const productSelectOptions = useMemo(
    () =>
      Array.from(productMap.values()).map((product) => ({
        value: product.product_id,
        label: `${product.name}${product.brand ? ` / ${product.brand}` : ''}`
      })),
    [productMap]
  );

  const loadProductOptions = async (keyword?: string) => {
    if (!tenantId) {
      return;
    }
    setProductLoading(true);
    try {
      const response = await listProducts(tenantId, {
        keyword: keyword?.trim() || undefined,
        limit: 50
      });
      setProductOptions(response.data ?? []);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载商品失败');
    } finally {
      setProductLoading(false);
    }
  };

  const loadSupplierOptions = async (keyword?: string) => {
    if (!tenantId) {
      return;
    }
    setSupplierLoading(true);
    try {
      const response = await listSuppliers(tenantId, {
        keyword: keyword?.trim() || undefined,
        limit: 50
      });
      setSupplierOptions(
        response.items.map((supplier) => ({
          value: supplier.supplier_id,
          label: supplier.name
        }))
      );
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载供应商失败');
    } finally {
      setSupplierLoading(false);
    }
  };

  const mergeSupplierOptions = (suppliers: Array<{ supplier_id: string; name: string }>) => {
    if (!suppliers.length) {
      return;
    }
    setSupplierOptions((prev) => {
      const map = new Map(prev.map((option) => [option.value, option]));
      suppliers.forEach((supplier) => {
        if (!map.has(supplier.supplier_id)) {
          map.set(supplier.supplier_id, {
            value: supplier.supplier_id,
            label: supplier.name
          });
        }
      });
      return Array.from(map.values());
    });
  };

  const fetchSkuDetail = async (skuId: string) => {
    if (!tenantId) {
      throw new Error('请先选择租户');
    }
    const detail = await getSkuDetail(tenantId, skuId);
    mergeSupplierOptions(
      detail.suppliers.map((supplier) => ({
        supplier_id: supplier.supplier_id,
        name: supplier.name
      }))
    );
    return detail;
  };

  const openDetailDrawer = async (record: Sku) => {
    setDetailOpen(true);
    setDetailLoading(true);
    setDetailData(null);
    try {
      const detail = await fetchSkuDetail(record.sku_id);
      setDetailData(detail);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载货品详情失败');
    } finally {
      setDetailLoading(false);
    }
  };

  const openEditDrawer = async (record: Sku) => {
    setEditingSku(record);
    setDrawerOpen(true);
    const { units, extraSpec } = splitSkuSpec(record.spec ?? undefined);
    drawerForm.setFieldsValue({
      product_id: record.product_id,
      sku_code: record.sku_code,
      barcode: record.barcode ?? undefined,
      name: record.name ?? undefined,
      spec: extraSpec,
      units,
      tags: record.tags ?? [],
      image_urls: record.image_urls ?? [],
      weight_gram: record.weight_gram ?? undefined,
      length_mm: record.length_mm ?? undefined,
      width_mm: record.width_mm ?? undefined,
      height_mm: record.height_mm ?? undefined,
      volume_cm3: record.volume_cm3 ?? undefined,
      sku_kind: record.sku_kind ?? 'NORMAL',
      cost: record.cost_cent != null ? record.cost_cent / 100 : undefined,
      is_active: record.is_active,
      batch_enabled: record.batch_enabled ?? false,
      serial_enabled: record.serial_enabled ?? false
    });
    void loadProductOptions();
    void loadSupplierOptions();
    try {
      const detail = await fetchSkuDetail(record.sku_id);
      const supplierIds = detail.suppliers.map((supplier) => supplier.supplier_id);
      const primarySupplierId =
        detail.suppliers.find((supplier) => supplier.is_primary)?.supplier_id ?? undefined;
      drawerForm.setFieldsValue({
        tags: detail.sku.tags ?? [],
        image_urls: detail.sku.image_urls ?? [],
        supplier_ids: supplierIds,
        primary_supplier_id: primarySupplierId
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载供应商失败');
    }
  };

  const handleCopy = async (record: Sku) => {
    if (!tenantId) {
      message.error('请先选择租户');
      return;
    }
    try {
      await copySku(tenantId, record.sku_id);
      message.success('货品已复制');
      await queryClient.invalidateQueries({ queryKey: ['skus', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '复制失败');
    }
  };

  const handleDelete = (record: Sku) => {
    if (!tenantId) {
      message.error('请先选择租户');
      return;
    }
    Modal.confirm({
      title: '删除货品',
      content: `确认删除货品 ${record.sku_code}？`,
      okText: '删除',
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await deleteSku(tenantId, record.sku_id);
          message.success('货品已删除');
          await queryClient.invalidateQueries({ queryKey: ['skus', tenantId] });
        } catch (err) {
          message.error(err instanceof Error ? err.message : '删除失败');
        }
      }
    });
  };

  const columns = useMemo<ColumnsType<Sku>>(() => {
    const base: ColumnsType<Sku> = [
      {
        title: 'SKU编码',
        dataIndex: 'sku_code'
      },
      {
        title: '商品',
        dataIndex: 'product_id',
        render: (_, record) => productMap.get(record.product_id)?.name ?? record.product_id
      },
      {
        title: '条码',
        dataIndex: 'barcode',
        render: (_, record) => record.barcode ?? '-'
      },
      {
        title: '货品名称',
        dataIndex: 'name',
        render: (_, record) => record.name ?? '-'
      },
      {
        title: '标签',
        dataIndex: 'tags',
        render: (_, record) => renderTagList(record.tags ?? [])
      },
      {
        title: '图片',
        dataIndex: 'image_urls',
        render: (_, record) => renderImageCount(record.image_urls ?? [])
      },
      {
        title: '规格/单位',
        dataIndex: 'spec',
        render: (_, record) => {
          const { units } = splitSkuSpec(record.spec ?? undefined);
          return formatUnitSummary(units);
        }
      }
    ];

    if (costFeature.enabled) {
      base.push({
        title: '成本',
        dataIndex: 'cost_cent',
        align: 'right',
        render: (_, record) => formatCost(record.cost_cent ?? null)
      });
    }

    base.push(
      {
        title: '尺寸',
        key: 'dimensions',
        render: (_, record) => formatDimensions(record)
      },
      {
        title: '重量/体积',
        key: 'weight_volume',
        render: (_, record) => formatWeightVolume(record)
      },
      {
        title: '货品类型',
        dataIndex: 'sku_kind',
        render: (_, record) =>
          skuKindLabels[(record.sku_kind ?? 'NORMAL') as SkuKind] ?? '-'
      },
      {
        title: '批次管理',
        dataIndex: 'batch_enabled',
        render: (_, record) =>
          record.batch_enabled ? <Tag color="blue">启用</Tag> : '-'
      },
      {
        title: '序列号',
        dataIndex: 'serial_enabled',
        render: (_, record) =>
          record.serial_enabled ? <Tag color="purple">启用</Tag> : '-'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => {
          const status = formatStatus(record);
          return (
            <Tag color={status === 'ACTIVE' ? 'green' : 'red'}>
              {statusLabels[status] ?? status}
            </Tag>
          );
        }
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space wrap>
            <Button size="small" onClick={() => openDetailDrawer(record)}>
              查看
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => void openEditDrawer(record)}>
              编辑
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => void handleCopy(record)}>
              复制
            </Button>
            <Button danger size="small" disabled={!canWrite} onClick={() => handleDelete(record)}>
              删除
            </Button>
          </Space>
        )
      }
    );

    return base;
  }, [
    canWrite,
    costFeature.enabled,
    openDetailDrawer,
    openEditDrawer,
    handleCopy,
    handleDelete,
    productMap
  ]);

  const tableFooterSyncKey = useMemo(
    () => `${columns.length}-${listItems.length}`,
    [columns.length, listItems.length]
  );

  const failureColumns = useMemo<ColumnsType<SkuImportFailureRow>>(
    () => [
      { title: '行号', dataIndex: 'row', width: 80 },
      {
        title: '规格编号',
        dataIndex: 'sku_code',
        render: (value) => value ?? '-'
      },
      {
        title: '字段',
        dataIndex: 'field',
        render: (value) => value ?? '-'
      },
      { title: '失败原因', dataIndex: 'message' }
    ],
    []
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
    setEditingSku(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      sku_kind: 'NORMAL',
      is_active: true,
      batch_enabled: false,
      serial_enabled: false,
      tags: [],
      image_urls: [],
      units: [{ name: '', ratio: 1, rounding: 'NONE', is_base: true }],
      supplier_ids: [],
      primary_supplier_id: undefined
    });
    void loadProductOptions();
    void loadSupplierOptions();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const spec = mergeSkuSpec(parseJson(values.spec), values.units);
      const isActive = values.is_active ?? true;
      const status = isActive ? 'ACTIVE' : 'INACTIVE';
      const tags = normalizeStringList(values.tags);
      const imageUrls = normalizeStringList(values.image_urls);
      const supplierIds = normalizeStringList(values.supplier_ids);
      const primarySupplierId =
        values.primary_supplier_id && supplierIds.includes(values.primary_supplier_id)
          ? values.primary_supplier_id
          : null;
      const costCent = costFeature.enabled
        ? values.cost != null
          ? Math.round(values.cost * 100)
          : null
        : undefined;
      setSaving(true);
      if (editingSku) {
        const payload: SkuUpdatePayload = {
          sku_code: values.sku_code.trim(),
          barcode: values.barcode?.trim() || null,
          name: values.name?.trim() || null,
          spec: spec ?? null,
          tags,
          image_urls: imageUrls,
          supplier_ids: supplierIds,
          primary_supplier_id: primarySupplierId,
          weight_gram: values.weight_gram ?? null,
          length_mm: values.length_mm ?? null,
          width_mm: values.width_mm ?? null,
          height_mm: values.height_mm ?? null,
          volume_cm3: values.volume_cm3 ?? null,
          sku_kind: values.sku_kind,
          ...(costFeature.enabled ? { cost_cent: costCent } : {}),
          is_active: isActive,
          status,
          batch_enabled: values.batch_enabled,
          serial_enabled: values.serial_enabled
        };
        await updateSku(tenantId, editingSku.sku_id, payload);
        message.success('货品已更新');
      } else {
        if (!values.product_id) {
          message.error('请选择商品');
          setSaving(false);
          return;
        }
        const payload: SkuCreatePayload = {
          product_id: values.product_id,
          sku_code: values.sku_code.trim(),
          barcode: values.barcode?.trim() || null,
          name: values.name?.trim() || null,
          spec: spec ?? null,
          tags,
          image_urls: imageUrls,
          supplier_ids: supplierIds,
          primary_supplier_id: primarySupplierId,
          weight_gram: values.weight_gram ?? null,
          length_mm: values.length_mm ?? null,
          width_mm: values.width_mm ?? null,
          height_mm: values.height_mm ?? null,
          volume_cm3: values.volume_cm3 ?? null,
          sku_kind: values.sku_kind,
          ...(costFeature.enabled ? { cost_cent: costCent } : {}),
          is_active: isActive,
          status,
          batch_enabled: values.batch_enabled,
          serial_enabled: values.serial_enabled
        };
        await createSku(tenantId, payload);
        message.success('货品已创建');
      }
      setDrawerOpen(false);
      setEditingSku(null);
      await queryClient.invalidateQueries({ queryKey: ['skus', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const resetImportState = () => {
    setImportFileList([]);
    setDuplicateStrategy('skip');
    setImporting(false);
    setImportJob(null);
    setFailureDrawerOpen(false);
    setFailureRows([]);
    setFailureTotal(0);
    setFailurePagination({
      current: 1,
      pageSize: 10
    });
  };

  const openImportModal = () => {
    resetImportState();
    setImportOpen(true);
  };

  const closeImportModal = () => {
    setImportOpen(false);
    resetImportState();
  };

  const handleDownloadTemplate = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    try {
      const blob = await downloadSkuImportTemplate(tenantId);
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = '货品导入模板.xlsx';
      link.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '下载模板失败');
    }
  };

  const handleStartImport = async () => {
    if (!tenantId) {
      message.error('请先选择商家');
      return;
    }
    if (!importFileList.length || !importFileList[0]?.originFileObj) {
      message.error('请先选择导入文件');
      return;
    }
    if (importProcessing) {
      return;
    }
    setImporting(true);
    setImportJob(null);
    try {
      const file = importFileList[0].originFileObj as File;
      const uploadResult = await uploadSkuImportFile(tenantId, file);
      const job = await createSkuImportJob(tenantId, {
        file_id: uploadResult.file_id,
        file_name: uploadResult.file_name,
        file_type: uploadResult.file_type,
        duplicate_strategy: duplicateStrategy
      });
      setImportJob(job);
      message.success('导入任务已创建');
    } catch (err) {
      message.error(err instanceof Error ? err.message : '导入失败');
    } finally {
      setImporting(false);
    }
  };

  const loadFailureRows = async (page: number, size: number) => {
    if (!tenantId || !importJob) {
      return;
    }
    setFailureLoading(true);
    try {
      const response = await listSkuImportFailures(tenantId, importJob.job_id, {
        offset: (page - 1) * size,
        limit: size
      });
      setFailureRows(response.items);
      setFailureTotal(response.total);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '获取失败明细失败');
    } finally {
      setFailureLoading(false);
    }
  };

  const openFailureDrawer = () => {
    setFailureDrawerOpen(true);
    setFailurePagination((prev) => ({ ...prev, current: 1 }));
    void loadFailureRows(1, failurePageSize);
  };

  const handleDownloadFailureLog = async () => {
    if (!tenantId || !importJob) {
      message.error('请先选择商家');
      return;
    }
    try {
      const blob = await downloadSkuImportFailureLog(tenantId, importJob.job_id);
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `货品导入失败日志-${importJob.job_id}.csv`;
      link.click();
      URL.revokeObjectURL(url);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '下载失败日志失败');
    }
  };

  const tableFooter = (
    <UiModule03TableFooter
      tableContainerRef={tableContainerRef}
      syncKey={tableFooterSyncKey}
      summary={`共 ${total} 条`}
      total={total}
      pagination={pagination}
      onPaginationChange={setPagination}
    />
  );

  return (
    <>
      <Space direction="vertical" size={16} style={{ width: '100%' }}>
        <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
          <Typography.Title level={3} style={{ margin: 0 }} data-testid="skus-title">
            SKU 管理
          </Typography.Title>
          <Tooltip title={!canWrite ? '暂无权限' : undefined}>
            <span>
              <Dropdown.Button
                type="primary"
                onClick={openCreateDrawer}
                disabled={!canWrite}
                menu={{
                  items: [{ key: 'import', label: 'Excel 导入' }],
                  onClick: ({ key }) => {
                    if (key === 'import') {
                      openImportModal();
                    }
                  }
                }}
              >
                新增货品
              </Dropdown.Button>
            </span>
          </Tooltip>
        </Space>
        <Card>
          <Form form={form} layout="vertical" onFinish={handleSearch}>
            <Row gutter={16}>
              <Col xs={24} md={6}>
                <Form.Item name="product_id" label="商品">
                  <Select
                    allowClear
                    showSearch
                    placeholder="选择商品"
                    loading={productLoading}
                    options={productSelectOptions}
                    filterOption={false}
                    onSearch={(value) => loadProductOptions(value)}
                    onFocus={() => loadProductOptions()}
                  />
                </Form.Item>
              </Col>
              <Col xs={24} md={8}>
                <Form.Item name="keyword" label="关键字">
                  <Input allowClear placeholder="SKU/条码/名称/标签" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item name="is_active" label="状态">
                  <Select allowClear options={activeOptions} placeholder="全部" />
                </Form.Item>
              </Col>
              <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
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
        <UiModule03MainTableSection<Sku>
          tableContainerRef={tableContainerRef}
          rowKey="sku_id"
          columns={columns}
          data={listItems}
          loading={isLoading || isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="货品列表加载失败"
          emptyText="暂无货品"
          total={total}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="skus.table.columns"
          enablePanels={false}
          tableFooter={tableFooter}
          showStatusBar={false}
        />
      </Space>
      <Modal
        title="货品 Excel 导入"
        open={importOpen}
        onCancel={closeImportModal}
        footer={null}
        width={720}
        destroyOnClose
      >
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Alert
            type="info"
            showIcon
            message="模板填写要求"
            description={
              <div>
                <div>红色字体为必填字段；单/多规格统一模板。</div>
                <div>多规格导入必须填写“规格编号/规格编码”，且不可重复。</div>
                <div>分类编号/分类名称二选一填写，且必须为系统内已存在分类。</div>
              </div>
            }
          />
          <Card size="small">
            <Space direction="vertical" size={8}>
              <Typography.Text strong>1. 下载模板</Typography.Text>
              <Button onClick={handleDownloadTemplate} disabled={!canWrite}>
                下载模板
              </Button>
            </Space>
          </Card>
          <Card size="small">
            <Space direction="vertical" size={8} style={{ width: '100%' }}>
              <Typography.Text strong>2. 上传文件</Typography.Text>
              <Upload
                beforeUpload={() => false}
                fileList={importFileList}
                onChange={({ fileList: nextList }) => {
                  setImportFileList(nextList.slice(-1));
                  setImportJob(null);
                }}
                disabled={!canWrite || importProcessing}
                accept=".xls,.xlsx,.csv"
              >
                <Button disabled={!canWrite || importProcessing}>选择文件</Button>
              </Upload>
              <Typography.Text type="secondary">支持 Excel/CSV 文件</Typography.Text>
            </Space>
          </Card>
          <Card size="small">
            <Space direction="vertical" size={8} style={{ width: '100%' }}>
              <Typography.Text strong>3. 导入设置</Typography.Text>
              <Space>
                <Typography.Text>重复 SKU 处理</Typography.Text>
                <Select
                  value={duplicateStrategy}
                  options={duplicateStrategyOptions}
                  onChange={(value) => setDuplicateStrategy(value)}
                  disabled={!canWrite || importProcessing}
                  style={{ width: 220 }}
                />
              </Space>
            </Space>
          </Card>
          <Card size="small">
            <Space direction="vertical" size={8}>
              <Typography.Text strong>4. 开始导入</Typography.Text>
              <Button
                type="primary"
                onClick={handleStartImport}
                loading={importing}
                disabled={!canWrite || importProcessing || importFileList.length === 0}
              >
                开始导入
              </Button>
            </Space>
          </Card>
          {importJob ? (
            <Card size="small">
              <Space direction="vertical" size={8} style={{ width: '100%' }}>
                <Typography.Text strong>5. 导入结果</Typography.Text>
                <Progress
                  percent={importJob.progress}
                  status={
                    importJob.status === 'FAILED'
                      ? 'exception'
                      : isImportFinished(importJob.status)
                      ? 'success'
                      : 'active'
                  }
                />
                <Descriptions column={4} size="small">
                  <Descriptions.Item label="状态">
                    {importStatusLabels[importJob.status] ?? importJob.status}
                  </Descriptions.Item>
                  <Descriptions.Item label="总行数">{importJob.total_rows}</Descriptions.Item>
                  <Descriptions.Item label="成功数">{importJob.success_rows}</Descriptions.Item>
                  <Descriptions.Item label="失败数">{importJob.failed_rows}</Descriptions.Item>
                </Descriptions>
                {importJob.failed_rows > 0 ? (
                  <Space>
                    <Button onClick={openFailureDrawer}>查看失败明细</Button>
                    <Button onClick={handleDownloadFailureLog}>下载失败日志</Button>
                  </Space>
                ) : null}
              </Space>
            </Card>
          ) : null}
        </Space>
      </Modal>
      <Drawer
        title="导入失败明细"
        open={failureDrawerOpen}
        onClose={() => setFailureDrawerOpen(false)}
        width={720}
        destroyOnClose
      >
        <Table<SkuImportFailureRow>
          rowKey={(record) => `${record.row}-${record.field ?? 'NA'}`}
          columns={failureColumns}
          dataSource={failureRows}
          loading={failureLoading}
          pagination={{
            current: failurePage,
            pageSize: failurePageSize,
            total: failureTotal,
            showSizeChanger: true,
            onChange: (page, size) => {
              const nextSize = size ?? failurePageSize;
              setFailurePagination({ current: page, pageSize: nextSize });
              void loadFailureRows(page, nextSize);
            }
          }}
        />
      </Drawer>
      <Drawer
        title={editingSku ? '编辑货品' : '新增货品'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={560}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="product_id"
            label="商品"
            rules={[{ required: true, message: '请选择商品' }]}
          >
            <Select
              showSearch
              allowClear
              disabled={Boolean(editingSku)}
              placeholder="搜索商品"
              loading={productLoading}
              options={productSelectOptions}
              filterOption={false}
              onSearch={(value) => loadProductOptions(value)}
              onFocus={() => loadProductOptions()}
            />
          </Form.Item>
          <Form.Item
            name="sku_code"
            label="SKU编码"
            rules={[{ required: true, message: '请输入SKU编码' }]}
          >
            <Input placeholder="SKU编码" />
          </Form.Item>
          <Form.Item name="barcode" label="条码">
            <Input placeholder="条码" />
          </Form.Item>
          <Form.Item name="name" label="货品名称">
            <Input placeholder="货品名称" />
          </Form.Item>
          <Form.List name="units">
            {(fields, { add, remove }) => (
              <Space direction="vertical" size={12} style={{ width: '100%' }}>
                <Typography.Text strong>规格单位</Typography.Text>
                {fields.map((field) => (
                  <Space key={field.key} align="baseline" style={{ width: '100%' }} wrap>
                    <Form.Item
                      {...field}
                      label="单位"
                      name={[field.name, 'name']}
                      style={{ minWidth: 120 }}
                    >
                      <Input placeholder="单位名称" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="换算比"
                      name={[field.name, 'ratio']}
                      style={{ minWidth: 100 }}
                    >
                      <InputNumber min={1} precision={0} />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="条码"
                      name={[field.name, 'barcode']}
                      style={{ minWidth: 150 }}
                    >
                      <Input placeholder="条码" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="取整"
                      name={[field.name, 'rounding']}
                      style={{ minWidth: 140 }}
                    >
                      <Select allowClear placeholder="取整规则" options={unitRoundingOptions} />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      label="基准"
                      name={[field.name, 'is_base']}
                      valuePropName="checked"
                    >
                      <Switch checkedChildren="是" unCheckedChildren="否" />
                    </Form.Item>
                    <Button type="link" danger onClick={() => remove(field.name)}>
                      移除
                    </Button>
                  </Space>
                ))}
                <Button
                  type="dashed"
                  onClick={() =>
                    add({
                      name: '',
                      ratio: 1,
                      rounding: 'NONE',
                      is_base: fields.length === 0
                    })
                  }
                >
                  新增单位
                </Button>
              </Space>
            )}
          </Form.List>
          <Form.Item name="spec" label="扩展规格 JSON">
            <Input.TextArea placeholder='{"color":"red"}' autoSize={{ minRows: 2 }} />
          </Form.Item>
          <Form.Item name="tags" label="标签">
            <Select
              mode="tags"
              tokenSeparators={[',']}
              placeholder="输入标签"
              options={[]}
            />
          </Form.Item>
          <Form.Item name="image_urls" label="图片链接">
            <Select
              mode="tags"
              tokenSeparators={[',']}
              placeholder="https://example.com/image.png"
              options={[]}
            />
          </Form.Item>
          <Form.Item name="supplier_ids" label="供应商">
            <Select
              mode="multiple"
              allowClear
              showSearch
              placeholder="搜索供应商"
              loading={supplierLoading}
              options={supplierOptions}
              filterOption={false}
              onSearch={(value) => loadSupplierOptions(value)}
              onFocus={() => loadSupplierOptions()}
            />
          </Form.Item>
          <Form.Item name="primary_supplier_id" label="主供应商">
            <Select
              allowClear
              showSearch
              placeholder="选择主供应商"
              loading={supplierLoading}
              options={supplierOptions}
              filterOption={false}
              onSearch={(value) => loadSupplierOptions(value)}
              onFocus={() => loadSupplierOptions()}
            />
          </Form.Item>
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="weight_gram" label="重量(g)">
                <InputNumber min={0} precision={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="volume_cm3" label="体积(cm³)">
                <InputNumber min={0} precision={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="length_mm" label="长度(mm)">
                <InputNumber min={0} precision={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="width_mm" label="宽度(mm)">
                <InputNumber min={0} precision={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="height_mm" label="高度(mm)">
                <InputNumber min={0} precision={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
          {costFeature.enabled ? (
            <Form.Item name="cost" label="成本(元)">
              <InputNumber min={0} precision={2} style={{ width: '100%' }} />
            </Form.Item>
          ) : null}
          <Form.Item
            name="sku_kind"
            label="货品类型"
            rules={[{ required: true, message: '请选择货品类型' }]}
          >
            <Select options={skuKindOptions} />
          </Form.Item>
          <Form.Item name="batch_enabled" label="批次管理" valuePropName="checked">
            <Switch checkedChildren="启用" unCheckedChildren="停用" />
          </Form.Item>
          <Form.Item name="serial_enabled" label="序列号管理" valuePropName="checked">
            <Switch checkedChildren="启用" unCheckedChildren="停用" />
          </Form.Item>
          <Form.Item name="is_active" label="启用状态" valuePropName="checked">
            <Switch checkedChildren="启用" unCheckedChildren="停用" />
          </Form.Item>
        </Form>
      </Drawer>
      <Drawer
        title="货品详情"
        open={detailOpen}
        onClose={() => {
          setDetailOpen(false);
          setDetailData(null);
        }}
        width={640}
        destroyOnClose
      >
        {detailLoading ? (
          <div style={{ textAlign: 'center', padding: '48px 0' }}>
            <Spin />
          </div>
        ) : detailData ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions column={2} size="small" bordered>
              <Descriptions.Item label="SKU编码">{detailData.sku.sku_code}</Descriptions.Item>
              <Descriptions.Item label="货品名称">
                {detailData.sku.name ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="商品">
                {detailData.product?.name ?? detailData.sku.product_id}
              </Descriptions.Item>
              <Descriptions.Item label="条码">
                {detailData.sku.barcode ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="货品类型">
                {skuKindLabels[(detailData.sku.sku_kind ?? 'NORMAL') as SkuKind] ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="状态">
                {statusLabels[formatStatus(detailData.sku)] ?? formatStatus(detailData.sku)}
              </Descriptions.Item>
              <Descriptions.Item label="规格/单位">
                {formatUnitSummary(splitSkuSpec(detailData.sku.spec ?? undefined).units)}
              </Descriptions.Item>
              <Descriptions.Item label="尺寸">
                {formatDimensions(detailData.sku)}
              </Descriptions.Item>
              <Descriptions.Item label="重量/体积">
                {formatWeightVolume(detailData.sku)}
              </Descriptions.Item>
              {costFeature.enabled ? (
                <Descriptions.Item label="成本">
                  {formatCost(detailData.sku.cost_cent ?? null)}
                </Descriptions.Item>
              ) : null}
              <Descriptions.Item label="标签" span={2}>
                {renderTagList(detailData.sku.tags ?? [])}
              </Descriptions.Item>
              <Descriptions.Item label="供应商" span={2}>
                {detailData.suppliers.length ? (
                  <Space size={[4, 4]} wrap>
                    {detailData.suppliers.map((supplier) => (
                      <Tag key={supplier.supplier_id} color={supplier.is_primary ? 'blue' : 'default'}>
                        {supplier.name}
                      </Tag>
                    ))}
                  </Space>
                ) : (
                  '-'
                )}
              </Descriptions.Item>
            </Descriptions>
            <div>
              <Typography.Text strong>图片</Typography.Text>
              <div style={{ marginTop: 8 }}>
                {detailData.sku.image_urls && detailData.sku.image_urls.length ? (
                  <Image.PreviewGroup>
                    <Space wrap>
                      {detailData.sku.image_urls.map((url) => (
                        <Image key={url} src={url} width={72} height={72} />
                      ))}
                    </Space>
                  </Image.PreviewGroup>
                ) : (
                  <Typography.Text type="secondary">暂无图片</Typography.Text>
                )}
              </div>
            </div>
          </Space>
        ) : (
          <EmptyState description="暂无货品详情" />
        )}
      </Drawer>
    </>
  );
};
