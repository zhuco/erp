import './tagDirectoryPage.css';

import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Alert,
  Button,
  Card,
  Descriptions,
  Drawer,
  Empty,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Segmented,
  Space,
  Switch,
  Table,
  Tabs,
  Tag,
  Typography,
  message,
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';

import type {
  OrderTagDefinition,
  OrderTagDefinitionCreatePayload,
} from '@/api/orderTagDefinitions';
import {
  createOrderTagDefinition,
  deleteOrderTagDefinition,
  listOrderTagDefinitions,
  updateOrderTagDefinition,
} from '@/api/orderTagDefinitions';
import type { SkuTagDefinition, SkuTagDefinitionCreatePayload } from '@/api/skuTagDefinitions';
import {
  createSkuTagDefinition,
  deleteSkuTagDefinition,
  listSkuTagDefinitions,
  updateSkuTagDefinition,
} from '@/api/skuTagDefinitions';
import { listSettings, upsertSetting } from '@/api/system';
import type { ErpDataGridColumn } from '@/components/ErpDataGrid';
import { ErpDataGridNext } from '@/components/ErpDataGridNext';
import {
  WorkbenchLeftPanelTemplate,
  WorkbenchPageTemplate,
  WorkbenchToolbar,
  type WorkbenchLeftStatsItem,
} from '@/components/page-templates';
import { UiModule04BottomTable, UiModule05DetailPanel } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { builtinOrderTags, type BuiltinOrderTag } from '@/features/orderTags';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { useLeftFilterModule } from '@/pages/orders/shared/useLeftFilterModule';

export type TagDirectoryTab = 'orders' | 'skus' | 'builtin';

type TagDirectoryWorkbenchProps = {
  activeTab?: TagDirectoryTab;
  fixedTab?: TagDirectoryTab;
  onTabChange?: (tab: TagDirectoryTab) => void;
  showTabs?: boolean;
  showDirectoryTitle?: boolean;
};

type OrderTagFormValues = {
  name: string;
  color?: string;
  priority?: number;
  status?: string;
  auto_apply?: boolean;
  conditions_json?: string;
};

type SkuTagFormValues = {
  name: string;
  color?: string;
  priority?: number;
  status?: string;
};

type TemplateOption = {
  label: string;
  value: string;
  payload: unknown;
};

type SearchState = Record<TagDirectoryTab, string>;
type PaginationState = Record<TagDirectoryTab, TablePaginationConfig>;
type KeyState = Record<TagDirectoryTab, string | null>;
type SelectionState = Record<TagDirectoryTab, string[]>;
type LeftFilterValue = {
  selectedFilterId: string;
  keyword: string;
};
type LeftFilterState = Record<TagDirectoryTab, LeftFilterValue>;
type BottomDetailRow = {
  key: string;
  field: string;
  value: string;
};
type BottomPanelConfig = {
  key: string;
  label: string;
  rows: BottomDetailRow[];
  emptyDescription: string;
  syncKey: string;
};
type AnyTagRow = OrderTagDefinition | SkuTagDefinition | BuiltinOrderTag;

const DEFAULT_PAGE_SIZE = 20;
const PAGE_SIZE_OPTIONS = ['20', '50', '100'];
const TEMPLATE_SETTING_KEY = 'order.tag.templates';

const TAB_LABELS: Record<TagDirectoryTab, string> = {
  orders: '订单记号',
  skus: 'SKU 记号',
  builtin: '内置记号',
};

const SEARCH_PLACEHOLDERS: Record<TagDirectoryTab, string> = {
  orders: '搜索订单记号名称 / 颜色 / 条件',
  skus: '搜索 SKU 记号名称 / 颜色',
  builtin: '搜索内置记号名称 / 编码 / 别名',
};

const LEFT_SCREEN_META: Record<TagDirectoryTab, { title: string; placeholder: string }> = {
  orders: { title: '订单记号补充筛选', placeholder: '按名称 / 颜色 / 条件摘要缩小左侧结果' },
  skus: { title: 'SKU 记号补充筛选', placeholder: '按名称 / 颜色缩小左侧结果' },
  builtin: { title: '内置记号补充筛选', placeholder: '按名称 / 编码 / 别名缩小左侧结果' },
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' },
];

const templateDefaults: TemplateOption[] = [
  { label: '高金额订单', value: 'high_amount', payload: [{ field_path: 'total_cent', op: '>=', value: 5000 }] },
  { label: '多 SKU 订单', value: 'multi_sku', payload: [{ field_path: 'sku_count', op: '>=', value: 5 }] },
  { label: '地址异常', value: 'address_invalid', payload: [{ field_path: 'address_invalid', op: '=', value: true }] },
];

const createDefaultSearchState = (): SearchState => ({ orders: '', skus: '', builtin: '' });
const createDefaultPagination = (): TablePaginationConfig => ({
  current: 1,
  pageSize: DEFAULT_PAGE_SIZE,
  pageSizeOptions: PAGE_SIZE_OPTIONS,
});
const createDefaultKeyState = (): KeyState => ({ orders: null, skus: null, builtin: null });
const createDefaultSelectionState = (): SelectionState => ({ orders: [], skus: [], builtin: [] });
const createDefaultLeftFilterValue = (): LeftFilterValue => ({ selectedFilterId: 'view:all', keyword: '' });
const createDefaultLeftFilterState = (): LeftFilterState => ({
  orders: createDefaultLeftFilterValue(),
  skus: createDefaultLeftFilterValue(),
  builtin: createDefaultLeftFilterValue(),
});

const trimText = (value?: string | null) => String(value ?? '').trim();
const normalizeKeyword = (value?: string | null) => trimText(value).toLowerCase();
const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');
const renderStatusTag = (status?: string | null) =>
  status === 'DISABLED' ? <Tag color="red">停用</Tag> : <Tag color="green">启用</Tag>;

const renderColorValue = (color?: string | null) =>
  color ? (
    <span className="tag-directory-color-chip">
      <span className="tag-directory-color-dot" style={{ backgroundColor: color }} />
      <span>{color}</span>
    </span>
  ) : (
    '-'
  );

const summarizeConditions = (value: unknown) => {
  if (Array.isArray(value)) return value.length > 0 ? `已配置 ${value.length} 条条件` : '未配置';
  if (value && typeof value === 'object') {
    const size = Object.keys(value).length;
    return size > 0 ? `已配置 ${size} 个字段` : '未配置';
  }
  return value ? '已配置' : '未配置';
};

const measureConditions = (value: unknown) => {
  if (Array.isArray(value)) return value.length;
  if (value && typeof value === 'object') return Object.keys(value).length;
  return trimText(String(value ?? '')) ? 1 : 0;
};

const matchesBuiltinKeyword = (tag: BuiltinOrderTag, keyword: string) =>
  !keyword ||
  [tag.name, tag.code, tag.description, ...(tag.aliases ?? [])].some((field) =>
    String(field).toLowerCase().includes(keyword),
  );

const stringifyConditions = (value: unknown) => JSON.stringify(value ?? [], null, 2);

const isTemplateOption = (value: unknown): value is TemplateOption => {
  if (!value || typeof value !== 'object') return false;
  const candidate = value as { label?: unknown; value?: unknown };
  return typeof candidate.label === 'string' && typeof candidate.value === 'string';
};

const patchPaginationState = (
  previous: PaginationState,
  tab: TagDirectoryTab,
  patch: Partial<TablePaginationConfig>,
): PaginationState => ({
  ...previous,
  [tab]: { ...previous[tab], ...patch, pageSizeOptions: PAGE_SIZE_OPTIONS },
});

const normalizeSelectionKeys = (keys: Array<string | number | null | undefined>) =>
  Array.from(new Set(keys.map((key) => trimText(String(key ?? ''))).filter(Boolean)));

const hasSameKeys = (left: string[], right: string[]) =>
  left.length === right.length && left.every((value, index) => value === right[index]);

const resolveTargetIds = (selected: string[], active: string | null) =>
  selected.length > 0 ? selected : active ? [active] : [];

const slicePageRows = <TRow,>(rows: TRow[], pagination: TablePaginationConfig) => {
  const current = Math.max(1, Number(pagination.current ?? 1));
  const pageSize = Math.max(1, Number(pagination.pageSize ?? DEFAULT_PAGE_SIZE));
  const start = (current - 1) * pageSize;
  return rows.slice(start, start + pageSize);
};

const buildStatsPieBackground = (items: WorkbenchLeftStatsItem[]) => {
  const totalValue = items.reduce((sum, item) => sum + item.value, 0) || 1;
  let offset = 0;
  const segments = items.map((item) => {
    const start = offset;
    offset += (item.value / totalValue) * 360;
    return `${item.color} ${start.toFixed(2)}deg ${offset.toFixed(2)}deg`;
  });
  if (offset < 360) segments.push(`#eef2f7 ${offset.toFixed(2)}deg 360deg`);
  return `conic-gradient(${segments.join(', ')})`;
};

const resolveTagRowKey = (tab: TagDirectoryTab, row: AnyTagRow) =>
  tab === 'builtin' ? (row as BuiltinOrderTag).key : (row as OrderTagDefinition | SkuTagDefinition).tag_id;

const buildTagLeftNodes = (tab: TagDirectoryTab, rows: AnyTagRow[]) => {
  if (tab === 'orders') {
    const orderRows = rows as OrderTagDefinition[];
    return [
      {
        key: 'group:overview',
        label: '目录视图',
        nodeType: 'group',
        count: orderRows.length,
        children: [
          { key: 'view:all', label: '全部订单记号', nodeType: 'tag', count: orderRows.length },
          {
            key: 'status:ACTIVE',
            label: '启用中',
            nodeType: 'status',
            count: orderRows.filter((row) => row.status !== 'DISABLED').length,
          },
          {
            key: 'status:DISABLED',
            label: '已停用',
            nodeType: 'status',
            count: orderRows.filter((row) => row.status === 'DISABLED').length,
          },
        ],
      },
      {
        key: 'group:auto',
        label: '自动能力',
        nodeType: 'group',
        count: orderRows.length,
        children: [
          {
            key: 'auto:yes',
            label: '自动打标',
            nodeType: 'tag',
            count: orderRows.filter((row) => row.auto_apply).length,
          },
          {
            key: 'auto:no',
            label: '人工打标',
            nodeType: 'tag',
            count: orderRows.filter((row) => !row.auto_apply).length,
          },
        ],
      },
    ];
  }

  if (tab === 'skus') {
    const skuRows = rows as SkuTagDefinition[];
    return [
      {
        key: 'group:overview',
        label: '目录视图',
        nodeType: 'group',
        count: skuRows.length,
        children: [
          { key: 'view:all', label: '全部 SKU 记号', nodeType: 'tag', count: skuRows.length },
          {
            key: 'status:ACTIVE',
            label: '启用中',
            nodeType: 'status',
            count: skuRows.filter((row) => row.status !== 'DISABLED').length,
          },
          {
            key: 'status:DISABLED',
            label: '已停用',
            nodeType: 'status',
            count: skuRows.filter((row) => row.status === 'DISABLED').length,
          },
        ],
      },
    ];
  }

  const builtinRows = rows as BuiltinOrderTag[];
  return [
    {
      key: 'group:overview',
      label: '目录视图',
      nodeType: 'group',
      count: builtinRows.length,
      children: [
        { key: 'view:all', label: '全部内置记号', nodeType: 'tag', count: builtinRows.length },
        {
          key: 'alias:yes',
          label: '带别名',
          nodeType: 'tag',
          count: builtinRows.filter((row) => (row.aliases?.length ?? 0) > 0).length,
        },
        {
          key: 'alias:no',
          label: '无别名',
          nodeType: 'tag',
          count: builtinRows.filter((row) => (row.aliases?.length ?? 0) === 0).length,
        },
      ],
    },
  ];
};

const filterTagRowsByLeftFilters = (tab: TagDirectoryTab, rows: AnyTagRow[], filters: LeftFilterValue) => {
  const keyword = normalizeKeyword(filters.keyword);

  return rows.filter((row) => {
    if (tab === 'orders') {
      const orderRow = row as OrderTagDefinition;
      if (filters.selectedFilterId === 'status:ACTIVE' && orderRow.status === 'DISABLED') return false;
      if (filters.selectedFilterId === 'status:DISABLED' && orderRow.status !== 'DISABLED') return false;
      if (filters.selectedFilterId === 'auto:yes' && !orderRow.auto_apply) return false;
      if (filters.selectedFilterId === 'auto:no' && orderRow.auto_apply) return false;
      if (!keyword) return true;
      return [orderRow.name, orderRow.color, summarizeConditions(orderRow.conditions_json)]
        .map((value) => String(value ?? '').toLowerCase())
        .some((value) => value.includes(keyword));
    }

    if (tab === 'skus') {
      const skuRow = row as SkuTagDefinition;
      if (filters.selectedFilterId === 'status:ACTIVE' && skuRow.status === 'DISABLED') return false;
      if (filters.selectedFilterId === 'status:DISABLED' && skuRow.status !== 'DISABLED') return false;
      if (!keyword) return true;
      return [skuRow.name, skuRow.color]
        .map((value) => String(value ?? '').toLowerCase())
        .some((value) => value.includes(keyword));
    }

    const builtinRow = row as BuiltinOrderTag;
    if (filters.selectedFilterId === 'alias:yes' && (builtinRow.aliases?.length ?? 0) === 0) return false;
    if (filters.selectedFilterId === 'alias:no' && (builtinRow.aliases?.length ?? 0) > 0) return false;
    if (!keyword) return true;
    return matchesBuiltinKeyword(builtinRow, keyword);
  });
};

export const TagDirectoryWorkbench = ({
  activeTab,
  fixedTab,
  onTabChange,
  showTabs = !fixedTab,
  showDirectoryTitle = true,
}: TagDirectoryWorkbenchProps) => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const featureFlag = useFeatureFlag('directory.tag_system');
  const canReadOrders = useCan('order_tags.read');
  const canWriteOrders = useCan('order_tags.write');
  const canReadSkus = useCan('sku_tags.read');
  const canWriteSkus = useCan('sku_tags.write');
  const [orderForm] = Form.useForm<OrderTagFormValues>();
  const [skuForm] = Form.useForm<SkuTagFormValues>();

  const [searchInputs, setSearchInputs] = useState<SearchState>(createDefaultSearchState);
  const [searchKeywords, setSearchKeywords] = useState<SearchState>(createDefaultSearchState);
  const [leftFilters, setLeftFilters] = useState<LeftFilterState>(createDefaultLeftFilterState);
  const [paginations, setPaginations] = useState<PaginationState>({
    orders: createDefaultPagination(),
    skus: createDefaultPagination(),
    builtin: createDefaultPagination(),
  });
  const [activeKeys, setActiveKeys] = useState<KeyState>(createDefaultKeyState);
  const [selectedKeys, setSelectedKeys] = useState<SelectionState>(createDefaultSelectionState);
  const [orderDrawerOpen, setOrderDrawerOpen] = useState(false);
  const [skuDrawerOpen, setSkuDrawerOpen] = useState(false);
  const [editingOrderTag, setEditingOrderTag] = useState<OrderTagDefinition | null>(null);
  const [editingSkuTag, setEditingSkuTag] = useState<SkuTagDefinition | null>(null);
  const [savingOrder, setSavingOrder] = useState(false);
  const [savingSku, setSavingSku] = useState(false);
  const [conditionsError, setConditionsError] = useState<string | null>(null);
  const [templateKey, setTemplateKey] = useState<string>();
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [templateName, setTemplateName] = useState('');
  const [templateSaving, setTemplateSaving] = useState(false);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const availableTabs = useMemo(() => {
    const tabs: TagDirectoryTab[] = [];
    if (canReadOrders) tabs.push('orders');
    if (canReadSkus) tabs.push('skus');
    tabs.push('builtin');
    return tabs;
  }, [canReadOrders, canReadSkus]);

  const resolvedTab = useMemo<TagDirectoryTab>(() => {
    if (fixedTab) return fixedTab;
    if (activeTab && availableTabs.includes(activeTab)) return activeTab;
    return availableTabs[0] ?? 'builtin';
  }, [activeTab, availableTabs, fixedTab]);

  useEffect(() => {
    if (!fixedTab && onTabChange && activeTab !== resolvedTab) onTabChange(resolvedTab);
  }, [activeTab, fixedTab, onTabChange, resolvedTab]);

  useEffect(() => {
    setActiveBottomTab('profile');
  }, [resolvedTab]);

  const currentPagination = paginations[resolvedTab];
  const currentSearchInput = searchInputs[resolvedTab];
  const currentFeatureEnabled = resolvedTab === 'builtin' || featureFlag.enabled;
  const currentLeftFilter = leftFilters[resolvedTab];

  // SECTION: queries
  const orderQuery = useQuery({
    queryKey: ['order-tag-definitions', tenantId, searchKeywords.orders],
    queryFn: () =>
      listOrderTagDefinitions(tenantId, {
        keyword: searchKeywords.orders || undefined,
        offset: 0,
        limit: 500,
      }),
    enabled: Boolean(tenantId) && canReadOrders && featureFlag.enabled,
    placeholderData: (previous) => previous,
  });

  const skuQuery = useQuery({
    queryKey: ['sku-tag-definitions', tenantId, searchKeywords.skus],
    queryFn: () =>
      listSkuTagDefinitions(tenantId, {
        keyword: searchKeywords.skus || undefined,
        offset: 0,
        limit: 500,
      }),
    enabled: Boolean(tenantId) && canReadSkus && featureFlag.enabled,
    placeholderData: (previous) => previous,
  });

  const templateQuery = useQuery({
    queryKey: ['order-tag-templates', tenantId],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: TEMPLATE_SETTING_KEY,
      }),
    enabled: Boolean(tenantId) && canReadOrders && featureFlag.enabled,
  });

  // SECTION: derived
  const templateOptions = useMemo(() => {
    const savedTemplates = templateQuery.data?.[0]?.value_json?.templates;
    const normalized = Array.isArray(savedTemplates) ? savedTemplates.filter(isTemplateOption) : [];
    const merged = [...templateDefaults];
    normalized.forEach((template) => {
      if (!merged.some((item) => item.value === template.value)) {
        merged.push(template);
      }
    });
    return merged;
  }, [templateQuery.data]);

  const orderSourceRows = orderQuery.data?.items ?? [];
  const skuSourceRows = skuQuery.data?.items ?? [];
  const builtinSourceRows = useMemo(() => {
    const keyword = normalizeKeyword(searchKeywords.builtin);
    return builtinOrderTags.filter((tag) => matchesBuiltinKeyword(tag, keyword));
  }, [searchKeywords.builtin]);

  const sourceRowsByTab = useMemo<Record<TagDirectoryTab, AnyTagRow[]>>(
    () => ({
      orders: canReadOrders && featureFlag.enabled ? orderSourceRows : [],
      skus: canReadSkus && featureFlag.enabled ? skuSourceRows : [],
      builtin: builtinSourceRows,
    }),
    [builtinSourceRows, canReadOrders, canReadSkus, featureFlag.enabled, orderSourceRows, skuSourceRows],
  );

  const currentSourceRows = sourceRowsByTab[resolvedTab];

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(() => {
    if (resolvedTab === 'orders') {
      const rows = currentSourceRows as OrderTagDefinition[];
      return [
        { key: 'total', label: '全部订单记号', value: rows.length, color: '#2f6fed' },
        { key: 'active', label: '启用', value: rows.filter((row) => row.status !== 'DISABLED').length, color: '#0f9d58' },
        { key: 'auto', label: '自动打标', value: rows.filter((row) => row.auto_apply).length, color: '#d48806' },
      ];
    }

    if (resolvedTab === 'skus') {
      const rows = currentSourceRows as SkuTagDefinition[];
      return [
        { key: 'total', label: '全部 SKU 记号', value: rows.length, color: '#2f6fed' },
        { key: 'active', label: '启用', value: rows.filter((row) => row.status !== 'DISABLED').length, color: '#0f9d58' },
        { key: 'disabled', label: '停用', value: rows.filter((row) => row.status === 'DISABLED').length, color: '#d0d7de' },
      ];
    }

    const rows = currentSourceRows as BuiltinOrderTag[];
    return [
      { key: 'total', label: '全部内置记号', value: rows.length, color: '#2f6fed' },
      { key: 'alias', label: '带别名', value: rows.filter((row) => (row.aliases?.length ?? 0) > 0).length, color: '#0f9d58' },
      { key: 'described', label: '带说明', value: rows.filter((row) => trimText(row.description).length > 0).length, color: '#d48806' },
    ];
  }, [currentSourceRows, resolvedTab]);

  const leftStatsPieBackground = useMemo(() => buildStatsPieBackground(leftStatsItems), [leftStatsItems]);

  const patchCurrentPagination = (tab: TagDirectoryTab, patch: Partial<TablePaginationConfig>) => {
    setPaginations((previous) => patchPaginationState(previous, tab, patch));
  };

  const patchCurrentLeftFilter = (patch: Partial<LeftFilterValue>) => {
    patchCurrentPagination(resolvedTab, { current: 1 });
    setLeftFilters((previous) => ({
      ...previous,
      [resolvedTab]: {
        ...previous[resolvedTab],
        ...patch,
      },
    }));
  };

  const resetCurrentLeftFilter = () => {
    patchCurrentPagination(resolvedTab, { current: 1 });
    setLeftFilters((previous) => ({
      ...previous,
      [resolvedTab]: createDefaultLeftFilterValue(),
    }));
  };

  const {
    filteredRows: currentDisplayRows,
    hasManualFilterActive,
    leftPanel,
  } = useLeftFilterModule<AnyTagRow, LeftFilterValue>({
    rows: currentSourceRows,
    filters: currentLeftFilter,
    onFiltersChange: (action) => {
      patchCurrentPagination(resolvedTab, { current: 1 });
      setLeftFilters((previous) => ({
        ...previous,
        [resolvedTab]: typeof action === 'function' ? action(previous[resolvedTab]) : action,
      }));
    },
    initialExpandedKeys: ['group:overview', 'group:auto'],
    emptyRowLabel: '没有可用筛选项',
    hasManualFilter: (filters) => filters.selectedFilterId !== 'view:all' || trimText(filters.keyword).length > 0,
    filterRows: (rows, filters) => filterTagRowsByLeftFilters(resolvedTab, rows, filters),
    buildCheckedKeys: (filters) => new Set([filters.selectedFilterId || 'view:all']),
    buildLeftNodes: (rows) => buildTagLeftNodes(resolvedTab, rows),
    reduceFiltersByNode: (previous, node) => ({
      ...previous,
      selectedFilterId: node.key,
    }),
    onBeforeLeafFilterApply: () => patchCurrentPagination(resolvedTab, { current: 1 }),
    renderPanel: ({ leftTab, onLeftTabChange, leftRowsForRender, onLeftRowClick, hasManualFilterActive }) => (
      <WorkbenchLeftPanelTemplate
        className="tag-directory-left-panel"
        dataTestId="tag-directory-left-panel"
        leftTab={leftTab}
        onLeftTabChange={onLeftTabChange}
        leftRowsForRender={leftRowsForRender}
        onLeftRowClick={onLeftRowClick}
        leftStatsItems={leftStatsItems}
        leftStatsPieBackground={leftStatsPieBackground}
        globalOrderTotal={currentSourceRows.length}
        statsScopeText={
          !currentFeatureEnabled
            ? '当前业务未启用记号系统'
            : hasManualFilterActive
              ? '口径：当前搜索结果与左侧筛选后的记号'
              : '口径：当前搜索结果中的全部记号'
        }
        screenContent={
          <Space direction="vertical" size={10} style={{ width: '100%' }}>
            <div className="tag-directory-screen-field">
              <Typography.Text type="secondary">{LEFT_SCREEN_META[resolvedTab].title}</Typography.Text>
              <Input
                size="small"
                allowClear
                value={currentLeftFilter.keyword}
                placeholder={LEFT_SCREEN_META[resolvedTab].placeholder}
                onChange={(event) => {
                  patchCurrentLeftFilter({ keyword: event.target.value });
                }}
              />
            </div>
            <div className="tag-directory-screen-field">
              <Typography.Text type="secondary">当前左侧视图</Typography.Text>
              <Typography.Text type="secondary">
                {currentLeftFilter.selectedFilterId === 'view:all' ? '全部记号' : `已切换到 ${currentLeftFilter.selectedFilterId}`}
              </Typography.Text>
            </div>
            <Button size="small" onClick={resetCurrentLeftFilter}>
              重置左侧筛选
            </Button>
          </Space>
        }
      />
    ),
  });

  const currentPageRows = useMemo(() => slicePageRows(currentDisplayRows, currentPagination), [currentDisplayRows, currentPagination]);

  useEffect(() => {
    setActiveKeys((previous) => {
      const previousKey = previous[resolvedTab];
      const nextKey =
        previousKey && currentPageRows.some((row) => resolveTagRowKey(resolvedTab, row) === previousKey)
          ? previousKey
          : currentPageRows[0]
            ? resolveTagRowKey(resolvedTab, currentPageRows[0])
            : null;
      return nextKey === previousKey ? previous : { ...previous, [resolvedTab]: nextKey };
    });

    setSelectedKeys((previous) => {
      const availableKeySet = new Set(currentPageRows.map((row) => resolveTagRowKey(resolvedTab, row)));
      const nextKeys = previous[resolvedTab].filter((key) => availableKeySet.has(key));
      return hasSameKeys(previous[resolvedTab], nextKeys) ? previous : { ...previous, [resolvedTab]: nextKeys };
    });
  }, [currentPageRows, resolvedTab]);

  const orderRows = resolvedTab === 'orders' ? (currentPageRows as OrderTagDefinition[]) : [];
  const skuRows = resolvedTab === 'skus' ? (currentPageRows as SkuTagDefinition[]) : [];
  const builtinRows = resolvedTab === 'builtin' ? (currentPageRows as BuiltinOrderTag[]) : [];

  const activeOrderTag = useMemo(
    () => (resolvedTab === 'orders' ? orderRows.find((item) => item.tag_id === activeKeys.orders) ?? null : null),
    [activeKeys.orders, orderRows, resolvedTab],
  );
  const activeSkuTag = useMemo(
    () => (resolvedTab === 'skus' ? skuRows.find((item) => item.tag_id === activeKeys.skus) ?? null : null),
    [activeKeys.skus, resolvedTab, skuRows],
  );
  const activeBuiltinTag = useMemo(
    () => (resolvedTab === 'builtin' ? builtinRows.find((item) => item.key === activeKeys.builtin) ?? null : null),
    [activeKeys.builtin, builtinRows, resolvedTab],
  );

  const currentSelectedCount = selectedKeys[resolvedTab].length;
  const currentTotal = currentDisplayRows.length;
  const hasActiveRecord = Boolean(activeOrderTag || activeSkuTag || activeBuiltinTag);
  const activeRecordId = activeOrderTag?.tag_id ?? activeSkuTag?.tag_id ?? activeBuiltinTag?.key ?? 'empty';
  const activeRecordName = activeOrderTag?.name ?? activeSkuTag?.name ?? activeBuiltinTag?.name ?? '-';
  // SECTION: actions
  const handleSearch = () => {
    setSearchKeywords((previous) => ({
      ...previous,
      [resolvedTab]: trimText(searchInputs[resolvedTab]),
    }));
    patchCurrentPagination(resolvedTab, { current: 1 });
  };

  const openCreateOrderDrawer = () => {
    setEditingOrderTag(null);
    setConditionsError(null);
    setTemplateKey(undefined);
    orderForm.resetFields();
    orderForm.setFieldsValue({
      priority: 0,
      status: 'ACTIVE',
      auto_apply: false,
      conditions_json: '[]',
    });
    setOrderDrawerOpen(true);
  };

  const openEditOrderDrawer = (record: OrderTagDefinition) => {
    setEditingOrderTag(record);
    setConditionsError(null);
    setTemplateKey(undefined);
    orderForm.setFieldsValue({
      name: record.name,
      color: record.color ?? undefined,
      priority: record.priority,
      status: record.status,
      auto_apply: record.auto_apply,
      conditions_json: stringifyConditions(record.conditions_json),
    });
    setOrderDrawerOpen(true);
  };

  const openCreateSkuDrawer = () => {
    setEditingSkuTag(null);
    skuForm.resetFields();
    skuForm.setFieldsValue({
      priority: 0,
      status: 'ACTIVE',
    });
    setSkuDrawerOpen(true);
  };

  const openEditSkuDrawer = (record: SkuTagDefinition) => {
    setEditingSkuTag(record);
    skuForm.setFieldsValue({
      name: record.name,
      color: record.color ?? undefined,
      priority: record.priority,
      status: record.status,
    });
    setSkuDrawerOpen(true);
  };

  const handleSaveOrder = async () => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    try {
      const values = await orderForm.validateFields();
      let conditions: Record<string, unknown> | unknown[] = [];
      if (trimText(values.conditions_json)) {
        try {
          conditions = JSON.parse(String(values.conditions_json));
          setConditionsError(null);
        } catch {
          setConditionsError('条件 JSON 格式不正确');
          return;
        }
      }
      const payload: OrderTagDefinitionCreatePayload = {
        name: trimText(values.name),
        color: trimText(values.color) || undefined,
        priority: values.priority ?? 0,
        status: values.status ?? 'ACTIVE',
        auto_apply: values.auto_apply ?? false,
        conditions_json: conditions,
      };
      setSavingOrder(true);
      if (editingOrderTag) {
        await updateOrderTagDefinition(tenantId, editingOrderTag.tag_id, payload);
        message.success('订单记号已更新');
      } else {
        await createOrderTagDefinition(tenantId, payload);
        message.success('订单记号已创建');
      }
      setOrderDrawerOpen(false);
      setEditingOrderTag(null);
      await queryClient.invalidateQueries({ queryKey: ['order-tag-definitions', tenantId] });
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSavingOrder(false);
    }
  };

  const handleSaveSku = async () => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    try {
      const values = await skuForm.validateFields();
      const payload: SkuTagDefinitionCreatePayload = {
        name: trimText(values.name),
        color: trimText(values.color) || undefined,
        priority: values.priority ?? 0,
        status: values.status ?? 'ACTIVE',
      };
      setSavingSku(true);
      if (editingSkuTag) {
        await updateSkuTagDefinition(tenantId, editingSkuTag.tag_id, payload);
        message.success('SKU 记号已更新');
      } else {
        await createSkuTagDefinition(tenantId, payload);
        message.success('SKU 记号已创建');
      }
      setSkuDrawerOpen(false);
      setEditingSkuTag(null);
      await queryClient.invalidateQueries({ queryKey: ['sku-tag-definitions', tenantId] });
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSavingSku(false);
    }
  };

  const handleSaveTemplate = async () => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    const name = trimText(templateName);
    if (!name) {
      message.warning('请输入模板名称');
      return;
    }
    const rawConditions = orderForm.getFieldValue('conditions_json') as string | undefined;
    if (!trimText(rawConditions)) {
      message.warning('请先填写条件 JSON');
      return;
    }
    let parsedPayload: unknown = [];
    try {
      parsedPayload = JSON.parse(String(rawConditions));
    } catch {
      message.warning('条件 JSON 格式不正确');
      return;
    }
    const currentSetting = templateQuery.data?.[0];
    const existingTemplates = Array.isArray(currentSetting?.value_json?.templates)
      ? (currentSetting.value_json.templates as unknown[]).filter(isTemplateOption)
      : [];
    setTemplateSaving(true);
    try {
      await upsertSetting(tenantId, {
        scope_type: 'TENANT',
        scope_id: null,
        key: TEMPLATE_SETTING_KEY,
        value: {
          templates: [
            ...existingTemplates,
            {
              label: name,
              value: `custom_${Date.now()}`,
              payload: parsedPayload,
            },
          ],
        },
        expected_version: currentSetting?.version,
      });
      message.success('条件模板已保存');
      setTemplateModalOpen(false);
      setTemplateName('');
      await queryClient.invalidateQueries({ queryKey: ['order-tag-templates', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '条件模板保存失败');
    } finally {
      setTemplateSaving(false);
    }
  };

  const handleDeleteOrders = async (ids: string[]) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (ids.length === 0) {
      message.warning('请先选择要删除的订单记号');
      return;
    }
    try {
      await Promise.all(ids.map((id) => deleteOrderTagDefinition(tenantId, id)));
      message.success(`已删除 ${ids.length} 个订单记号`);
      setSelectedKeys((previous) => ({ ...previous, orders: [] }));
      await queryClient.invalidateQueries({ queryKey: ['order-tag-definitions', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '订单记号删除失败');
    }
  };

  const handleDeleteSkus = async (ids: string[]) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    if (ids.length === 0) {
      message.warning('请先选择要删除的 SKU 记号');
      return;
    }
    try {
      await Promise.all(ids.map((id) => deleteSkuTagDefinition(tenantId, id)));
      message.success(`已删除 ${ids.length} 个 SKU 记号`);
      setSelectedKeys((previous) => ({ ...previous, skus: [] }));
      await queryClient.invalidateQueries({ queryKey: ['sku-tag-definitions', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : 'SKU 记号删除失败');
    }
  };

  // SECTION: columns
  const orderColumns = useMemo<ErpDataGridColumn<OrderTagDefinition>[]>(
    () => [
      { key: 'name', field: 'name', title: '记号名称', width: 180, minWidth: 160, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'color', field: 'color', title: '颜色', width: 140, minWidth: 120, renderCell: (value) => renderColorValue(value as string | null | undefined) },
      { key: 'priority', field: 'priority', title: '优先级', width: 100 },
      { key: 'auto_apply', field: 'auto_apply', title: '自动打标', width: 120, renderCell: (value) => (value ? <Tag color="blue">自动</Tag> : <Tag>人工</Tag>) },
      { key: 'conditions_json', field: 'conditions_json', title: '条件摘要', minWidth: 220, renderCell: (value) => summarizeConditions(value) },
      { key: 'status', field: 'status', title: '状态', width: 100, filter: 'agSetColumnFilter', filterParams: { values: ['ACTIVE', 'DISABLED'] }, renderCell: (value) => renderStatusTag(value as string | null | undefined) },
      { key: 'updated_at', field: 'updated_at', title: '更新时间', width: 160, renderCell: (_value, record) => formatDateTime(record.updated_at ?? record.created_at) },
    ],
    [],
  );

  const skuColumns = useMemo<ErpDataGridColumn<SkuTagDefinition>[]>(
    () => [
      { key: 'name', field: 'name', title: '记号名称', width: 180, minWidth: 160, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'color', field: 'color', title: '颜色', width: 140, minWidth: 120, renderCell: (value) => renderColorValue(value as string | null | undefined) },
      { key: 'priority', field: 'priority', title: '优先级', width: 100 },
      { key: 'status', field: 'status', title: '状态', width: 100, filter: 'agSetColumnFilter', filterParams: { values: ['ACTIVE', 'DISABLED'] }, renderCell: (value) => renderStatusTag(value as string | null | undefined) },
      { key: 'updated_at', field: 'updated_at', title: '更新时间', width: 160, renderCell: (_value, record) => formatDateTime(record.updated_at ?? record.created_at) },
    ],
    [],
  );

  const builtinColumns = useMemo<ErpDataGridColumn<BuiltinOrderTag>[]>(
    () => [
      { key: 'code', field: 'code', title: '编码', width: 110, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'name', field: 'name', title: '名称', width: 160, minWidth: 140, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'description', field: 'description', title: '说明', minWidth: 260 },
      { key: 'aliases', field: 'aliases', title: '别名', minWidth: 220, renderCell: (value) => (Array.isArray(value) && value.length > 0 ? value.join(' / ') : '-') },
      {
        key: 'preview',
        title: '样式预览',
        width: 140,
        renderCell: (_value, record) => (
          <Tag color={record.backgroundColor} style={{ color: record.textColor, borderColor: record.backgroundColor }}>
            {record.name}
          </Tag>
        ),
      },
    ],
    [],
  );

  // SECTION: panels
  const toolbarLeftContent = useMemo(() => {
    if (resolvedTab === 'orders') {
      const targetIds = resolveTargetIds(selectedKeys.orders, activeKeys.orders);
      const targetRecord = orderRows.find((item) => item.tag_id === targetIds[0]) ?? activeOrderTag;
      return (
        <Space size={8} wrap className="tag-directory-toolbar-actions">
          <Button type="primary" size="small" disabled={!currentFeatureEnabled || !canWriteOrders} onClick={openCreateOrderDrawer}>
            新建记号
          </Button>
          <Button
            size="small"
            disabled={!currentFeatureEnabled || !canWriteOrders || targetIds.length !== 1 || !targetRecord}
            onClick={() => {
              if (targetRecord) openEditOrderDrawer(targetRecord);
            }}
          >
            编辑
          </Button>
          <Button
            size="small"
            danger
            disabled={!currentFeatureEnabled || !canWriteOrders || targetIds.length === 0}
            onClick={() => {
              void Modal.confirm({
                title: `确认删除选中的 ${targetIds.length} 个订单记号吗？`,
                okText: '删除',
                cancelText: '取消',
                onOk: async () => handleDeleteOrders(targetIds),
              });
            }}
          >
            删除
          </Button>
        </Space>
      );
    }

    if (resolvedTab === 'skus') {
      const targetIds = resolveTargetIds(selectedKeys.skus, activeKeys.skus);
      const targetRecord = skuRows.find((item) => item.tag_id === targetIds[0]) ?? activeSkuTag;
      return (
        <Space size={8} wrap className="tag-directory-toolbar-actions">
          <Button type="primary" size="small" disabled={!currentFeatureEnabled || !canWriteSkus} onClick={openCreateSkuDrawer}>
            新建记号
          </Button>
          <Button
            size="small"
            disabled={!currentFeatureEnabled || !canWriteSkus || targetIds.length !== 1 || !targetRecord}
            onClick={() => {
              if (targetRecord) openEditSkuDrawer(targetRecord);
            }}
          >
            编辑
          </Button>
          <Button
            size="small"
            danger
            disabled={!currentFeatureEnabled || !canWriteSkus || targetIds.length === 0}
            onClick={() => {
              void Modal.confirm({
                title: `确认删除选中的 ${targetIds.length} 个 SKU 记号吗？`,
                okText: '删除',
                cancelText: '取消',
                onOk: async () => handleDeleteSkus(targetIds),
              });
            }}
          >
            删除
          </Button>
        </Space>
      );
    }

    return (
      <Space size={8} wrap className="tag-directory-toolbar-actions">
        <Button size="small" disabled>
          内置只读
        </Button>
        <Typography.Text type="secondary">
          内置记号由系统统一维护，当前页面提供统一布局、筛选和明细浏览能力。
        </Typography.Text>
      </Space>
    );
  }, [
    activeKeys.orders,
    activeKeys.skus,
    activeOrderTag,
    activeSkuTag,
    canWriteOrders,
    canWriteSkus,
    currentFeatureEnabled,
    orderRows,
    resolvedTab,
    selectedKeys.orders,
    selectedKeys.skus,
    skuRows,
  ]);

  const header = useMemo(() => {
    if (!showTabs && !showDirectoryTitle) return null;
    return (
      <div className="tag-directory-header">
        <div className="tag-directory-header-text">
          {showDirectoryTitle ? (
            <Typography.Title level={4} style={{ margin: 0 }}>
              记号目录
            </Typography.Title>
          ) : null}
          <Typography.Text type="secondary">
            订单记号、SKU 记号与内置记号统一运行在工业工作台结构下，差异只保留业务能力和数据来源。
          </Typography.Text>
        </div>
        {showTabs ? (
          <Segmented
            value={resolvedTab}
            options={availableTabs.map((tab) => ({ label: TAB_LABELS[tab], value: tab }))}
            onChange={(value) => {
              if (typeof value === 'string') onTabChange?.(value as TagDirectoryTab);
            }}
          />
        ) : null}
      </div>
    );
  }, [availableTabs, onTabChange, resolvedTab, showDirectoryTitle, showTabs]);

  const rightPanel = useMemo(() => {
    if (!currentFeatureEnabled && resolvedTab !== 'builtin') {
      return (
        <div className="tag-directory-right-panel">
          <Card size="small" title="工作台说明">
            <Typography.Text type="secondary">
              {featureFlag.reason || '当前租户尚未启用记号系统，请先完成系统配置。'}
            </Typography.Text>
          </Card>
        </div>
      );
    }

    if (resolvedTab === 'orders') {
      if (!activeOrderTag) {
        return (
          <div className="tag-directory-right-panel">
            <Card size="small" title="订单记号详情">
              <Typography.Text type="secondary">
                请选择一条订单记号，右侧详情与底部明细会自动同步到当前记录。
              </Typography.Text>
            </Card>
          </div>
        );
      }
      return (
        <div className="tag-directory-right-panel">
          <UiModule05DetailPanel
            defaultActiveKeys={['detail', 'rules', 'guide']}
            panels={[
              {
                key: 'detail',
                label: '记号详情',
                children: (
                  <Descriptions column={1} size="small">
                    <Descriptions.Item label="记号名称">{activeOrderTag.name}</Descriptions.Item>
                    <Descriptions.Item label="颜色">{renderColorValue(activeOrderTag.color)}</Descriptions.Item>
                    <Descriptions.Item label="优先级">{activeOrderTag.priority}</Descriptions.Item>
                    <Descriptions.Item label="状态">{renderStatusTag(activeOrderTag.status)}</Descriptions.Item>
                    <Descriptions.Item label="自动打标">{activeOrderTag.auto_apply ? <Tag color="blue">自动</Tag> : <Tag>人工</Tag>}</Descriptions.Item>
                    <Descriptions.Item label="更新时间">{formatDateTime(activeOrderTag.updated_at ?? activeOrderTag.created_at)}</Descriptions.Item>
                  </Descriptions>
                ),
              },
              {
                key: 'rules',
                label: '条件预览',
                children: <pre className="tag-directory-json-block">{stringifyConditions(activeOrderTag.conditions_json)}</pre>,
              },
              {
                key: 'guide',
                label: '工作台约束',
                children: (
                  <Space direction="vertical" size={10} style={{ width: '100%' }}>
                    <Typography.Text type="secondary">
                      当前页面已经切换到与订单审核一致的工业工作台结构：左侧筛选、紧凑主表、底部明细、右侧详情同时生效。
                    </Typography.Text>
                    <Typography.Text type="secondary">
                      主表仅承载检索和选择，新增、编辑、删除等动作统一收敛到上方工具栏，避免行内操作打断表格节奏。
                    </Typography.Text>
                  </Space>
                ),
              },
            ]}
          />
        </div>
      );
    }

    if (resolvedTab === 'skus') {
      if (!activeSkuTag) {
        return (
          <div className="tag-directory-right-panel">
            <Card size="small" title="SKU 记号详情">
              <Typography.Text type="secondary">
                请选择一条 SKU 记号，右侧详情与底部明细会自动同步到当前记录。
              </Typography.Text>
            </Card>
          </div>
        );
      }
      return (
        <div className="tag-directory-right-panel">
          <UiModule05DetailPanel
            defaultActiveKeys={['detail', 'guide']}
            panels={[
              {
                key: 'detail',
                label: '记号详情',
                children: (
                  <Descriptions column={1} size="small">
                    <Descriptions.Item label="记号名称">{activeSkuTag.name}</Descriptions.Item>
                    <Descriptions.Item label="颜色">{renderColorValue(activeSkuTag.color)}</Descriptions.Item>
                    <Descriptions.Item label="优先级">{activeSkuTag.priority}</Descriptions.Item>
                    <Descriptions.Item label="状态">{renderStatusTag(activeSkuTag.status)}</Descriptions.Item>
                    <Descriptions.Item label="更新时间">{formatDateTime(activeSkuTag.updated_at ?? activeSkuTag.created_at)}</Descriptions.Item>
                  </Descriptions>
                ),
              },
              {
                key: 'guide',
                label: '工作台约束',
                children: (
                  <Space direction="vertical" size={10} style={{ width: '100%' }}>
                    <Typography.Text type="secondary">
                      SKU 记号目录与订单记号目录复用同一套工业工作台骨架，布局和交互节奏保持一致。
                    </Typography.Text>
                    <Typography.Text type="secondary">
                      页面差异仅保留在按钮权限、主表字段和数据源，便于后续将更多目录页收敛到同一模板。
                    </Typography.Text>
                  </Space>
                ),
              },
            ]}
          />
        </div>
      );
    }

    if (!activeBuiltinTag) {
      return (
        <div className="tag-directory-right-panel">
          <Card size="small" title="内置记号详情">
            <Typography.Text type="secondary">
              请选择一条内置记号，右侧详情与底部明细会自动同步到当前记录。
            </Typography.Text>
          </Card>
        </div>
      );
    }

    return (
      <div className="tag-directory-right-panel">
        <UiModule05DetailPanel
          defaultActiveKeys={['detail', 'preview', 'guide']}
          panels={[
            {
              key: 'detail',
              label: '记号详情',
              children: (
                <Descriptions column={1} size="small">
                  <Descriptions.Item label="编码">{activeBuiltinTag.code}</Descriptions.Item>
                  <Descriptions.Item label="名称">{activeBuiltinTag.name}</Descriptions.Item>
                  <Descriptions.Item label="说明">{activeBuiltinTag.description}</Descriptions.Item>
                  <Descriptions.Item label="别名数量">{activeBuiltinTag.aliases?.length ?? 0}</Descriptions.Item>
                </Descriptions>
              ),
            },
            {
              key: 'preview',
              label: '样式预览',
              children: (
                <Space direction="vertical" size={10}>
                  <Tag color={activeBuiltinTag.backgroundColor} style={{ color: activeBuiltinTag.textColor, borderColor: activeBuiltinTag.backgroundColor }}>
                    {activeBuiltinTag.name}
                  </Tag>
                  <Typography.Text type="secondary">别名：{activeBuiltinTag.aliases?.join(' / ') || '无'}</Typography.Text>
                </Space>
              ),
            },
            {
              key: 'guide',
              label: '工作台约束',
              children: (
                <Space direction="vertical" size={10} style={{ width: '100%' }}>
                  <Typography.Text type="secondary">
                    内置记号保持只读，但同样纳入统一的左筛选、主表、底部明细和右侧详情结构。
                  </Typography.Text>
                  <Typography.Text type="secondary">
                    这样用户在不同目录页之间切换时，布局认知和操作路径不会发生变化。
                  </Typography.Text>
                </Space>
              ),
            },
          ]}
        />
      </div>
    );
  }, [activeBuiltinTag, activeOrderTag, activeSkuTag, currentFeatureEnabled, featureFlag.reason, resolvedTab]);

  const bottomDetailColumns = useMemo(
    () => [
      { title: '字段', dataIndex: 'field', key: 'field', width: 180 },
      { title: '值', dataIndex: 'value', key: 'value', width: 360 },
    ],
    [],
  );

  const orderProfileRows = useMemo<BottomDetailRow[]>(
    () =>
      activeOrderTag
        ? [
            { key: 'name', field: '记号名称', value: activeOrderTag.name },
            { key: 'color', field: '颜色', value: trimText(activeOrderTag.color) || '-' },
            { key: 'priority', field: '优先级', value: String(activeOrderTag.priority) },
            { key: 'status', field: '状态', value: activeOrderTag.status === 'DISABLED' ? '停用' : '启用' },
            { key: 'auto_apply', field: '自动打标', value: activeOrderTag.auto_apply ? '是' : '否' },
          ]
        : [],
    [activeOrderTag],
  );

  const orderRuleRows = useMemo<BottomDetailRow[]>(
    () =>
      activeOrderTag
        ? [
            { key: 'conditions_summary', field: '条件摘要', value: summarizeConditions(activeOrderTag.conditions_json) },
            { key: 'conditions_count', field: '条件数量', value: String(measureConditions(activeOrderTag.conditions_json)) },
            { key: 'template_support', field: '模板能力', value: '支持租户级条件模板复用' },
            { key: 'current_search', field: '当前检索关键词', value: trimText(searchKeywords.orders) || '-' },
          ]
        : [],
    [activeOrderTag, searchKeywords.orders],
  );

  const orderTimelineRows = useMemo<BottomDetailRow[]>(
    () =>
      activeOrderTag
        ? [
            { key: 'created_at', field: '创建时间', value: formatDateTime(activeOrderTag.created_at) },
            { key: 'updated_at', field: '更新时间', value: formatDateTime(activeOrderTag.updated_at ?? activeOrderTag.created_at) },
            { key: 'selected_count', field: '当前勾选数量', value: String(selectedKeys.orders.length) },
          ]
        : [],
    [activeOrderTag, selectedKeys.orders.length],
  );

  const skuProfileRows = useMemo<BottomDetailRow[]>(
    () =>
      activeSkuTag
        ? [
            { key: 'name', field: '记号名称', value: activeSkuTag.name },
            { key: 'color', field: '颜色', value: trimText(activeSkuTag.color) || '-' },
            { key: 'priority', field: '优先级', value: String(activeSkuTag.priority) },
            { key: 'status', field: '状态', value: activeSkuTag.status === 'DISABLED' ? '停用' : '启用' },
          ]
        : [],
    [activeSkuTag],
  );

  const skuPolicyRows = useMemo<BottomDetailRow[]>(
    () =>
      activeSkuTag
        ? [
            { key: 'maintain_mode', field: '维护方式', value: canWriteSkus ? '通过工具栏统一维护' : '只读' },
            { key: 'current_search', field: '当前检索关键词', value: trimText(searchKeywords.skus) || '-' },
            { key: 'selected_count', field: '当前勾选数量', value: String(selectedKeys.skus.length) },
          ]
        : [],
    [activeSkuTag, canWriteSkus, searchKeywords.skus, selectedKeys.skus.length],
  );

  const skuTimelineRows = useMemo<BottomDetailRow[]>(
    () =>
      activeSkuTag
        ? [
            { key: 'created_at', field: '创建时间', value: formatDateTime(activeSkuTag.created_at) },
            { key: 'updated_at', field: '更新时间', value: formatDateTime(activeSkuTag.updated_at ?? activeSkuTag.created_at) },
            { key: 'data_scope', field: '数据范围', value: 'SKU 记号主数据目录' },
          ]
        : [],
    [activeSkuTag],
  );

  const builtinProfileRows = useMemo<BottomDetailRow[]>(
    () =>
      activeBuiltinTag
        ? [
            { key: 'code', field: '编码', value: activeBuiltinTag.code },
            { key: 'name', field: '名称', value: activeBuiltinTag.name },
            { key: 'description', field: '说明', value: activeBuiltinTag.description },
          ]
        : [],
    [activeBuiltinTag],
  );

  const builtinAliasRows = useMemo<BottomDetailRow[]>(
    () =>
      activeBuiltinTag
        ? (activeBuiltinTag.aliases ?? []).map((alias, index) => ({
            key: `alias-${alias}-${index}`,
            field: `别名 ${index + 1}`,
            value: alias,
          }))
        : [],
    [activeBuiltinTag],
  );

  const builtinOpsRows = useMemo<BottomDetailRow[]>(
    () =>
      activeBuiltinTag
        ? [
            { key: 'source', field: '数据来源', value: '系统内置静态能力' },
            { key: 'edit_mode', field: '编辑方式', value: '只读' },
            { key: 'current_search', field: '当前检索关键词', value: trimText(searchKeywords.builtin) || '-' },
          ]
        : [],
    [activeBuiltinTag, searchKeywords.builtin],
  );

  const currentBottomPanelConfigs = useMemo<BottomPanelConfig[]>(() => {
    if (resolvedTab === 'orders') {
      return [
        { key: 'profile', label: '基础资料', rows: orderProfileRows, emptyDescription: '暂无基础资料', syncKey: `tag-order-profile-${activeRecordId}-${orderProfileRows.length}` },
        { key: 'rules', label: '规则明细', rows: orderRuleRows, emptyDescription: '暂无规则明细', syncKey: `tag-order-rules-${activeRecordId}-${orderRuleRows.length}` },
        { key: 'timeline', label: '维护轨迹', rows: orderTimelineRows, emptyDescription: '暂无维护轨迹', syncKey: `tag-order-timeline-${activeRecordId}-${orderTimelineRows.length}` },
      ];
    }
    if (resolvedTab === 'skus') {
      return [
        { key: 'profile', label: '基础资料', rows: skuProfileRows, emptyDescription: '暂无基础资料', syncKey: `tag-sku-profile-${activeRecordId}-${skuProfileRows.length}` },
        { key: 'ops', label: '操作约束', rows: skuPolicyRows, emptyDescription: '暂无操作约束', syncKey: `tag-sku-ops-${activeRecordId}-${skuPolicyRows.length}` },
        { key: 'timeline', label: '维护轨迹', rows: skuTimelineRows, emptyDescription: '暂无维护轨迹', syncKey: `tag-sku-timeline-${activeRecordId}-${skuTimelineRows.length}` },
      ];
    }
    return [
      { key: 'profile', label: '基础资料', rows: builtinProfileRows, emptyDescription: '暂无基础资料', syncKey: `tag-builtin-profile-${activeRecordId}-${builtinProfileRows.length}` },
      { key: 'aliases', label: '别名映射', rows: builtinAliasRows, emptyDescription: '暂无别名映射', syncKey: `tag-builtin-aliases-${activeRecordId}-${builtinAliasRows.length}` },
      { key: 'ops', label: '维护约束', rows: builtinOpsRows, emptyDescription: '暂无维护约束', syncKey: `tag-builtin-ops-${activeRecordId}-${builtinOpsRows.length}` },
    ];
  }, [
    activeRecordId,
    builtinAliasRows,
    builtinOpsRows,
    builtinProfileRows,
    orderProfileRows,
    orderRuleRows,
    orderTimelineRows,
    resolvedTab,
    skuPolicyRows,
    skuProfileRows,
    skuTimelineRows,
  ]);

  const bottomPanel = useMemo(
    () => (
      <Tabs
        size="small"
        type="card"
        activeKey={activeBottomTab}
        onChange={setActiveBottomTab}
        items={currentBottomPanelConfigs.map((config) => ({
          key: config.key,
          label: `${config.label} (${config.rows.length})`,
          children: hasActiveRecord ? (
            <UiModule04BottomTable syncKey={config.syncKey}>
              <Table<BottomDetailRow>
                size="small"
                rowKey="key"
                columns={bottomDetailColumns}
                dataSource={config.rows}
                pagination={false}
                scroll={{ x: 'max-content' }}
                locale={{ emptyText: <Empty description={config.emptyDescription} /> }}
              />
            </UiModule04BottomTable>
          ) : (
            <Empty description="请选择一条记号记录查看底部明细" />
          ),
        }))}
      />
    ),
    [activeBottomTab, bottomDetailColumns, currentBottomPanelConfigs, hasActiveRecord],
  );

  const rightPanelSyncKey = useMemo(
    () =>
      JSON.stringify({
        resolvedTab,
        activeRecordId,
        activeRecordName,
      }),
    [activeRecordId, activeRecordName, resolvedTab],
  );

  const tableContent = useMemo(() => {
    if (!currentFeatureEnabled) {
      return (
        <Alert
          message="记号系统未启用"
          description={featureFlag.reason || '请先在系统设置中启用记号系统。'}
          type="warning"
          showIcon
        />
      );
    }

    if (resolvedTab === 'orders') {
      return (
        <div className="tag-directory-table-shell">
          <ErpDataGridNext<OrderTagDefinition>
            rows={orderRows}
            columns={orderColumns}
            rowKey={(record) => record.tag_id}
            gridPreset="industrial-review-compact"
            columnSettingsKey="tags.order-directory.main-grid"
            dataTestId="tag-directory-orders-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            selectedRowKeys={selectedKeys.orders}
            activeRowKey={activeKeys.orders}
            onSelectionChange={(keys) => {
              setSelectedKeys((previous) => ({
                ...previous,
                orders: normalizeSelectionKeys(keys as Array<string | number | null | undefined>),
              }));
            }}
            onActiveRowChange={(record) => {
              setActiveKeys((previous) => ({ ...previous, orders: record.tag_id }));
            }}
            onRowDoubleClick={(record) => {
              if (canWriteOrders) openEditOrderDrawer(record);
            }}
            emptyText="暂无订单记号数据"
            loading={orderQuery.isLoading || orderQuery.isFetching}
            error={orderQuery.error}
            errorTitle="订单记号加载失败"
            onRetry={() => {
              void orderQuery.refetch();
            }}
          />
        </div>
      );
    }

    if (resolvedTab === 'skus') {
      return (
        <div className="tag-directory-table-shell">
          <ErpDataGridNext<SkuTagDefinition>
            rows={skuRows}
            columns={skuColumns}
            rowKey={(record) => record.tag_id}
            gridPreset="industrial-review-compact"
            columnSettingsKey="tags.sku-directory.main-grid"
            dataTestId="tag-directory-skus-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            selectedRowKeys={selectedKeys.skus}
            activeRowKey={activeKeys.skus}
            onSelectionChange={(keys) => {
              setSelectedKeys((previous) => ({
                ...previous,
                skus: normalizeSelectionKeys(keys as Array<string | number | null | undefined>),
              }));
            }}
            onActiveRowChange={(record) => {
              setActiveKeys((previous) => ({ ...previous, skus: record.tag_id }));
            }}
            onRowDoubleClick={(record) => {
              if (canWriteSkus) openEditSkuDrawer(record);
            }}
            emptyText="暂无 SKU 记号数据"
            loading={skuQuery.isLoading || skuQuery.isFetching}
            error={skuQuery.error}
            errorTitle="SKU 记号加载失败"
            onRetry={() => {
              void skuQuery.refetch();
            }}
          />
        </div>
      );
    }

    return (
      <div className="tag-directory-table-shell">
        <ErpDataGridNext<BuiltinOrderTag>
          rows={builtinRows}
          columns={builtinColumns}
          rowKey={(record) => record.key}
          gridPreset="industrial-review-compact"
          columnSettingsKey="tags.builtin-directory.main-grid"
          dataTestId="tag-directory-builtin-grid"
          showColumnSettings
          showSelectionColumn
          selectionCellMode="custom"
          selectionHeaderMode="allInvert"
          selectedRowKeys={selectedKeys.builtin}
          activeRowKey={activeKeys.builtin}
          onSelectionChange={(keys) => {
            setSelectedKeys((previous) => ({
              ...previous,
              builtin: normalizeSelectionKeys(keys as Array<string | number | null | undefined>),
            }));
          }}
          onActiveRowChange={(record) => {
            setActiveKeys((previous) => ({ ...previous, builtin: record.key }));
          }}
          emptyText="暂无内置记号数据"
        />
      </div>
    );
  }, [
    activeKeys.builtin,
    activeKeys.orders,
    activeKeys.skus,
    builtinColumns,
    builtinRows,
    canWriteOrders,
    canWriteSkus,
    currentFeatureEnabled,
    featureFlag.reason,
    orderColumns,
    orderQuery.error,
    orderQuery.isFetching,
    orderQuery.isLoading,
    orderRows,
    resolvedTab,
    selectedKeys.builtin,
    selectedKeys.orders,
    selectedKeys.skus,
    skuColumns,
    skuQuery.error,
    skuQuery.isFetching,
    skuQuery.isLoading,
    skuRows,
  ]);

  return (
    <WorkbenchPageTemplate
      pageTestId="tag-directory-workbench-page"
      className="order-review-page tag-directory-page"
      leftPanel={leftPanel}
      leftPanelVisible={true}
      header={header}
      toolbar={
        <WorkbenchToolbar
          leftContent={toolbarLeftContent}
          searchValue={currentSearchInput}
          onSearchChange={(value) => {
            setSearchInputs((previous) => ({ ...previous, [resolvedTab]: value }));
          }}
          onSearch={handleSearch}
          searchPlaceholder={SEARCH_PLACEHOLDERS[resolvedTab]}
          onExport={() => message.info('记号目录导出能力待接入')}
          onPrint={() => message.info('记号目录打印能力待接入')}
          onHelp={() => message.info('记号目录帮助文档待接入')}
        />
      }
      tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
      tableContent={tableContent}
      footerSummary={
        <>
          <span>{`${TAB_LABELS[resolvedTab]}：共 ${currentTotal} 条，本页 ${currentPageRows.length} 条，已选 ${currentSelectedCount} 条`}</span>
          <span>{`当前焦点：${activeRecordName}`}</span>
        </>
      }
      footerSyncKey={`tag-directory-${resolvedTab}-${currentTotal}-${currentPagination.current ?? 1}-${currentPagination.pageSize ?? DEFAULT_PAGE_SIZE}`}
      total={currentTotal}
      pagination={currentPagination}
      onPaginationChange={(nextPagination) => {
        patchCurrentPagination(resolvedTab, nextPagination);
      }}
      showFooterScrollbarRow={true}
      rightPanel={rightPanel}
      rightPanelVisible={true}
      rightPanelSyncKey={rightPanelSyncKey}
      bottomPanel={bottomPanel}
      bottomPanelVisible={true}
      overlays={
        <>
          <Drawer
            title={editingOrderTag ? '编辑订单记号' : '新建订单记号'}
            open={orderDrawerOpen}
            onClose={() => setOrderDrawerOpen(false)}
            width={560}
            destroyOnHidden
            footer={
              <Space>
                <Button onClick={() => setOrderDrawerOpen(false)}>取消</Button>
                <Button type="primary" loading={savingOrder} onClick={() => void handleSaveOrder()}>
                  保存
                </Button>
              </Space>
            }
          >
            <Form form={orderForm} layout="vertical">
              <Form.Item name="name" label="记号名称" rules={[{ required: true, message: '请输入记号名称' }]}>
                <Input placeholder="例如：高金额订单" />
              </Form.Item>
              <Form.Item name="color" label="颜色">
                <Input placeholder="例如：#1677ff" />
              </Form.Item>
              <Form.Item name="priority" label="优先级">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
              <Form.Item name="auto_apply" label="自动打标" valuePropName="checked">
                <Switch checkedChildren="开启" unCheckedChildren="关闭" />
              </Form.Item>
              <Form.Item label="条件模板">
                <Space style={{ width: '100%' }} align="start">
                  <Select
                    allowClear
                    value={templateKey}
                    style={{ flex: 1 }}
                    placeholder="选择模板快速填充条件"
                    options={templateOptions.map((item) => ({ label: item.label, value: item.value }))}
                    onChange={(value) => {
                      if (!value) {
                        setTemplateKey(undefined);
                        return;
                      }
                      const matched = templateOptions.find((item) => item.value === value);
                      if (matched) {
                        orderForm.setFieldsValue({ conditions_json: stringifyConditions(matched.payload) });
                        setConditionsError(null);
                        setTemplateKey(value);
                      }
                    }}
                  />
                  <Button onClick={() => setTemplateModalOpen(true)}>保存模板</Button>
                </Space>
              </Form.Item>
              <Form.Item
                name="conditions_json"
                label="条件 JSON"
                extra="用于定义自动打标条件，格式与订单规则字段保持一致。"
                validateStatus={conditionsError ? 'error' : ''}
                help={conditionsError ?? undefined}
              >
                <Input.TextArea rows={8} placeholder='[{"field_path":"total_cent","op":">=","value":5000}]' />
              </Form.Item>
              <Form.Item name="status" label="状态" initialValue="ACTIVE">
                <Select options={statusOptions} />
              </Form.Item>
            </Form>
          </Drawer>

          <Drawer
            title={editingSkuTag ? '编辑 SKU 记号' : '新建 SKU 记号'}
            open={skuDrawerOpen}
            onClose={() => setSkuDrawerOpen(false)}
            width={520}
            destroyOnHidden
            footer={
              <Space>
                <Button onClick={() => setSkuDrawerOpen(false)}>取消</Button>
                <Button type="primary" loading={savingSku} onClick={() => void handleSaveSku()}>
                  保存
                </Button>
              </Space>
            }
          >
            <Form form={skuForm} layout="vertical">
              <Form.Item name="name" label="记号名称" rules={[{ required: true, message: '请输入记号名称' }]}>
                <Input placeholder="例如：禁售 SKU" />
              </Form.Item>
              <Form.Item name="color" label="颜色">
                <Input placeholder="例如：#13c2c2" />
              </Form.Item>
              <Form.Item name="priority" label="优先级">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
              <Form.Item name="status" label="状态" initialValue="ACTIVE">
                <Select options={statusOptions} />
              </Form.Item>
            </Form>
          </Drawer>

          <Modal
            title="保存条件模板"
            open={templateModalOpen}
            okText="保存"
            cancelText="取消"
            confirmLoading={templateSaving}
            onOk={() => void handleSaveTemplate()}
            onCancel={() => {
              setTemplateModalOpen(false);
              setTemplateName('');
            }}
          >
            <Input
              value={templateName}
              onChange={(event) => setTemplateName(event.target.value)}
              placeholder="请输入模板名称"
            />
          </Modal>
        </>
      }
    />
  );
};
