import {
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
  Popconfirm,
  Row,
  Select,
  Space,
  Switch,
  Table,
  Tabs,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { InputRef, MenuProps, TablePaginationConfig } from 'antd';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useRef, useState, type ReactElement } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { createProduct, deleteProduct, listProducts, updateProduct } from '@/api/products';
import type { Product, ProductCreateRequest, ProductListQuery } from '@/api/products';
import { createProductBrand, listProductBrands, type ProductBrand } from '@/api/productBrands';
import {
  createProductCategory,
  listProductCategories,
  type ProductCategory,
} from '@/api/productCategories';
import { createProductUnit, listProductUnits, type ProductUnit } from '@/api/productUnits';
import {
  createProductWarehouse,
  deleteProductWarehouse,
  listProductWarehouses,
  updateProductWarehouse,
  type ProductWarehouse,
} from '@/api/productWarehouses';
import { listProductKits } from '@/api/kits';
import type { KitSummary } from '@/api/kits';
import { deleteSku, searchSkus } from '@/api/skus';
import type { SkuSearchItem } from '@/api/skus';
import { listSkuTagDefinitions } from '@/api/skuTagDefinitions';
import { listSuppliers } from '@/api/suppliers';
import { listWarehouses } from '@/api/warehouses';
import { listOperationLogs } from '@/api/system';
import type { OperationLog } from '@/api/system';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import {
  UiModule01QuickFiltersPanel,
  UiModule02ToolbarRight,
  UiModule03ContextMenu,
  useUiModule03MainTableInteractions,
  UiModule03MainTableSection,
  UiModule03TableFooter,
  UiModule04BottomTable,
  UiModule05DetailPanel,
} from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { resolveOperationActionLabel } from '@/utils/logLabels';
import { ProductApparelSpecComposer } from '@/pages/products/components/ProductApparelSpecComposer';
import { ProductIndustrySections } from '@/pages/products/components/ProductIndustrySections';
import { ProductIndustrySummary } from '@/pages/products/components/ProductIndustrySummary';
import {
  buildProductIndustryAttrs,
  getSpecNamePreview,
  industryProfileOptions,
  omitProductIndustryAttrs,
  readBooleanAttr,
  readNumberAttr,
  readProductAttrs,
  readStringAttr,
  type IndustryProfile,
  type ProductIndustryFormValues,
  type ProductSpecForm,
} from '@/pages/products/productIndustry';

import './productsPage.css';

type WarningStockMode = 'UNLIMITED' | 'FIXED' | 'DAYS';

type WarningStockEntry = {
  mode?: WarningStockMode;
  value?: number;
};

type WarningStockForm = {
  upper?: WarningStockEntry;
  lower?: WarningStockEntry;
};

type ProductFormValues = ProductIndustryFormValues & {
  product_code?: string;
  name?: string;
  alias?: string;
  brand?: string;
  category?: string;
  base_unit?: string;
  second_unit?: string;
  default_supplier_id?: string;
  default_warehouse_id?: string;
  primary_barcode?: string;
  product_tags?: string[];
  remark?: string;
  warning_stock?: WarningStockForm;
};

type ProductFilters = {
  keyword?: string;
  brand?: string;
  category?: string;
  status?: string;
};

type ProductQuickFilterId = 'all' | 'ACTIVE' | 'INACTIVE';

type CreateSelectState = {
  value: string;
  loading: boolean;
};

const defaultWarningStock: WarningStockForm = {
  upper: { mode: 'UNLIMITED' },
  lower: { mode: 'UNLIMITED' },
};

const warningModeOptions = [
  { value: 'UNLIMITED', label: '不限' },
  { value: 'FIXED', label: '固定值' },
  { value: 'DAYS', label: '按天出库量' },
];

const statusOptions = [
  { value: 'ACTIVE', label: '启用' },
  { value: 'INACTIVE', label: '停用' },
];

const defaultProductQuickFilterItems: Array<{
  id: ProductQuickFilterId;
  name: string;
}> = [
  { id: 'all', name: '全部货品' },
  { id: 'ACTIVE', name: '启用货品' },
  { id: 'INACTIVE', name: '停用货品' },
];

const defaultProductQuickFilterIds: ProductQuickFilterId[] = defaultProductQuickFilterItems.map(
  (item) => item.id,
);

const productQuickFilterSettingOptions = defaultProductQuickFilterItems.map((item) => ({
  label: item.name,
  value: item.id,
}));

const normalizeQuickFilterIds = (value: unknown): ProductQuickFilterId[] => {
  if (!Array.isArray(value)) {
    return defaultProductQuickFilterIds;
  }
  const allowed = new Set<ProductQuickFilterId>(defaultProductQuickFilterIds);
  const ids = Array.from(
    new Set(
      value.filter(
        (item): item is ProductQuickFilterId =>
          typeof item === 'string' && allowed.has(item as ProductQuickFilterId),
      ),
    ),
  );
  return ids.length ? ids : defaultProductQuickFilterIds;
};

const isProductInactive = (status?: string | null) => status !== 'ACTIVE';

const formatProductStatus = (status?: string | null) =>
  isProductInactive(status) ? '停用' : '启用';

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const normalizeTags = (tags?: string[]) => (tags ?? []).map((tag) => tag.trim()).filter(Boolean);

const normalizeWarningEntry = (entry?: WarningStockEntry): WarningStockEntry => {
  if (!entry?.mode) {
    return { mode: 'UNLIMITED' };
  }
  if (entry.mode === 'UNLIMITED') {
    return { mode: 'UNLIMITED' };
  }
  const value = Number(entry.value ?? 0);
  return { mode: entry.mode, value: Number.isFinite(value) ? value : 0 };
};

const normalizeWarningStock = (warning?: WarningStockForm): WarningStockForm => ({
  upper: normalizeWarningEntry(warning?.upper),
  lower: normalizeWarningEntry(warning?.lower),
});

const formatWarningStock = (warning?: WarningStockForm | Record<string, unknown>) => {
  if (!warning) {
    return '-';
  }
  const upper = (warning as WarningStockForm).upper;
  const lower = (warning as WarningStockForm).lower;
  const formatEntry = (label: string, entry?: WarningStockEntry) => {
    if (!entry?.mode || entry.mode === 'UNLIMITED') {
      return `${label}：不限`;
    }
    if (entry.mode === 'DAYS') {
      return `${label}：${entry.value ?? 0}天出库量`;
    }
    return `${label}：${entry.value ?? 0}`;
  };
  return (
    <Space direction="vertical" size={0}>
      <span>{formatEntry('上限', upper)}</span>
      <span>{formatEntry('下限', lower)}</span>
    </Space>
  );
};

const normalizeSpecs = (specs?: ProductSpecForm[]) =>
  (specs ?? [])
    .map((spec) => ({
      spec_code: spec.spec_code?.trim() || undefined,
      spec_name: spec.spec_name?.trim() || undefined,
      cost: spec.cost,
      color: spec.color?.trim() || undefined,
      size: spec.size?.trim() || undefined,
      ingredient: spec.ingredient?.trim() || undefined,
      barcode: spec.barcode?.trim() || undefined,
      weight: spec.weight,
      unit: spec.unit?.trim() || undefined,
      remark: spec.remark?.trim() || undefined,
      tag: spec.tag?.trim() || undefined,
      second_unit: spec.second_unit?.trim() || undefined,
    }))
    .filter((spec) => spec.spec_code || spec.spec_name);

const buildSkuLabel = (sku: SkuSearchItem) => {
  const name = sku.name ? ` ${sku.name}` : '';
  const barcode = sku.barcode ? ` (${sku.barcode})` : '';
  return `${sku.sku_code}${name}${barcode}`;
};

export const ProductsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const { setLeftPanel, setRightPanel, setLeftPanelVisible, setRightPanelVisible, clearPanels } =
    useWorkbenchPanels();
  const [form] = Form.useForm<ProductFormValues>();
  const [filters, setFilters] = useState<ProductFilters>({});
  const [debouncedFilters, setDebouncedFilters] = useState<ProductFilters>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 50,
  });
  const [cursorMap, setCursorMap] = useState<Record<number, string | undefined>>({});
  const [selectedProduct, setSelectedProduct] = useState<Product | null>(null);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    product: Product;
  } | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [editingProduct, setEditingProduct] = useState<Product | null>(null);
  const [saving, setSaving] = useState(false);
  const [activeTab, setActiveTab] = useState('basic');
  const [activeBottomTab, setActiveBottomTab] = useState('warehouses');
  const [createCategory, setCreateCategory] = useState<CreateSelectState>({
    value: '',
    loading: false,
  });
  const [createBrand, setCreateBrand] = useState<CreateSelectState>({
    value: '',
    loading: false,
  });
  const [createUnit, setCreateUnit] = useState<CreateSelectState>({
    value: '',
    loading: false,
  });
  const [specTagOptions, setSpecTagOptions] = useState<string[]>([]);
  const [quickFilterVisibleIds, setQuickFilterVisibleIds] = useState<ProductQuickFilterId[]>(
    defaultProductQuickFilterIds,
  );
  const [quickFilterSettingDraft, setQuickFilterSettingDraft] = useState<ProductQuickFilterId[]>(
    defaultProductQuickFilterIds,
  );
  const [quickFilterSettingsOpen, setQuickFilterSettingsOpen] = useState(false);
  const [helpModalOpen, setHelpModalOpen] = useState(false);
  const attrsSnapshotRef = useRef<Record<string, unknown>>({});
  const industryProfileSeedRef = useRef<IndustryProfile>('GENERAL');
  const tableContainerRef = useRef<HTMLDivElement | null>(null);
  const searchInputRef = useRef<InputRef>(null);
  const searchInputFocusedRef = useRef(false);

  const currentPage = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 50;
  const cursor = cursorMap[currentPage];
  const quickFilterStorageKey = useMemo(
    () => `products.quick-filters.visible.${tenantId ?? 'default'}`,
    [tenantId],
  );

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedFilters(filters);
    }, 300);
    return () => window.clearTimeout(timer);
  }, [filters]);

  useEffect(() => {
    setPagination((prev) => {
      if ((prev.current ?? 1) === 1) {
        return prev;
      }
      return { ...prev, current: 1 };
    });
    setSelectedProduct(null);
  }, [debouncedFilters]);

  useEffect(() => {
    const raw = window.localStorage.getItem(quickFilterStorageKey);
    if (!raw) {
      setQuickFilterVisibleIds(defaultProductQuickFilterIds);
      return;
    }
    try {
      const parsed = JSON.parse(raw) as unknown;
      setQuickFilterVisibleIds(normalizeQuickFilterIds(parsed));
    } catch {
      setQuickFilterVisibleIds(defaultProductQuickFilterIds);
    }
  }, [quickFilterStorageKey]);

  useEffect(() => {
    window.localStorage.setItem(quickFilterStorageKey, JSON.stringify(quickFilterVisibleIds));
  }, [quickFilterStorageKey, quickFilterVisibleIds]);

  const productQuery = useMemo<ProductListQuery>(
    () => ({
      keyword: debouncedFilters.keyword?.trim() || undefined,
      brand: debouncedFilters.brand?.trim() || undefined,
      category: debouncedFilters.category?.trim() || undefined,
      status: debouncedFilters.status?.trim() || undefined,
      cursor,
      limit: pageSize,
    }),
    [
      cursor,
      debouncedFilters.brand,
      debouncedFilters.category,
      debouncedFilters.keyword,
      debouncedFilters.status,
      pageSize,
    ],
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['products', tenantId, productQuery],
    queryFn: () => listProducts(tenantId, productQuery),
    keepPreviousData: true,
    enabled: Boolean(tenantId),
  });
  const listItems = data?.data ?? [];

  useEffect(() => {
    searchInputFocusedRef.current = false;
  }, [tenantId]);

  useEffect(() => {
    if (!tenantId || searchInputFocusedRef.current) {
      return;
    }
    const frameId = window.requestAnimationFrame(() => {
      searchInputRef.current?.focus();
      searchInputFocusedRef.current = true;
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [tenantId]);

  const resolveProductKey = useCallback((product: Product) => product.product_id, []);

  const handleRowContextMenu = useCallback(
    ({ record, event }: { record: Product; event: MouseEvent }) => {
      setContextMenu({ x: event.clientX, y: event.clientY, product: record });
    },
    [setContextMenu],
  );

  const {
    selectedRowKeys,
    rowSelection,
    rowClassName,
    onRow: baseRowHandlers,
    clearSelection,
  } = useUiModule03MainTableInteractions<Product>({
    data: listItems,
    rowKey: resolveProductKey,
    tableContainerRef,
    activeRow: selectedProduct,
    onActiveRowChange: setSelectedProduct,
    onRowContextMenu: handleRowContextMenu,
    contextMenuSelectsRow: false,
    enableRowClickSelection: false,
    enableRowClickMultiSelection: true,
    enableSpaceSelection: true,
  });

  const hasSelection = selectedRowKeys.length > 0;

  useEffect(() => {
    if (!selectedProduct) {
      return;
    }
    const exists = listItems.some((item) => item.product_id === selectedProduct.product_id);
    if (!exists) {
      setSelectedProduct(null);
    }
  }, [listItems, selectedProduct]);

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

  const categoriesQuery = useQuery({
    queryKey: ['product-categories', tenantId],
    queryFn: () => listProductCategories(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const brandsQuery = useQuery({
    queryKey: ['product-brands', tenantId],
    queryFn: () => listProductBrands(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const unitsQuery = useQuery({
    queryKey: ['product-units', tenantId],
    queryFn: () => listProductUnits(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const suppliersQuery = useQuery({
    queryKey: ['suppliers', tenantId],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId),
  });
  const skuTagDefinitionsQuery = useQuery({
    queryKey: ['sku-tag-definitions', tenantId, 'options'],
    queryFn: () =>
      listSkuTagDefinitions(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200,
      }),
    enabled: Boolean(tenantId),
  });

  const categories = categoriesQuery.data?.items ?? [];
  const brands = brandsQuery.data?.items ?? [];
  const units = unitsQuery.data?.items ?? [];
  const suppliers = suppliersQuery.data?.items ?? [];
  const warehouses = warehousesQuery.data?.items ?? [];
  const skuTagDefinitions = skuTagDefinitionsQuery.data?.items ?? [];

  const supplierMap = useMemo(
    () => new Map(suppliers.map((item) => [item.supplier_id, item])),
    [suppliers],
  );
  const warehouseMap = useMemo(
    () => new Map(warehouses.map((item) => [item.warehouse_id, item])),
    [warehouses],
  );

  const categoryOptions = useMemo(
    () => categories.map((item) => ({ value: item.name, label: item.name })),
    [categories],
  );
  const brandOptions = useMemo(
    () => brands.map((item) => ({ value: item.name, label: item.name })),
    [brands],
  );
  const unitOptions = useMemo(
    () => units.map((item) => ({ value: item.name, label: item.name })),
    [units],
  );
  const supplierOptions = useMemo(
    () => suppliers.map((item) => ({ value: item.supplier_id, label: item.name })),
    [suppliers],
  );
  const warehouseOptions = useMemo(
    () =>
      warehouses.map((item) => ({
        value: item.warehouse_id,
        label: `${item.code} ${item.name}`.trim(),
      })),
    [warehouses],
  );
  const skuTagOptions = useMemo(
    () => skuTagDefinitions.map((tag) => ({ value: tag.name, label: tag.name })),
    [skuTagDefinitions],
  );

  const createOption = async (
    kind: '分类' | '品牌' | '单位',
    value: string,
    setter: (state: CreateSelectState) => void,
    action: (payload: { name: string }) => Promise<ProductCategory | ProductBrand | ProductUnit>,
    onSuccess?: (name: string) => void,
  ) => {
    const trimmed = value.trim();
    if (!trimmed) {
      return;
    }
    setter((prev) => ({ ...prev, loading: true }));
    try {
      await action({ name: trimmed });
      message.success(`已新增${kind}`);
      setter({ value: '', loading: false });
      onSuccess?.(trimmed);
    } catch (err) {
      message.error(err instanceof Error ? err.message : `新增${kind}失败`);
      setter((prev) => ({ ...prev, loading: false }));
    }
  };

  const handleCreateCategory = () =>
    createOption(
      '分类',
      createCategory.value,
      setCreateCategory,
      (payload) => createProductCategory(tenantId, payload),
      (name) => {
        form.setFieldValue('category', name);
        void queryClient.invalidateQueries({ queryKey: ['product-categories', tenantId] });
      },
    );
  const handleCreateBrand = () =>
    createOption(
      '品牌',
      createBrand.value,
      setCreateBrand,
      (payload) => createProductBrand(tenantId, payload),
      (name) => {
        form.setFieldValue('brand', name);
        void queryClient.invalidateQueries({ queryKey: ['product-brands', tenantId] });
      },
    );
  const handleCreateUnit = () =>
    createOption(
      '单位',
      createUnit.value,
      setCreateUnit,
      (payload) => createProductUnit(tenantId, payload),
      (name) => {
        form.setFieldValue('base_unit', name);
        void queryClient.invalidateQueries({ queryKey: ['product-units', tenantId] });
      },
    );

  const applyProductToForm = (record: Product, overrides: Partial<ProductFormValues> = {}) => {
    const attrs = readProductAttrs(record);
    const { specs, ...rest } = attrs;
    attrsSnapshotRef.current = rest;
    const industryProfileValue = readStringAttr(attrs, 'industry_profile', 'GENERAL');
    industryProfileSeedRef.current = industryProfileValue;
    const normalizedWarning = normalizeWarningStock(record.warning_stock as WarningStockForm);
    form.setFieldsValue({
      product_code: record.product_code ?? undefined,
      name: record.name,
      alias: record.alias ?? undefined,
      brand: record.brand ?? undefined,
      category: record.category ?? undefined,
      base_unit: record.base_unit ?? undefined,
      second_unit: record.second_unit ?? undefined,
      default_supplier_id: record.default_supplier_id ?? undefined,
      default_warehouse_id: record.default_warehouse_id ?? undefined,
      primary_barcode: record.primary_barcode ?? undefined,
      product_tags: record.product_tags ?? [],
      remark: record.remark ?? undefined,
      warning_stock: normalizedWarning,
      batch_enabled: record.batch_enabled ?? false,
      shelf_life_days: record.shelf_life_days ?? undefined,
      forbid_sale_days: record.forbid_sale_days ?? undefined,
      serial_enabled: record.serial_enabled ?? false,
      serial_push_platform: record.serial_push_platform ?? false,
      industry_profile: industryProfileValue,
      production_date_enabled: readBooleanAttr(attrs, 'production_date_enabled'),
      shelf_life_unit: readStringAttr(attrs, 'shelf_life_unit', 'DAY'),
      near_expiry_warning_days: readNumberAttr(attrs, 'near_expiry_warning_days'),
      variant_mode: readStringAttr(
        attrs,
        'variant_mode',
        industryProfileValue === 'APPAREL' ? 'APPAREL_MATRIX' : 'STANDARD',
      ),
      apparel_size_system: readStringAttr(attrs, 'apparel_size_system', 'CN'),
      serial_tracking_mode: readStringAttr(
        attrs,
        'serial_tracking_mode',
        industryProfileValue === 'DIGITAL' ? 'ITEM' : 'NONE',
      ),
      serial_outbound_required: readBooleanAttr(attrs, 'serial_outbound_required'),
      serial_inbound_unique_required: readBooleanAttr(attrs, 'serial_inbound_unique_required'),
      specs: Array.isArray(specs) ? (specs as ProductSpecForm[]) : [],
      ...overrides,
    });
  };

  const openCreateModal = () => {
    setEditingProduct(null);
    attrsSnapshotRef.current = {};
    industryProfileSeedRef.current = 'GENERAL';
    setModalOpen(true);
    setActiveTab('basic');
    form.resetFields();
    form.setFieldsValue({
      warning_stock: defaultWarningStock,
      batch_enabled: false,
      serial_enabled: false,
      serial_push_platform: false,
      industry_profile: 'GENERAL',
      production_date_enabled: false,
      shelf_life_unit: 'DAY',
      near_expiry_warning_days: undefined,
      variant_mode: 'STANDARD',
      apparel_size_system: 'CN',
      serial_tracking_mode: 'NONE',
      serial_outbound_required: false,
      serial_inbound_unique_required: false,
      product_tags: [],
      specs: [],
    });
  };

  const openEditModal = (record: Product) => {
    setEditingProduct(record);
    setSelectedProduct(record);
    setModalOpen(true);
    setActiveTab('basic');
    applyProductToForm(record);
  };

  const openCopyModal = (record: Product) => {
    setEditingProduct(null);
    setSelectedProduct(record);
    setModalOpen(true);
    setActiveTab('basic');
    applyProductToForm(record, {
      product_code: undefined,
      name: `${record.name}-复制`,
    });
  };

  const handleCloseModal = () => {
    setModalOpen(false);
    setEditingProduct(null);
    setActiveTab('basic');
    attrsSnapshotRef.current = {};
    industryProfileSeedRef.current = 'GENERAL';
  };

  const handleProductModalAfterOpenChange = (open: boolean) => {
    if (open && editingProduct) {
      applyProductToForm(editingProduct);
    }
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      if (!tenantId) {
        message.warning('请先选择租户');
        return;
      }
      const warning_stock = normalizeWarningStock(values.warning_stock);
      const specs = normalizeSpecs(values.specs);
      const attrs = {
        ...omitProductIndustryAttrs(attrsSnapshotRef.current),
        ...buildProductIndustryAttrs(values),
        specs,
      };
      const payload: ProductCreateRequest = {
        product_code: values.product_code?.trim() || undefined,
        name: values.name?.trim() || '',
        alias: values.alias?.trim() || undefined,
        brand: values.brand?.trim() || undefined,
        category: values.category?.trim() || undefined,
        base_unit: values.base_unit?.trim() || undefined,
        second_unit: values.second_unit?.trim() || undefined,
        default_supplier_id: values.default_supplier_id || undefined,
        default_warehouse_id: values.default_warehouse_id || undefined,
        primary_barcode: values.primary_barcode?.trim() || undefined,
        product_tags: normalizeTags(values.product_tags),
        remark: values.remark?.trim() || undefined,
        warning_stock,
        batch_enabled: values.batch_enabled ?? false,
        shelf_life_days: values.shelf_life_days ?? undefined,
        forbid_sale_days: values.forbid_sale_days ?? undefined,
        serial_enabled: values.serial_enabled ?? false,
        serial_push_platform: values.serial_push_platform ?? false,
        attrs,
      };
      setSaving(true);
      if (editingProduct) {
        await updateProduct(tenantId, editingProduct.product_id, payload);
        message.success('货品已更新');
      } else {
        await createProduct(tenantId, payload);
        message.success('货品已创建');
      }
      handleCloseModal();
      await queryClient.invalidateQueries({ queryKey: ['products', tenantId] });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = (record: Product) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    Modal.confirm({
      title: '删除货品',
      content: `确认删除货品 ${record.name} ?`,
      okText: '删除',
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await deleteProduct(tenantId, record.product_id);
          message.success('货品已删除');
          clearSelection();
          setSelectedProduct(null);
          await queryClient.invalidateQueries({ queryKey: ['products', tenantId] });
        } catch (err) {
          message.error(err instanceof Error ? err.message : '删除失败');
        }
      },
    });
  };

  const handleStatusToggle = async (record: Product) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    const nextStatus = isProductInactive(record.status) ? 'ACTIVE' : 'INACTIVE';
    try {
      await updateProduct(tenantId, record.product_id, { status: nextStatus });
      message.success(`已${nextStatus === 'ACTIVE' ? '启用' : '停用'}货品`);
      await queryClient.invalidateQueries({ queryKey: ['products', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新状态失败');
    }
  };

  const handlePaginationChange = (next: TablePaginationConfig) => {
    setPagination((prev) => ({ ...prev, ...next }));
    clearSelection();
    setSelectedProduct(null);
    if (next.pageSize !== pagination.pageSize) {
      setCursorMap({});
    }
  };

  const handleKeywordChange = useCallback(
    (value: string) => setFilters((prev) => ({ ...prev, keyword: value })),
    [],
  );

  const applyFilters = useCallback((nextFilters: ProductFilters) => {
    setFilters(nextFilters);
    setDebouncedFilters(nextFilters);
  }, []);

  const handleResetFilters = useCallback(() => {
    applyFilters({});
  }, [applyFilters]);

  const renderCreateDropdown = (
    menu: ReactElement,
    state: CreateSelectState,
    setState: (state: CreateSelectState) => void,
    onCreate: () => void,
    placeholder: string,
  ) => (
    <div className="products-select-dropdown">
      {menu}
      <Divider style={{ margin: '8px 0' }} />
      <Space style={{ padding: '0 8px 8px' }}>
        <Input
          size="small"
          value={state.value}
          placeholder={placeholder}
          onChange={(event) => setState({ ...state, value: event.target.value })}
        />
        <Button size="small" type="primary" loading={state.loading} onClick={onCreate}>
          新增
        </Button>
      </Space>
    </div>
  );

  const columns = useMemo<ColumnsType<Product>>(
    () => [
      {
        title: '序号',
        key: 'index',
        width: 70,
        align: 'center',
        render: (_value, _record, index) => (currentPage - 1) * pageSize + index + 1,
      },
      {
        title: '编号',
        dataIndex: 'product_code',
        width: 120,
        render: (value) => value ?? '-',
      },
      {
        title: '名称',
        dataIndex: 'name',
        width: 160,
      },
      {
        title: '别名',
        dataIndex: 'alias',
        width: 120,
        render: (value) => value ?? '-',
      },
      {
        title: '分类',
        dataIndex: 'category',
        width: 120,
        render: (value) => value ?? '-',
      },
      {
        title: '品牌',
        dataIndex: 'brand',
        width: 120,
        render: (value) => value ?? '-',
      },
      {
        title: '基础单位',
        dataIndex: 'base_unit',
        width: 100,
        render: (value) => value ?? '-',
      },
      {
        title: '第二单位',
        dataIndex: 'second_unit',
        width: 100,
        render: (value) => value ?? '-',
      },
      {
        title: '主条码',
        dataIndex: 'primary_barcode',
        width: 140,
        render: (value) => value ?? '-',
      },
      {
        title: '默认供应商',
        dataIndex: 'default_supplier_id',
        width: 140,
        render: (value) => supplierMap.get(value ?? '')?.name ?? '-',
      },
      {
        title: '默认仓库',
        dataIndex: 'default_warehouse_id',
        width: 140,
        render: (value) => warehouseMap.get(value ?? '')?.name ?? '-',
      },
      {
        title: '批次管理',
        dataIndex: 'batch_enabled',
        width: 100,
        render: (value) => (value ? <Tag color="blue">启用</Tag> : '-'),
      },
      {
        title: '唯一码',
        dataIndex: 'serial_enabled',
        width: 100,
        render: (value) => (value ? <Tag color="purple">启用</Tag> : '-'),
      },
      {
        title: '库存预警',
        dataIndex: 'warning_stock',
        width: 160,
        render: (value) => formatWarningStock(value as WarningStockForm),
      },
      {
        title: '状态',
        dataIndex: 'status',
        width: 100,
        render: (_, record) => (
          <Switch
            size="small"
            checked={!isProductInactive(record.status)}
            checkedChildren="启用"
            unCheckedChildren="停用"
            onChange={() => handleStatusToggle(record)}
            disabled={!canWrite}
          />
        ),
      },
      {
        title: '操作',
        key: 'actions',
        fixed: 'right',
        width: 160,
        render: (_, record) => (
          <Space>
            <Button size="small" onClick={() => openEditModal(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该货品？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => handleDelete(record)}
              disabled={!canWrite}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        ),
      },
    ],
    [canWrite, currentPage, pageSize, supplierMap, warehouseMap],
  );

  const tableFooterSyncKey = useMemo(
    () => `${columns.length}-${listItems.length}`,
    [columns.length, listItems.length],
  );

  const columnQueryCells = useMemo(
    () => ({
      product_code: (
        <Input
          size="small"
          allowClear
          placeholder="编号/名称/别名/条码"
          value={filters.keyword}
          onChange={(event) => handleKeywordChange(event.target.value)}
        />
      ),
      category: (
        <Select
          size="small"
          allowClear
          placeholder="分类"
          options={categoryOptions}
          value={filters.category}
          onChange={(value) => setFilters((prev) => ({ ...prev, category: value ?? undefined }))}
        />
      ),
      brand: (
        <Select
          size="small"
          allowClear
          placeholder="品牌"
          options={brandOptions}
          value={filters.brand}
          onChange={(value) => setFilters((prev) => ({ ...prev, brand: value ?? undefined }))}
        />
      ),
      status: (
        <Select
          size="small"
          allowClear
          placeholder="状态"
          options={statusOptions}
          value={filters.status}
          onChange={(value) => setFilters((prev) => ({ ...prev, status: value ?? undefined }))}
        />
      ),
    }),
    [
      brandOptions,
      categoryOptions,
      filters.brand,
      filters.category,
      filters.keyword,
      filters.status,
      handleKeywordChange,
      statusOptions,
    ],
  );

  const handleBatchStatus = async (status: 'ACTIVE' | 'INACTIVE') => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (!selectedRowKeys.length) {
      message.warning('请先选择货品');
      return;
    }
    try {
      await Promise.all(
        selectedRowKeys.map((key) => updateProduct(tenantId, String(key), { status })),
      );
      message.success(status === 'ACTIVE' ? '已批量启用' : '已批量停用');
      clearSelection();
      setSelectedProduct(null);
      await queryClient.invalidateQueries({ queryKey: ['products', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '批量更新失败');
    }
  };

  const handleBatchDelete = () => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (!selectedRowKeys.length) {
      message.warning('请先选择货品');
      return;
    }
    Modal.confirm({
      title: '批量删除货品',
      content: `确认删除选中的 ${selectedRowKeys.length} 条货品吗？`,
      okText: '删除',
      okButtonProps: { danger: true },
      onOk: async () => {
        try {
          await Promise.all(selectedRowKeys.map((key) => deleteProduct(tenantId, String(key))));
          message.success('货品已删除');
          clearSelection();
          setSelectedProduct(null);
          await queryClient.invalidateQueries({ queryKey: ['products', tenantId] });
        } catch (err) {
          message.error(err instanceof Error ? err.message : '删除失败');
        }
      },
    });
  };

  const handleEditSelected = () => {
    if (!selectedProduct) {
      message.warning('请先选择货品');
      return;
    }
    openEditModal(selectedProduct);
  };

  const handleCopySelected = () => {
    if (!selectedProduct) {
      message.warning('请先选择货品');
      return;
    }
    openCopyModal(selectedProduct);
  };

  const handleDeleteSelected = () => {
    if (selectedRowKeys.length > 1) {
      handleBatchDelete();
      return;
    }
    if (!selectedProduct) {
      message.warning('请先选择货品');
      return;
    }
    handleDelete(selectedProduct);
  };

  const resolveToolbarTargets = useCallback(() => {
    if (!selectedRowKeys.length) {
      return listItems;
    }
    const selectedIds = new Set(selectedRowKeys.map((key) => String(key)));
    const selectedItems = listItems.filter((item) => selectedIds.has(item.product_id));
    return selectedItems.length ? selectedItems : listItems;
  }, [listItems, selectedRowKeys]);

  const handleExport = useCallback(() => {
    const targets = resolveToolbarTargets();
    if (!targets.length) {
      message.warning('暂无可导出的货品');
      return;
    }
    const rowOffset = (currentPage - 1) * pageSize;
    const headers = [
      '序号',
      '货品编号',
      '货品名称',
      '货品别名',
      '分类',
      '品牌',
      '基础单位',
      '第二单位',
      '主条码',
      '默认供应商',
      '默认仓库',
      '状态',
      '更新时间',
    ];
    const rows = targets.map((record, index) => [
      rowOffset + index + 1,
      record.product_code ?? '-',
      record.name ?? '-',
      record.alias ?? '-',
      record.category ?? '-',
      record.brand ?? '-',
      record.base_unit ?? '-',
      record.second_unit ?? '-',
      record.primary_barcode ?? '-',
      record.default_supplier_id
        ? (supplierMap.get(record.default_supplier_id)?.name ?? record.default_supplier_id)
        : '-',
      record.default_warehouse_id
        ? (warehouseMap.get(record.default_warehouse_id)?.name ?? record.default_warehouse_id)
        : '-',
      formatProductStatus(record.status),
      formatDateTime(record.updated_at ?? record.created_at),
    ]);
    const escapeCsvValue = (value: string | number) => {
      const raw = String(value ?? '');
      if (raw.includes('"') || raw.includes(',') || raw.includes('\n')) {
        return `"${raw.replace(/"/g, '""')}"`;
      }
      return raw;
    };
    const csvContent = [headers, ...rows]
      .map((row) => row.map(escapeCsvValue).join(','))
      .join('\r\n');
    const fileName = `products-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '')}.csv`;
    const blob = new Blob(['\ufeff', csvContent], {
      type: 'text/csv;charset=utf-8;',
    });
    const url = window.URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = fileName;
    document.body.appendChild(link);
    link.click();
    link.remove();
    window.URL.revokeObjectURL(url);
    message.success('导出已完成');
  }, [currentPage, pageSize, resolveToolbarTargets, supplierMap, warehouseMap]);

  const handlePrint = useCallback(() => {
    const targets = resolveToolbarTargets();
    if (!targets.length) {
      message.warning('暂无可打印的货品');
      return;
    }
    const rowOffset = (currentPage - 1) * pageSize;
    const headers = ['序号', '货品编号', '货品名称', '分类', '品牌', '主条码', '状态', '更新时间'];
    const rows = targets.map((record, index) => [
      rowOffset + index + 1,
      record.product_code ?? '-',
      record.name ?? '-',
      record.category ?? '-',
      record.brand ?? '-',
      record.primary_barcode ?? '-',
      formatProductStatus(record.status),
      formatDateTime(record.updated_at ?? record.created_at),
    ]);
    const escapeHtml = (value: string | number) =>
      String(value ?? '')
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
    const tableHead = headers.map((header) => `<th>${escapeHtml(header)}</th>`).join('');
    const tableBody = rows
      .map((row) => `<tr>${row.map((cell) => `<td>${escapeHtml(cell)}</td>`).join('')}</tr>`)
      .join('');
    const html = `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>货品打印</title>
    <style>
      body { font-family: Arial, sans-serif; padding: 16px; }
      table { width: 100%; border-collapse: collapse; font-size: 12px; }
      th, td { border: 1px solid #ddd; padding: 6px 8px; text-align: left; }
      th { background: #f5f5f5; }
    </style>
  </head>
  <body>
    <table>
      <thead><tr>${tableHead}</tr></thead>
      <tbody>${tableBody}</tbody>
    </table>
  </body>
</html>`;
    const printWindow = window.open('', '_blank', 'width=1200,height=800');
    if (!printWindow) {
      message.error('无法打开打印窗口');
      return;
    }
    printWindow.document.write(html);
    printWindow.document.close();
    printWindow.focus();
    printWindow.print();
    printWindow.onafterprint = () => {
      printWindow.close();
    };
  }, [currentPage, pageSize, resolveToolbarTargets]);

  const handleOpenHelp = useCallback(() => {
    setHelpModalOpen(true);
  }, []);

  const quickFilters = (
    <Space size={12} wrap={false} className="products-toolbar-filters">
      <Input.Search
        size="small"
        allowClear
        autoFocus
        ref={searchInputRef}
        placeholder="编号/名称/别名/条码"
        value={filters.keyword}
        onChange={(event) => handleKeywordChange(event.target.value)}
        onSearch={(value) => handleKeywordChange(value)}
        data-testid="products-search"
      />
      <Button size="small" onClick={handleResetFilters} data-testid="products-search-reset">
        重置
      </Button>
    </Space>
  );

  const quickFilterItems = useMemo(
    () => defaultProductQuickFilterItems.filter((item) => quickFilterVisibleIds.includes(item.id)),
    [quickFilterVisibleIds],
  );

  const handleQuickFilterSelect = useCallback(
    (filterId: string) => {
      const normalizedFilterId = filterId as ProductQuickFilterId;
      const nextFilters: ProductFilters =
        normalizedFilterId === 'all'
          ? {}
          : {
              status: normalizedFilterId as ProductFilters['status'],
            };
      applyFilters(nextFilters);
      clearSelection();
      setSelectedProduct(null);
      setPagination((prev) => ({ ...prev, current: 1 }));
    },
    [applyFilters, clearSelection],
  );

  const handleOpenQuickFilterSettings = useCallback(() => {
    setQuickFilterSettingDraft(quickFilterVisibleIds);
    setQuickFilterSettingsOpen(true);
  }, [quickFilterVisibleIds]);

  const handleResetQuickFilterSettings = useCallback(() => {
    setQuickFilterSettingDraft(defaultProductQuickFilterIds);
  }, []);

  const handleSaveQuickFilterSettings = useCallback(() => {
    if (!quickFilterSettingDraft.length) {
      message.warning('至少保留一个快捷筛选');
      return;
    }
    const nextVisibleIds = normalizeQuickFilterIds(quickFilterSettingDraft);
    setQuickFilterVisibleIds(nextVisibleIds);
    setQuickFilterSettingsOpen(false);
    const activeId = (filters.status ?? 'all') as ProductQuickFilterId;
    if (!nextVisibleIds.includes(activeId)) {
      const fallback = nextVisibleIds.includes('all') ? 'all' : nextVisibleIds[0];
      handleQuickFilterSelect(fallback);
    }
  }, [filters.status, handleQuickFilterSelect, quickFilterSettingDraft]);

  const activeQuickFilterId = useMemo<ProductQuickFilterId>(() => {
    const activeId = (filters.status ?? 'all') as ProductQuickFilterId;
    if (quickFilterItems.some((item) => item.id === activeId)) {
      return activeId;
    }
    return quickFilterItems[0]?.id ?? 'all';
  }, [filters.status, quickFilterItems]);

  const quickFilterPanel = useMemo(
    () => (
      <UiModule01QuickFiltersPanel
        filters={quickFilterItems}
        activeFilterId={activeQuickFilterId}
        onOpenSettings={handleOpenQuickFilterSettings}
        onSelectFilter={handleQuickFilterSelect}
        dataTestId="products-quick-filters"
        settingsButtonTestId="products-quick-filter-settings-open"
        filterButtonTestIdPrefix="products-quick-filter-item"
      />
    ),
    [activeQuickFilterId, handleOpenQuickFilterSettings, handleQuickFilterSelect, quickFilterItems],
  );

  const quickQueryPanel = useMemo(
    () => (
      <div className="products-left-query">
        <Typography.Text strong>条件查询</Typography.Text>
        <Space direction="vertical" size={8} style={{ width: '100%' }}>
          <Input
            size="small"
            allowClear
            placeholder="编号/名称/别名/条码"
            value={filters.keyword}
            onChange={(event) => handleKeywordChange(event.target.value)}
          />
          <Select
            size="small"
            allowClear
            placeholder="分类"
            options={categoryOptions}
            value={filters.category}
            onChange={(value) => setFilters((prev) => ({ ...prev, category: value ?? undefined }))}
          />
          <Select
            size="small"
            allowClear
            placeholder="品牌"
            options={brandOptions}
            value={filters.brand}
            onChange={(value) => setFilters((prev) => ({ ...prev, brand: value ?? undefined }))}
          />
          <Select
            size="small"
            allowClear
            placeholder="状态"
            options={statusOptions}
            value={filters.status}
            onChange={(value) => setFilters((prev) => ({ ...prev, status: value ?? undefined }))}
          />
          <Space size={8}>
            <Button size="small" type="primary" onClick={() => applyFilters(filters)}>
              查询
            </Button>
            <Button size="small" onClick={handleResetFilters}>
              重置
            </Button>
          </Space>
        </Space>
      </div>
    ),
    [
      applyFilters,
      brandOptions,
      categoryOptions,
      filters,
      handleKeywordChange,
      handleResetFilters,
      statusOptions,
    ],
  );

  const leftPanelContent = useMemo(
    () => (
      <Space direction="vertical" size={12} style={{ width: '100%' }}>
        {quickFilterPanel}
        {quickQueryPanel}
      </Space>
    ),
    [quickFilterPanel, quickQueryPanel],
  );

  const batchEnabled = Form.useWatch('batch_enabled', form);
  const serialEnabled = Form.useWatch('serial_enabled', form);
  const industryProfile = Form.useWatch('industry_profile', form) ?? 'GENERAL';
  const specs = Form.useWatch('specs', form) ?? [];

  useEffect(() => {
    if (!modalOpen || !industryProfile || industryProfileSeedRef.current === industryProfile) {
      return;
    }
    industryProfileSeedRef.current = industryProfile;
    if (industryProfile === 'FOOD') {
      form.setFieldsValue({
        batch_enabled: true,
        production_date_enabled: true,
        shelf_life_unit: form.getFieldValue('shelf_life_unit') ?? 'DAY',
      });
      return;
    }
    if (industryProfile === 'APPAREL') {
      form.setFieldsValue({
        variant_mode: 'APPAREL_MATRIX',
        apparel_size_system: form.getFieldValue('apparel_size_system') ?? 'CN',
      });
      return;
    }
    if (industryProfile === 'DIGITAL') {
      form.setFieldsValue({
        serial_enabled: true,
        serial_tracking_mode: form.getFieldValue('serial_tracking_mode') ?? 'ITEM',
      });
      return;
    }
    form.setFieldsValue({
      variant_mode: 'STANDARD',
      serial_tracking_mode: 'NONE',
    });
  }, [form, industryProfile, modalOpen]);

  const warehouseQuery = useQuery({
    queryKey: ['product-warehouses', tenantId, editingProduct?.product_id],
    queryFn: () => listProductWarehouses(tenantId, editingProduct?.product_id ?? ''),
    enabled: Boolean(tenantId && editingProduct?.product_id && modalOpen),
  });

  const kitQuery = useQuery({
    queryKey: ['product-kits', tenantId, editingProduct?.product_id],
    queryFn: () =>
      listProductKits(tenantId, editingProduct?.product_id ?? '', { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId && editingProduct?.product_id && modalOpen),
  });

  const logQuery = useQuery({
    queryKey: ['product-logs', tenantId, editingProduct?.product_id],
    queryFn: () =>
      listOperationLogs(tenantId, {
        ref_type: 'PRODUCT',
        ref_id: editingProduct?.product_id,
        limit: 50,
      }),
    enabled: Boolean(tenantId && editingProduct?.product_id && modalOpen),
  });

  const panelWarehouseQuery = useQuery({
    queryKey: ['product-warehouses', tenantId, selectedProduct?.product_id, 'panel'],
    queryFn: () => listProductWarehouses(tenantId, selectedProduct?.product_id ?? ''),
    enabled: Boolean(tenantId && selectedProduct?.product_id),
    keepPreviousData: true,
  });

  const panelKitQuery = useQuery({
    queryKey: ['product-kits', tenantId, selectedProduct?.product_id, 'panel'],
    queryFn: () =>
      listProductKits(tenantId, selectedProduct?.product_id ?? '', { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId && selectedProduct?.product_id),
    keepPreviousData: true,
  });

  const panelLogQuery = useQuery({
    queryKey: ['product-logs', tenantId, selectedProduct?.product_id, 'panel'],
    queryFn: () =>
      listOperationLogs(tenantId, {
        ref_type: 'PRODUCT',
        ref_id: selectedProduct?.product_id,
        limit: 50,
      }),
    enabled: Boolean(tenantId && selectedProduct?.product_id),
    keepPreviousData: true,
  });

  const productWarehouses = warehouseQuery.data?.items ?? [];
  const productKits = kitQuery.data?.items ?? [];
  const productLogs = logQuery.data?.data ?? [];

  const panelWarehouses = panelWarehouseQuery.data?.items ?? [];
  const panelKits = panelKitQuery.data?.items ?? [];
  const panelLogs = panelLogQuery.data?.data ?? [];

  const [warehouseForm] = Form.useForm<{
    warehouse_id?: string;
    is_default?: boolean;
    allow_negative_stock?: boolean;
  }>();

  const handleAddWarehouse = async () => {
    try {
      const values = await warehouseForm.validateFields();
      if (!tenantId || !editingProduct) {
        message.warning('请先保存货品');
        return;
      }
      await createProductWarehouse(tenantId, editingProduct.product_id, {
        warehouse_id: values.warehouse_id,
        is_default: values.is_default ?? false,
        allow_negative_stock: values.allow_negative_stock ?? false,
      });
      if (values.is_default) {
        await updateProduct(tenantId, editingProduct.product_id, {
          default_warehouse_id: values.warehouse_id,
        });
      }
      message.success('已新增仓库配置');
      warehouseForm.resetFields();
      await queryClient.invalidateQueries({
        queryKey: ['product-warehouses', tenantId, editingProduct.product_id],
      });
      await queryClient.invalidateQueries({ queryKey: ['products', tenantId] });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    }
  };

  const handleWarehouseDefault = async (record: ProductWarehouse, isDefault: boolean) => {
    if (!tenantId || !editingProduct) {
      return;
    }
    try {
      await updateProductWarehouse(tenantId, editingProduct.product_id, record.warehouse_id, {
        is_default: isDefault,
      });
      await updateProduct(tenantId, editingProduct.product_id, {
        default_warehouse_id: isDefault ? record.warehouse_id : undefined,
      });
      message.success('默认仓库已更新');
      await queryClient.invalidateQueries({
        queryKey: ['product-warehouses', tenantId, editingProduct.product_id],
      });
      await queryClient.invalidateQueries({ queryKey: ['products', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新默认仓库失败');
    }
  };

  const handleWarehouseNegative = async (record: ProductWarehouse, allow: boolean) => {
    if (!tenantId || !editingProduct) {
      return;
    }
    try {
      await updateProductWarehouse(tenantId, editingProduct.product_id, record.warehouse_id, {
        allow_negative_stock: allow,
      });
      message.success('仓库设置已更新');
      await queryClient.invalidateQueries({
        queryKey: ['product-warehouses', tenantId, editingProduct.product_id],
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新仓库失败');
    }
  };

  const handleDeleteWarehouse = async (record: ProductWarehouse) => {
    if (!tenantId || !editingProduct) {
      return;
    }
    try {
      await deleteProductWarehouse(tenantId, editingProduct.product_id, record.warehouse_id);
      message.success('仓库已移除');
      await queryClient.invalidateQueries({
        queryKey: ['product-warehouses', tenantId, editingProduct.product_id],
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除仓库失败');
    }
  };

  const [kitModalOpen, setKitModalOpen] = useState(false);
  const [kitSkuKeyword, setKitSkuKeyword] = useState('');
  const [kitSkuOptions, setKitSkuOptions] = useState<SkuSearchItem[]>([]);
  const [kitSkuLoading, setKitSkuLoading] = useState(false);
  const [selectedKitSku, setSelectedKitSku] = useState<string | null>(null);

  const loadKitSkuOptions = async (keyword?: string) => {
    if (!tenantId) {
      return;
    }
    setKitSkuLoading(true);
    try {
      const response = await searchSkus(tenantId, keyword ?? '', 'KIT');
      setKitSkuOptions(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载组合装SKU失败');
    } finally {
      setKitSkuLoading(false);
    }
  };

  const handleOpenKitEditor = () => {
    if (!selectedKitSku) {
      message.warning('请先选择组合装SKU');
      return;
    }
    window.open(`/kits/${selectedKitSku}`, '_blank');
    setKitModalOpen(false);
    setSelectedKitSku(null);
  };

  const handleDeleteKit = async (record: KitSummary) => {
    if (!tenantId) {
      return;
    }
    try {
      await deleteSku(tenantId, record.kit_sku_id);
      message.success('组合装已删除');
      await queryClient.invalidateQueries({
        queryKey: ['product-kits', tenantId, editingProduct?.product_id],
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除组合装失败');
    }
  };

  const warehouseColumns = useMemo<ColumnsType<ProductWarehouse>>(
    () => [
      {
        title: '仓库编码',
        dataIndex: 'warehouse_code',
      },
      {
        title: '仓库名称',
        dataIndex: 'warehouse_name',
      },
      {
        title: '默认仓库',
        dataIndex: 'is_default',
        render: (_, record) => (
          <Checkbox
            checked={record.is_default}
            onChange={(event) => handleWarehouseDefault(record, event.target.checked)}
            disabled={!canWrite}
          />
        ),
      },
      {
        title: '允许负库存',
        dataIndex: 'allow_negative_stock',
        render: (_, record) => (
          <Switch
            size="small"
            checked={record.allow_negative_stock}
            onChange={(value) => handleWarehouseNegative(record, value)}
            disabled={!canWrite}
          />
        ),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Popconfirm
            title="确认移除该仓库？"
            okText="移除"
            cancelText="取消"
            onConfirm={() => handleDeleteWarehouse(record)}
            disabled={!canWrite}
          >
            <Button size="small" danger disabled={!canWrite}>
              移除
            </Button>
          </Popconfirm>
        ),
      },
    ],
    [canWrite],
  );

  const kitColumns = useMemo<ColumnsType<KitSummary>>(
    () => [
      {
        title: '组合装SKU',
        dataIndex: 'kit_sku_code',
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <span>{record.kit_sku_code}</span>
            <span>{record.kit_name ?? '-'}</span>
          </Space>
        ),
      },
      {
        title: '组件数量',
        dataIndex: 'component_count',
        align: 'right',
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              onClick={() => window.open(`/kits/${record.kit_sku_id}`, '_blank')}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该组合装？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => handleDeleteKit(record)}
              disabled={!canWrite}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        ),
      },
    ],
    [canWrite],
  );

  const logColumns = useMemo<ColumnsType<OperationLog>>(
    () => [
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value),
      },
      {
        title: '操作人',
        dataIndex: 'actor_user_id',
        render: (value) => value ?? '-',
      },
      {
        title: '动作',
        dataIndex: 'action',
        render: (_, record) => resolveOperationActionLabel(record.action),
      },
      {
        title: '详情',
        dataIndex: 'metadata',
        render: (value) =>
          value && Object.keys(value).length ? (
            <Typography.Text type="secondary">{JSON.stringify(value)}</Typography.Text>
          ) : (
            '-'
          ),
      },
    ],
    [],
  );

  const panelWarehouseColumns = useMemo<ColumnsType<ProductWarehouse>>(
    () => [
      {
        title: '仓库',
        dataIndex: 'warehouse_name',
        render: (_, record) => record.warehouse_name ?? record.warehouse_code ?? '-',
      },
      {
        title: '默认仓库',
        dataIndex: 'is_default',
        render: (_, record) => (record.is_default ? <Tag color="green">是</Tag> : '-'),
      },
      {
        title: '允许负库存',
        dataIndex: 'allow_negative_stock',
        render: (_, record) => (record.allow_negative_stock ? <Tag>允许</Tag> : '-'),
      },
    ],
    [],
  );

  const panelKitColumns = useMemo<ColumnsType<KitSummary>>(
    () => [
      {
        title: '组合装SKU',
        dataIndex: 'kit_sku_code',
        render: (_, record) => (
          <Space direction="vertical" size={0}>
            <span>{record.kit_sku_code}</span>
            <span>{record.kit_name ?? '-'}</span>
          </Space>
        ),
      },
      {
        title: '组件数量',
        dataIndex: 'component_count',
        align: 'right',
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button size="small" onClick={() => window.open(`/kits/${record.kit_sku_id}`, '_blank')}>
            打开
          </Button>
        ),
      },
    ],
    [],
  );

  useEffect(() => {
    const tags = new Set(specTagOptions);
    (form.getFieldValue('specs') ?? []).forEach((spec: ProductSpecForm) => {
      if (spec?.tag) {
        tags.add(spec.tag);
      }
    });
    if (tags.size !== specTagOptions.length) {
      setSpecTagOptions(Array.from(tags));
    }
  }, [form, specTagOptions]);

  const toolbarLeft = (
    <Space size={8} wrap>
      <Tooltip title={!canWrite ? '暂无权限' : undefined}>
        <span>
          <Button
            type="primary"
            onClick={openCreateModal}
            disabled={!canWrite}
            data-testid="products-create-button"
          >
            新建货品
          </Button>
        </span>
      </Tooltip>
      <Button onClick={handleCopySelected} disabled={!canWrite || !selectedProduct}>
        复制
      </Button>
      <Button onClick={handleEditSelected} disabled={!canWrite || !selectedProduct}>
        修改
      </Button>
      <Button
        danger
        onClick={handleDeleteSelected}
        disabled={!canWrite || (!hasSelection && !selectedProduct)}
      >
        删除
      </Button>
      <Button onClick={() => handleBatchStatus('ACTIVE')} disabled={!canWrite || !hasSelection}>
        启用
      </Button>
      <Button onClick={() => handleBatchStatus('INACTIVE')} disabled={!canWrite || !hasSelection}>
        停用
      </Button>
      <Button onClick={() => refetch()}>刷新</Button>
    </Space>
  );

  const toolbarCenter = quickFilters;

  const toolbarRight = (
    <UiModule02ToolbarRight
      onExport={handleExport}
      onPrint={handlePrint}
      onHelp={handleOpenHelp}
      exportButtonTestId="products-export-button"
      printButtonTestId="products-print-button"
      helpButtonTestId="products-help-button"
    />
  );

  const tableFooter = (
    <UiModule03TableFooter
      tableContainerRef={tableContainerRef}
      syncKey={tableFooterSyncKey}
      summary={`共 ${total} 条，已勾选 ${selectedRowKeys.length} 条`}
      total={total}
      pagination={pagination}
      onPaginationChange={handlePaginationChange}
    />
  );

  const detailPanels = useMemo(() => {
    const tags = selectedProduct?.product_tags ?? [];
    const detailContent = selectedProduct ? (
      <Space direction="vertical" size={8} style={{ width: '100%' }}>
        <Descriptions size="small" column={1}>
          <Descriptions.Item label="货品编号">
            {selectedProduct.product_code ?? '-'}
          </Descriptions.Item>
          <Descriptions.Item label="货品名称">{selectedProduct.name ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="货品别名">{selectedProduct.alias ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="分类">{selectedProduct.category ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="品牌">{selectedProduct.brand ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="基础单位">{selectedProduct.base_unit ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="第二单位">{selectedProduct.second_unit ?? '-'}</Descriptions.Item>
          <Descriptions.Item label="默认供应商">
            {selectedProduct.default_supplier_id
              ? (supplierMap.get(selectedProduct.default_supplier_id)?.name ??
                selectedProduct.default_supplier_id)
              : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="默认仓库">
            {selectedProduct.default_warehouse_id
              ? (warehouseMap.get(selectedProduct.default_warehouse_id)?.name ??
                selectedProduct.default_warehouse_id)
              : '-'}
          </Descriptions.Item>
          <Descriptions.Item label="主条码">
            {selectedProduct.primary_barcode ?? '-'}
          </Descriptions.Item>
          <Descriptions.Item label="状态">
            {isProductInactive(selectedProduct.status) ? (
              <Tag>停用</Tag>
            ) : (
              <Tag color="green">启用</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="更新时间">
            {formatDateTime(selectedProduct.updated_at ?? selectedProduct.created_at)}
          </Descriptions.Item>
        </Descriptions>
        <ProductIndustrySummary product={selectedProduct} />
      </Space>
    ) : (
      <Typography.Text type="secondary">请选择货品查看详情</Typography.Text>
    );

    const tagContent = selectedProduct ? (
      tags.length ? (
        <Space size={6} wrap>
          {tags.map((tag) => (
            <Tag key={tag}>{tag}</Tag>
          ))}
        </Space>
      ) : (
        <Typography.Text type="secondary">暂无货品标签</Typography.Text>
      )
    ) : (
      <Typography.Text type="secondary">请选择货品查看标签</Typography.Text>
    );

    const actionContent = (
      <Space size={8} wrap>
        <Button size="small" onClick={handleCopySelected} disabled={!canWrite || !selectedProduct}>
          复制货品
        </Button>
        <Button size="small" onClick={handleEditSelected} disabled={!canWrite || !selectedProduct}>
          编辑货品
        </Button>
      </Space>
    );

    return [
      { key: 'detail', label: '货品详情', children: detailContent },
      { key: 'tags', label: '货品标签', children: tagContent },
      { key: 'actions', label: '快捷操作', children: actionContent },
    ];
  }, [
    canWrite,
    handleCopySelected,
    handleEditSelected,
    selectedProduct,
    supplierMap,
    warehouseMap,
  ]);

  const rightPanel = useMemo(
    () => <UiModule05DetailPanel panels={detailPanels} defaultActiveKeys={['detail']} />,
    [detailPanels],
  );

  const bottomPanel = useMemo(
    () => (
      <div className="products-bottom-panel">
        <Tabs
          size="small"
          type="card"
          activeKey={activeBottomTab}
          onChange={(key) => setActiveBottomTab(key)}
          items={[
            {
              key: 'warehouses',
              label: `仓库配置 (${panelWarehouses.length})`,
              children: selectedProduct ? (
                <UiModule04BottomTable
                  syncKey={`warehouses-${panelWarehouses.length}-${selectedProduct.product_id}`}
                >
                  <Table
                    size="small"
                    rowKey="warehouse_id"
                    columns={panelWarehouseColumns}
                    dataSource={panelWarehouses}
                    loading={panelWarehouseQuery.isFetching}
                    pagination={false}
                    scroll={{ x: 'max-content' }}
                    locale={{ emptyText: <Empty description="暂无仓库配置" /> }}
                  />
                </UiModule04BottomTable>
              ) : (
                <Empty description="请选择货品查看仓库配置" />
              ),
            },
            {
              key: 'kits',
              label: `关联组合装 (${panelKits.length})`,
              children: selectedProduct ? (
                <UiModule04BottomTable
                  syncKey={`kits-${panelKits.length}-${selectedProduct.product_id}`}
                >
                  <Table
                    size="small"
                    rowKey="kit_sku_id"
                    columns={panelKitColumns}
                    dataSource={panelKits}
                    loading={panelKitQuery.isFetching}
                    pagination={false}
                    scroll={{ x: 'max-content' }}
                    locale={{ emptyText: <Empty description="暂无组合装" /> }}
                  />
                </UiModule04BottomTable>
              ) : (
                <Empty description="请选择货品查看组合装" />
              ),
            },
            {
              key: 'logs',
              label: `操作日志 (${panelLogs.length})`,
              children: selectedProduct ? (
                <UiModule04BottomTable
                  syncKey={`logs-${panelLogs.length}-${selectedProduct.product_id}`}
                >
                  <Table
                    size="small"
                    rowKey="log_id"
                    columns={logColumns}
                    dataSource={panelLogs}
                    loading={panelLogQuery.isFetching}
                    pagination={false}
                    scroll={{ x: 'max-content' }}
                    locale={{ emptyText: <Empty description="暂无日志" /> }}
                  />
                </UiModule04BottomTable>
              ) : (
                <Empty description="请选择货品查看日志" />
              ),
            },
          ]}
        />
      </div>
    ),
    [
      activeBottomTab,
      logColumns,
      panelKits,
      panelKitColumns,
      panelKitQuery.isFetching,
      panelLogs,
      panelLogQuery.isFetching,
      panelWarehouses,
      panelWarehouseColumns,
      panelWarehouseQuery.isFetching,
      selectedProduct,
    ],
  );

  useEffect(() => {
    setLeftPanelVisible(true);
    setRightPanelVisible(true);
    setLeftPanel(leftPanelContent);
    setRightPanel(rightPanel);
    return () => clearPanels();
  }, [
    clearPanels,
    leftPanelContent,
    rightPanel,
    setLeftPanel,
    setLeftPanelVisible,
    setRightPanel,
    setRightPanelVisible,
  ]);

  const mainTableRowHandlers = useCallback(
    (record: Product, index?: number) => {
      const baseHandlers = baseRowHandlers?.(record, index) ?? {};
      return {
        ...baseHandlers,
        onDoubleClick: (event) => {
          baseHandlers.onDoubleClick?.(event);
          if (event.defaultPrevented) {
            return;
          }
          if (!canWrite) {
            message.warning('暂无编辑权限');
            return;
          }
          openEditModal(record);
        },
      };
    },
    [baseRowHandlers, canWrite, openEditModal],
  );

  const contextMenuItems: MenuProps['items'] = [
    { key: 'edit', label: '修改货品', disabled: !canWrite },
    { key: 'copy', label: '复制货品', disabled: !canWrite },
    { key: 'delete', label: '删除货品', disabled: !canWrite },
  ];

  return (
    <>
      <div data-testid="products-page">
        <UiModule03MainTableSection<Product>
          className="products-page"
          tableClassName="products-table"
          tableContainerRef={tableContainerRef}
          dataTestId="products-data-grid"
          suppressInitialFocus
          toolbarLeft={toolbarLeft}
          toolbarCenter={toolbarCenter}
          toolbarRight={toolbarRight}
          columns={columns}
          data={listItems}
          loading={isLoading || isFetching}
          error={error}
          onRetry={() => refetch()}
          errorTitle="货品列表加载失败"
          emptyText="暂无货品"
          total={total}
          pagination={pagination}
          onPaginationChange={handlePaginationChange}
          columnSettingsKey="products.table.columns"
          queryCells={columnQueryCells}
          rowKey="product_id"
          rowSelection={rowSelection}
          rowClassName={rowClassName}
          contextMenuSelectsRow={false}
          onRow={mainTableRowHandlers}
          tableFooter={tableFooter}
          bottomPanel={bottomPanel}
        />
      </div>

      <UiModule03ContextMenu
        open={Boolean(contextMenu)}
        position={contextMenu}
        menu={{
          items: contextMenuItems,
          onClick: ({ key }) => {
            if (!contextMenu) {
              return;
            }
            const record = contextMenu.product;
            if (key === 'edit') {
              openEditModal(record);
            } else if (key === 'copy') {
              openCopyModal(record);
            } else if (key === 'delete') {
              handleDelete(record);
            }
            setContextMenu(null);
          },
        }}
        onClose={() => setContextMenu(null)}
      />

      <Modal
        title="快捷筛选设置"
        open={quickFilterSettingsOpen}
        onCancel={() => setQuickFilterSettingsOpen(false)}
        onOk={handleSaveQuickFilterSettings}
        okText="保存"
        cancelText="取消"
        okButtonProps={{ 'data-testid': 'products-quick-filter-settings-save' }}
        cancelButtonProps={{ 'data-testid': 'products-quick-filter-settings-cancel' }}
        data-testid="products-quick-filter-settings-modal"
      >
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Typography.Text type="secondary">请选择需要展示的快捷筛选按钮。</Typography.Text>
          <div data-testid="products-quick-filter-settings-options">
            <Checkbox.Group
              options={productQuickFilterSettingOptions}
              value={quickFilterSettingDraft}
              onChange={(values) => {
                setQuickFilterSettingDraft(values as ProductQuickFilterId[]);
              }}
            />
          </div>
          <Button
            type="link"
            style={{ padding: 0, alignSelf: 'flex-start' }}
            onClick={handleResetQuickFilterSettings}
          >
            恢复默认
          </Button>
        </Space>
      </Modal>

      <Modal
        title="货品功能帮助"
        open={helpModalOpen}
        onCancel={() => setHelpModalOpen(false)}
        footer={
          <Button
            type="primary"
            onClick={() => setHelpModalOpen(false)}
            data-testid="products-help-modal-close"
          >
            我知道了
          </Button>
        }
        data-testid="products-help-modal"
      >
        <Space direction="vertical" size={8}>
          <Typography.Text>
            1. 先通过左侧快捷筛选或条件查询定位货品，再进行编辑、复制或删除。
          </Typography.Text>
          <Typography.Text>
            2. 工具栏“导出”会导出当前结果或已勾选货品；“打印”会生成可打印清单。
          </Typography.Text>
          <Typography.Text>
            3. 双击表格行可快速进入编辑；右键可打开上下文菜单执行常用操作。
          </Typography.Text>
        </Space>
      </Modal>

      <Modal
        title={editingProduct ? '编辑货品' : '新建货品'}
        open={modalOpen}
        onCancel={handleCloseModal}
        afterOpenChange={handleProductModalAfterOpenChange}
        className="products-edit-modal"
        rootClassName="products-edit-modal"
        width={1100}
        centered
        styles={{
          content: { padding: 0 },
          header: { padding: '2px 12px', minHeight: 24, margin: 0 },
          body: { padding: '12px 16px' },
        }}
        destroyOnClose
        data-testid="products-modal"
        footer={
          <Space>
            <Button onClick={handleCloseModal}>取消</Button>
            <Button type="primary" onClick={handleSave} loading={saving} disabled={!canWrite}>
              保存
            </Button>
          </Space>
        }
      >
        <Tabs
          className="products-edit-tabs"
          size="small"
          activeKey={activeTab}
          onChange={setActiveTab}
          items={[
            {
              key: 'basic',
              label: '基础信息',
              children: (
                <Form
                  form={form}
                  layout="horizontal"
                  size="small"
                  labelAlign="left"
                  labelCol={{ flex: '90px' }}
                  wrapperCol={{ flex: 1 }}
                  className="products-edit-form"
                >
                  <Row gutter={[8, 2]}>
                    <Col span={6}>
                      <Form.Item label="分类" name="category">
                        <Select
                          allowClear
                          options={categoryOptions}
                          dropdownRender={(menu) =>
                            renderCreateDropdown(
                              menu,
                              createCategory,
                              setCreateCategory,
                              handleCreateCategory,
                              '新增分类',
                            )
                          }
                        />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="编号" name="product_code">
                        <Input placeholder="货品编号" />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item
                        label="名称"
                        name="name"
                        rules={[{ required: true, message: '请输入货品名称' }]}
                      >
                        <Input placeholder="货品名称" />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="别名" name="alias">
                        <Input placeholder="货品别名" />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="基础单位" name="base_unit">
                        <Select
                          allowClear
                          options={unitOptions}
                          dropdownRender={(menu) =>
                            renderCreateDropdown(
                              menu,
                              createUnit,
                              setCreateUnit,
                              handleCreateUnit,
                              '新增单位',
                            )
                          }
                        />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="第二单位" name="second_unit">
                        <Select allowClear options={unitOptions} />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="品牌" name="brand">
                        <Select
                          allowClear
                          options={brandOptions}
                          dropdownRender={(menu) =>
                            renderCreateDropdown(
                              menu,
                              createBrand,
                              setCreateBrand,
                              handleCreateBrand,
                              '新增品牌',
                            )
                          }
                        />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="默认供应商" name="default_supplier_id">
                        <Select allowClear options={supplierOptions} />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="默认存放仓库" name="default_warehouse_id">
                        <Select allowClear options={warehouseOptions} />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="主条码" name="primary_barcode">
                        <Input placeholder="主条码" />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="行业模板" name="industry_profile">
                        <Select
                          options={industryProfileOptions}
                          data-testid="products-industry-profile"
                        />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="货品标记" name="product_tags">
                        <Select
                          mode="multiple"
                          allowClear
                          showSearch
                          placeholder="请选择标记"
                          options={skuTagOptions}
                          optionFilterProp="label"
                          notFoundContent="暂无标记，请先在标记目录新建"
                          loading={skuTagDefinitionsQuery.isLoading}
                        />
                      </Form.Item>
                    </Col>
                    <Col span={12}>
                      <Form.Item label="备注" name="remark">
                        <Input.TextArea rows={1} placeholder="备注信息" />
                      </Form.Item>
                    </Col>
                    <Col span={12}>
                      <Form.Item label="库存预警">
                        <Space direction="vertical" size={6} style={{ width: '100%' }}>
                          <Space size={6}>
                            <Form.Item
                              name={['warning_stock', 'upper', 'mode']}
                              style={{ marginBottom: 0 }}
                            >
                              <Select options={warningModeOptions} />
                            </Form.Item>
                            <Form.Item
                              shouldUpdate={(prev, curr) =>
                                prev.warning_stock?.upper?.mode !== curr.warning_stock?.upper?.mode
                              }
                              style={{ marginBottom: 0 }}
                            >
                              {() => {
                                const mode = form.getFieldValue([
                                  'warning_stock',
                                  'upper',
                                  'mode',
                                ]) as WarningStockMode;
                                if (!mode || mode === 'UNLIMITED') {
                                  return <Input disabled placeholder="上限" />;
                                }
                                return (
                                  <Form.Item
                                    name={['warning_stock', 'upper', 'value']}
                                    style={{ marginBottom: 0 }}
                                  >
                                    <InputNumber min={0} placeholder="上限值" />
                                  </Form.Item>
                                );
                              }}
                            </Form.Item>
                          </Space>
                          <Space size={6}>
                            <Form.Item
                              name={['warning_stock', 'lower', 'mode']}
                              style={{ marginBottom: 0 }}
                            >
                              <Select options={warningModeOptions} />
                            </Form.Item>
                            <Form.Item
                              shouldUpdate={(prev, curr) =>
                                prev.warning_stock?.lower?.mode !== curr.warning_stock?.lower?.mode
                              }
                              style={{ marginBottom: 0 }}
                            >
                              {() => {
                                const mode = form.getFieldValue([
                                  'warning_stock',
                                  'lower',
                                  'mode',
                                ]) as WarningStockMode;
                                if (!mode || mode === 'UNLIMITED') {
                                  return <Input disabled placeholder="下限" />;
                                }
                                return (
                                  <Form.Item
                                    name={['warning_stock', 'lower', 'value']}
                                    style={{ marginBottom: 0 }}
                                  >
                                    <InputNumber min={0} placeholder="下限值" />
                                  </Form.Item>
                                );
                              }}
                            </Form.Item>
                          </Space>
                        </Space>
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="批次管理" name="batch_enabled" valuePropName="checked">
                        <Switch checkedChildren="启用" unCheckedChildren="停用" />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="保质期(天)" name="shelf_life_days">
                        <InputNumber min={0} disabled={!batchEnabled} style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="禁售天数" name="forbid_sale_days">
                        <InputNumber min={0} disabled={!batchEnabled} style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item label="唯一码管理" name="serial_enabled" valuePropName="checked">
                        <Switch checkedChildren="启用" unCheckedChildren="停用" />
                      </Form.Item>
                    </Col>
                    <Col span={6}>
                      <Form.Item
                        label="回传平台"
                        name="serial_push_platform"
                        valuePropName="checked"
                      >
                        <Switch
                          checkedChildren="是"
                          unCheckedChildren="否"
                          disabled={!serialEnabled}
                        />
                      </Form.Item>
                    </Col>
                  </Row>
                  <ProductIndustrySections
                    industryProfile={industryProfile}
                    batchEnabled={Boolean(batchEnabled)}
                    serialEnabled={Boolean(serialEnabled)}
                    apparelComposer={
                      industryProfile === 'APPAREL' ? (
                        <ProductApparelSpecComposer
                          specs={specs}
                          onApply={(nextSpecs) => {
                            form.setFieldsValue({ specs: nextSpecs });
                          }}
                        />
                      ) : null
                    }
                  />
                  <Divider style={{ margin: '8px 0' }} />
                  <Form.List name="specs">
                    {(fields, { add, remove }) => (
                      <div className="products-specs-section">
                        <div className="products-specs-toolbar">
                          <Typography.Text strong>规格信息</Typography.Text>
                          <Button size="small" type="dashed" onClick={() => add({})}>
                            新增规格
                          </Button>
                        </div>
                        {fields.length === 0 ? (
                          <Empty description="暂无规格" />
                        ) : (
                          <div className="products-specs-table">
                            <div className="products-specs-header">
                              <Row className="products-specs-grid" wrap={false} gutter={8}>
                                <Col flex="140px">规格编号</Col>
                                <Col flex="160px">规格名称</Col>
                                <Col flex="120px">固定成本价</Col>
                                <Col flex="120px">颜色</Col>
                                <Col flex="120px">尺码</Col>
                                <Col flex="140px">成份</Col>
                                <Col flex="160px">条码</Col>
                                <Col flex="120px">重量</Col>
                                <Col flex="120px">单位</Col>
                                <Col flex="120px">第二单位</Col>
                                <Col flex="140px">标记</Col>
                                <Col flex="200px">备注</Col>
                                <Col flex="72px">操作</Col>
                              </Row>
                            </div>
                            <div className="products-specs-body">
                              {fields.map((field) => (
                                <div key={field.key} className="products-specs-row">
                                  <Row
                                    className="products-specs-grid"
                                    wrap={false}
                                    gutter={8}
                                    align="middle"
                                  >
                                    <Col flex="140px">
                                      <Form.Item
                                        {...field}
                                        name={[field.name, 'spec_code']}
                                        noStyle
                                      >
                                        <Input placeholder="规格编号" />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="160px">
                                      <Form.Item
                                        {...field}
                                        name={[field.name, 'spec_name']}
                                        noStyle
                                      >
                                        <Input placeholder="规格名称" />
                                      </Form.Item>
                                      {industryProfile === 'APPAREL' ? (
                                        <Typography.Text type="secondary" style={{ fontSize: 12 }}>
                                          自动预览：{getSpecNamePreview(specs[field.name])}
                                        </Typography.Text>
                                      ) : null}
                                    </Col>
                                    <Col flex="120px">
                                      <Form.Item {...field} name={[field.name, 'cost']} noStyle>
                                        <InputNumber min={0} style={{ width: '100%' }} />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="120px">
                                      <Form.Item {...field} name={[field.name, 'color']} noStyle>
                                        <Input placeholder="颜色" />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="120px">
                                      <Form.Item {...field} name={[field.name, 'size']} noStyle>
                                        <Input placeholder="尺码" />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="140px">
                                      <Form.Item
                                        {...field}
                                        name={[field.name, 'ingredient']}
                                        noStyle
                                      >
                                        <Input placeholder="成份" />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="160px">
                                      <Form.Item {...field} name={[field.name, 'barcode']} noStyle>
                                        <Input placeholder="条码" />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="120px">
                                      <Form.Item {...field} name={[field.name, 'weight']} noStyle>
                                        <InputNumber min={0} style={{ width: '100%' }} />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="120px">
                                      <Form.Item {...field} name={[field.name, 'unit']} noStyle>
                                        <Input placeholder="单位" />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="120px">
                                      <Form.Item
                                        {...field}
                                        name={[field.name, 'second_unit']}
                                        noStyle
                                      >
                                        <Input placeholder="第二单位" />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="140px">
                                      <Form.Item {...field} name={[field.name, 'tag']} noStyle>
                                        <Select
                                          allowClear
                                          showSearch
                                          options={specTagOptions.map((tag) => ({
                                            value: tag,
                                            label: tag,
                                          }))}
                                          onSearch={(value) => {
                                            if (!value) {
                                              return;
                                            }
                                            setSpecTagOptions((prev) =>
                                              prev.includes(value) ? prev : [...prev, value],
                                            );
                                          }}
                                        />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="200px">
                                      <Form.Item {...field} name={[field.name, 'remark']} noStyle>
                                        <Input placeholder="规格备注" />
                                      </Form.Item>
                                    </Col>
                                    <Col flex="72px">
                                      <Button
                                        type="link"
                                        size="small"
                                        danger
                                        onClick={() => remove(field.name)}
                                      >
                                        删除
                                      </Button>
                                    </Col>
                                  </Row>
                                </div>
                              ))}
                            </div>
                          </div>
                        )}
                      </div>
                    )}
                  </Form.List>
                </Form>
              ),
            },
            {
              key: 'warehouses',
              label: '存放仓库',
              children: editingProduct ? (
                <Space direction="vertical" size={8} style={{ width: '100%' }}>
                  <Card size="small" title="新增仓库">
                    <Form form={warehouseForm} layout="inline">
                      <Form.Item
                        name="warehouse_id"
                        rules={[{ required: true, message: '请选择仓库' }]}
                      >
                        <Select
                          placeholder="选择仓库"
                          options={warehouseOptions}
                          style={{ width: 240 }}
                        />
                      </Form.Item>
                      <Form.Item name="is_default" valuePropName="checked">
                        <Checkbox>设为默认</Checkbox>
                      </Form.Item>
                      <Form.Item name="allow_negative_stock" valuePropName="checked">
                        <Checkbox>允许负库存</Checkbox>
                      </Form.Item>
                      <Form.Item>
                        <Button
                          size="small"
                          type="primary"
                          onClick={handleAddWarehouse}
                          disabled={!canWrite}
                        >
                          添加
                        </Button>
                      </Form.Item>
                    </Form>
                  </Card>
                  <Table
                    size="small"
                    rowKey="warehouse_id"
                    columns={warehouseColumns}
                    dataSource={productWarehouses}
                    loading={warehouseQuery.isLoading || warehouseQuery.isFetching}
                    pagination={false}
                  />
                </Space>
              ) : (
                <Empty description="请先保存货品后配置仓库" />
              ),
            },
            {
              key: 'kits',
              label: '相关组合装',
              children: editingProduct ? (
                <Space direction="vertical" size={8} style={{ width: '100%' }}>
                  <Space size={8} style={{ justifyContent: 'space-between', width: '100%' }}>
                    <Typography.Text>包含该货品的组合装</Typography.Text>
                    <Button size="small" onClick={() => setKitModalOpen(true)}>
                      新增组合装
                    </Button>
                  </Space>
                  <Table
                    size="small"
                    rowKey="kit_sku_id"
                    columns={kitColumns}
                    dataSource={productKits}
                    loading={kitQuery.isLoading || kitQuery.isFetching}
                    pagination={false}
                  />
                </Space>
              ) : (
                <Empty description="请先保存货品后查看组合装" />
              ),
            },
            {
              key: 'logs',
              label: '日志',
              children: editingProduct ? (
                <Table
                  size="small"
                  rowKey="log_id"
                  columns={logColumns}
                  dataSource={productLogs}
                  loading={logQuery.isLoading || logQuery.isFetching}
                  pagination={false}
                  locale={{ emptyText: <Empty description="暂无日志" /> }}
                />
              ) : (
                <Empty description="请先保存货品后查看日志" />
              ),
            },
          ]}
        />
      </Modal>

      <Modal
        title="选择组合装SKU"
        open={kitModalOpen}
        onCancel={() => setKitModalOpen(false)}
        onOk={handleOpenKitEditor}
        okText="打开编辑"
        cancelText="取消"
      >
        <Select
          showSearch
          allowClear
          placeholder="搜索组合装SKU"
          value={selectedKitSku ?? undefined}
          loading={kitSkuLoading}
          options={kitSkuOptions.map((item) => ({
            value: item.sku_id,
            label: buildSkuLabel(item),
          }))}
          filterOption={false}
          onSearch={(value) => {
            setKitSkuKeyword(value);
            loadKitSkuOptions(value);
          }}
          onFocus={() => {
            if (!kitSkuOptions.length) {
              loadKitSkuOptions('');
            }
          }}
          onChange={(value) => setSelectedKitSku(value ?? null)}
          style={{ width: '100%' }}
        />
        {kitSkuKeyword ? (
          <Typography.Text type="secondary" style={{ display: 'block', marginTop: 8 }}>
            当前搜索：{kitSkuKeyword}
          </Typography.Text>
        ) : null}
      </Modal>
    </>
  );
};

