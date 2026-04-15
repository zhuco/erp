import {
  Alert,
  Button,
  Card,
  Checkbox,
  Col,
  DatePicker,
  Drawer,
  Dropdown,
  Form,
  Input,
  Modal,
  Popconfirm,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  ShopProductAutoMatchPayload,
  ShopProductCreateSkuPayload,
  ShopProductDownloadPayload,
  ShopProductDeleteMatchPayload,
  ShopProductMatchFailure,
  ShopProductManualMatchPayload,
  ShopProductSku,
  ShopProductSkuListQuery,
  ShopProductTask
} from '@/api/shopProducts';
import {
  autoMatchShopProducts,
  createShopProductSkus,
  deleteShopProductMatches,
  downloadShopProducts,
  manualMatchShopProducts
} from '@/api/shopProducts';
import { listMappingRules } from '@/api/mappingRules';
import { useShopProductSkus, useShopProductTasks, useSkusList } from '@/api/hooks';
import { listShops } from '@/api/shops';
import { listWarehouses } from '@/api/warehouses';
import { PlatformTag } from '@/components/PlatformTag';
import { ServerTable } from '@/components/ServerTable';
import { WorkbenchListPage } from '@/components/workbench/WorkbenchListPage';
import { UiModule02ToolbarRight } from '@/components/ui-modules';
import { platformLabels } from '@/features/platform';
import type { PlatformCode } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type ShopProductStatusFilter = 'ACTIVE' | 'INACTIVE' | 'ALL';

type FilterValues = {
  platform?: PlatformCode;
  shop_ids?: string[];
  product_type?: string;
  status?: ShopProductStatusFilter;
  keyword?: string;
  match_status?: string;
  mapping_status?: string;
  conflict_reason?: string;
  match_source?: string;
  updated_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type DownloadFormValues = {
  platform?: PlatformCode;
  shop_ids?: string[];
  product_type?: string;
  status?: ShopProductStatusFilter;
  keyword?: string;
  updated_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type CreateFormValues = {
  import_price?: boolean;
  import_image?: boolean;
  import_stock?: boolean;
  warehouse_id?: string;
};

type ManualFormValues = {
  sku_id?: string;
};

export type ShopProductDownloadPageProps = {
  title?: string;
};

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : (value / 100).toFixed(2);

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const statusOptions = [
  { label: '出售中', value: 'ACTIVE' },
  { label: '已下架', value: 'INACTIVE' },
  { label: '全部', value: 'ALL' }
];

const matchStatusOptions = [
  { label: '已匹配', value: 'MATCHED' },
  { label: '未匹配', value: 'UNMATCHED' },
  { label: '自动匹配', value: 'AUTO' },
  { label: '手动匹配', value: 'MANUAL' }
];

const mappingStatusOptions = [
  { label: '映射有效', value: 'ACTIVE' },
  { label: '匹配冲突', value: 'CONFLICT' },
  { label: '映射失效', value: 'INACTIVE' }
];

const matchSourceOptions = [
  { label: '自动', value: 'AUTO' },
  { label: '手动', value: 'MANUAL' }
];

const conflictReasonOptions = [
  { label: '无候选 SKU', value: 'NO_CANDIDATE' },
  { label: '多候选 SKU', value: 'MULTI_CANDIDATE' },
  { label: '候选 SKU 不可用', value: 'SKU_INACTIVE' }
];

const conflictReasonLabelMap: Record<string, string> = {
  NO_CANDIDATE: '无候选 SKU',
  MULTI_CANDIDATE: '多候选 SKU',
  SKU_INACTIVE: '候选 SKU 不可用'
};

const normalizeStatus = (status?: ShopProductStatusFilter) => {
  if (!status || status === 'ALL') {
    return undefined;
  }
  return status;
};

const resolveMatchStatus = (record: ShopProductSku) => {
  if (record.mapping_status === 'CONFLICT') {
    return { label: '匹配冲突', color: 'red' };
  }
  if (!record.mapping_id) {
    return { label: '未匹配', color: 'default' };
  }
  if (record.match_source === 'MANUAL') {
    return { label: '手动匹配', color: 'blue' };
  }
  if (record.match_source === 'AUTO') {
    return { label: '自动匹配', color: 'green' };
  }
  return { label: '已匹配', color: 'purple' };
};

export const ShopProductDownloadPage = ({ title }: ShopProductDownloadPageProps) => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const canWrite = useCan('shop_products.write');
  const pageTitle = title ?? '缃戝簵鍟嗗搧';
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [taskPagination, setTaskPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [selectedRows, setSelectedRows] = useState<ShopProductSku[]>([]);
  const [downloadOpen, setDownloadOpen] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [manualOpen, setManualOpen] = useState(false);
  const [taskDrawerOpen, setTaskDrawerOpen] = useState(false);
  const [downloadAutoMatch, setDownloadAutoMatch] = useState(false);
  const [createScope, setCreateScope] = useState<'selected' | 'filtered'>('selected');
  const [downloadLoading, setDownloadLoading] = useState(false);
  const [createLoading, setCreateLoading] = useState(false);
  const [autoMatchLoading, setAutoMatchLoading] = useState(false);
  const [manualLoading, setManualLoading] = useState(false);
  const [deleteLoading, setDeleteLoading] = useState(false);
  const [autoMatchFailures, setAutoMatchFailures] = useState<ShopProductMatchFailure[]>([]);

  const [filterForm] = Form.useForm<FilterValues>();
  const [downloadForm] = Form.useForm<DownloadFormValues>();
  const [createForm] = Form.useForm<CreateFormValues>();
  const [manualForm] = Form.useForm<ManualFormValues>();
  const importStock = Form.useWatch('import_stock', createForm);

  const { data: shopData, isLoading: shopLoading } = useQuery({
    queryKey: ['shops', tenantId, 'shop-products'],
    queryFn: () => listShops(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000
  });

  const platformOptions = useMemo(() => {
    const platforms = Array.from(
      new Set((shopData?.items ?? []).map((shop) => shop.platform))
    );
    return platforms.map((platform) => ({
      label: platformLabels[platform] ?? platform,
      value: platform
    }));
  }, [shopData?.items]);

  const shopOptions = useMemo(
    () =>
      shopData?.items.map((shop) => ({
        label: `${shop.name} (${shop.shop_id})`,
        value: shop.shop_id
      })) ?? [],
    [shopData?.items]
  );

  const shopMap = useMemo(() => {
    const map = new Map<string, string>();
    shopData?.items.forEach((shop) => {
      map.set(shop.shop_id, shop.name);
    });
    return map;
  }, [shopData?.items]);
  const activeRulePlatform = filters.platform ?? shopData?.items?.[0]?.platform;
  const { data: mappingRuleData } = useQuery({
    queryKey: ['mapping-rules', tenantId, activeRulePlatform, 'shop-products'],
    queryFn: () =>
      listMappingRules(tenantId, {
        platform: activeRulePlatform,
        entity_type: 'SHOP_PRODUCT',
        enabled: true,
        limit: 20
      }),
    enabled: Boolean(tenantId && activeRulePlatform)
  });
  const apparelRuleSummary = useMemo(() => {
    const targetRule = mappingRuleData?.items.find(
      (rule) => rule.rule_json?.industry_profile === 'APPAREL'
    );
    if (!targetRule) {
      return null;
    }
    const colorDict = (targetRule.rule_json?.color_dict as Record<string, string> | undefined) ?? {};
    const sizeDict = (targetRule.rule_json?.size_dict as Record<string, string> | undefined) ?? {};
    return {
      templateName: String(targetRule.rule_json?.template_name ?? '闉嬫湇妯℃澘'),
      colorCount: Object.keys(colorDict).length,
      sizeCount: Object.keys(sizeDict).length
    };
  }, [mappingRuleData?.items]);

  const { data: warehouseData, isLoading: warehouseLoading } = useQuery({
    queryKey: ['warehouses', tenantId, 'shop-product-create'],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: createOpen && Boolean(tenantId)
  });

  const warehouseOptions = useMemo(
    () =>
      (warehouseData?.items ?? []).map((warehouse) => ({
        value: warehouse.warehouse_id,
        label: `${warehouse.name} (${warehouse.code})`
      })),
    [warehouseData?.items]
  );

  const { data: skuData, isLoading: skuLoading } = useSkusList({
    pageSize: 200,
    enabled: manualOpen
  });
  const skuOptions = useMemo(() => skuData?.data ?? [], [skuData?.data]);

  const query = useMemo<ShopProductSkuListQuery>(() => {
    return {
      platform: filters.platform,
      shop_ids: filters.shop_ids,
      product_type: filters.product_type?.trim() || undefined,
      status: normalizeStatus(filters.status),
      keyword: filters.keyword?.trim() || undefined,
      match_status: filters.match_status,
      mapping_status: filters.mapping_status,
      conflict_reason: filters.conflict_reason,
      match_source: filters.match_source,
      updated_from: filters.updated_range?.[0]?.toISOString(),
      updated_to: filters.updated_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useShopProductSkus({
    query,
    enabled: Boolean(tenantId)
  });

  const taskQuery = useMemo(
    () => ({
      offset: ((taskPagination.current ?? 1) - 1) * (taskPagination.pageSize ?? 10),
      limit: taskPagination.pageSize ?? 10
    }),
    [taskPagination.current, taskPagination.pageSize]
  );
  const {
    data: taskData,
    isFetching: taskFetching,
    error: taskError,
    refetch: refetchTasks
  } = useShopProductTasks({ query: taskQuery, enabled: taskDrawerOpen });

  const columns = useMemo<ColumnsType<ShopProductSku>>(
    () => [
      {
        title: '骞冲彴',
        dataIndex: 'platform',
        render: (_, record) => <PlatformTag platform={record.platform} />
      },
      {
        title: '搴楅摵',
        dataIndex: 'shop_id',
        render: (value: string) => shopMap.get(value) ?? value
      },
      {
        title: '鍟嗗搧鏍囬',
        dataIndex: 'product_title'
      },
      {
        title: '鍟嗗缂栫爜',
        dataIndex: 'merchant_code',
        render: (value: string | null | undefined) => value ?? '-'
      },
      {
        title: '瑙勬牸缂栫爜',
        dataIndex: 'merchant_sku_code',
        render: (value: string | null | undefined, record) => value ?? record.platform_sku_id
      },
      {
        title: '瑙勬牸淇℃伅',
        dataIndex: 'spec_text',
        render: (value: string | null | undefined, record) =>
          value ?? record.sku_title ?? '-'
      },
      {
        title: '浠锋牸',
        dataIndex: 'price_cent',
        render: (value: number | null | undefined) => formatCurrency(value)
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: string) => <Tag>{value}</Tag>
      },
      {
        title: '鏇存柊鏃堕棿',
        dataIndex: 'platform_updated_at',
        render: (value: string | null | undefined, record) =>
          formatDateTime(value ?? record.updated_at)
      },
      {
        title: '匹配状态',
        dataIndex: 'match_source',
        render: (_, record) => {
          const status = resolveMatchStatus(record);
          return <Tag color={status.color}>{status.label}</Tag>;
        }
      },
      {
        title: '映射状态',
        dataIndex: 'mapping_status',
        render: (value: string | null | undefined) => (value ? <Tag>{value}</Tag> : '-')
      },
      {
        title: '鍐茬獊鍘熷洜',
        dataIndex: 'conflict_reason',
        render: (value: string | null | undefined) =>
          value ? conflictReasonLabelMap[value] ?? value : '-'
      },
      {
        title: '鍖归厤璐у搧',
        dataIndex: 'mapped_sku_code',
        render: (_, record) =>
          record.mapped_sku_code
            ? `${record.mapped_sku_code} ${record.mapped_sku_name ?? ''}`
            : '-'
      }
    ],
    [shopMap]
  );

  const taskColumns = useMemo<ColumnsType<ShopProductTask>>(
    () => [
      {
        title: '浠诲姟绫诲瀷',
        dataIndex: 'task_type'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: string) => <Tag>{value}</Tag>
      },
      {
        title: '鎬绘暟',
        dataIndex: 'total_count'
      },
      {
        title: '鎴愬姛',
        dataIndex: 'success_count'
      },
      {
        title: '澶辫触',
        dataIndex: 'failure_count'
      },
      {
        title: '閿欒',
        dataIndex: 'error',
        render: (value: string) => value || '-'
      },
      {
        title: '开始时间',
        dataIndex: 'started_at',
        render: (value: string) => formatDateTime(value)
      },
      {
        title: '瀹屾垚鏃堕棿',
        dataIndex: 'finished_at',
        render: (value: string | null | undefined) => formatDateTime(value)
      }
    ],
    []
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    filterForm.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const buildFilterPayload = (values: FilterValues): ShopProductSkuListQuery => ({
    platform: values.platform,
    shop_ids: values.shop_ids,
    product_type: values.product_type?.trim() || undefined,
    status: normalizeStatus(values.status),
    keyword: values.keyword?.trim() || undefined,
    match_status: values.match_status,
    mapping_status: values.mapping_status,
    conflict_reason: values.conflict_reason,
    match_source: values.match_source,
    updated_from: values.updated_range?.[0]?.toISOString(),
    updated_to: values.updated_range?.[1]?.toISOString()
  });

  const openDownloadModal = (autoMatch: boolean) => {
    const defaultStatus =
      filters.status && filters.status !== 'ALL' ? filters.status : 'ACTIVE';
    setDownloadAutoMatch(autoMatch);
    setDownloadOpen(true);
    downloadForm.setFieldsValue({
      platform: filters.platform,
      shop_ids: filters.shop_ids,
      product_type: filters.product_type,
      status: defaultStatus,
      keyword: filters.keyword,
      updated_range: filters.updated_range
    });
  };

  const handleDownload = async () => {
    try {
      const values = await downloadForm.validateFields();
      const payload: ShopProductDownloadPayload = {
        platform: values.platform,
        shop_ids: values.shop_ids,
        product_type: values.product_type?.trim() || undefined,
        status: normalizeStatus(values.status),
        keyword: values.keyword?.trim() || undefined,
        updated_from: values.updated_range?.[0]?.toISOString(),
        updated_to: values.updated_range?.[1]?.toISOString(),
        auto_match: downloadAutoMatch
      };
      setDownloadLoading(true);
      const response = await downloadShopProducts(tenantId, payload);
      message.success('下载任务已创建');
      if (response.auto_match_failures?.length) {
        Modal.warning({
          title: '鑷姩鍖归厤澶辫触',
          content: (
            <div>
              {response.auto_match_failures.slice(0, 6).map((item) => (
                <div key={item.sku_record_id}>
                  {`${item.platform_sku_id}: ${item.reason}`}
                </div>
              ))}
            </div>
          )
        });
      }
      setDownloadOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'skus', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'tasks', tenantId] });
      setTaskDrawerOpen(true);
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message || '涓嬭浇澶辫触');
      }
    } finally {
      setDownloadLoading(false);
    }
  };

  const openCreateModal = (scope: 'selected' | 'filtered') => {
    setCreateScope(scope);
    setCreateOpen(true);
    createForm.resetFields();
    createForm.setFieldsValue({
      import_price: true,
      import_image: true,
      import_stock: false
    });
  };

  const handleCreateSkus = async () => {
    try {
      const values = await createForm.validateFields();
      if (createScope === 'selected' && selectedRowKeys.length === 0) {
        message.warning('璇峰厛閫夋嫨鍟嗗搧');
        return;
      }
      const payload: ShopProductCreateSkuPayload = {
        sku_record_ids: createScope === 'selected' ? selectedRowKeys : undefined,
        filters: createScope === 'filtered' ? buildFilterPayload(filters) : undefined,
        import_price: values.import_price,
        import_image: values.import_image,
        import_stock: values.import_stock,
        warehouse_id: values.warehouse_id
      };
      setCreateLoading(true);
      await createShopProductSkus(tenantId, payload);
      message.success('货品档案创建任务已提交');
      setCreateOpen(false);
      setSelectedRowKeys([]);
      setSelectedRows([]);
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'tasks', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'skus', tenantId] });
      setTaskDrawerOpen(true);
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message || '鍒涘缓澶辫触');
      }
    } finally {
      setCreateLoading(false);
    }
  };

  const handleAutoMatch = async (scope: 'selected' | 'filtered') => {
    try {
      if (scope === 'selected' && selectedRowKeys.length === 0) {
        message.warning('璇峰厛閫夋嫨鍟嗗搧');
        return;
      }
      const payload: ShopProductAutoMatchPayload = {
        sku_record_ids: scope === 'selected' ? selectedRowKeys : undefined,
        filters: scope === 'filtered' ? buildFilterPayload(filters) : undefined
      };
      setAutoMatchLoading(true);
      const response = await autoMatchShopProducts(tenantId, payload);
      message.success('自动匹配任务已提交');
      if (response.failures?.length) {
        setAutoMatchFailures(response.failures);
      } else {
        setAutoMatchFailures([]);
      }
      setSelectedRowKeys([]);
      setSelectedRows([]);
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'tasks', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'skus', tenantId] });
      setTaskDrawerOpen(!response.failures?.length);
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message || '鑷姩鍖归厤澶辫触');
      }
    } finally {
      setAutoMatchLoading(false);
    }
  };

  const handleManualMatch = async () => {
    try {
      const values = await manualForm.validateFields();
      if (!selectedRowKeys.length) {
        message.warning('璇峰厛閫夋嫨鍟嗗搧');
        return;
      }
      if (!values.sku_id) {
        message.warning('璇烽€夋嫨璐у搧妗ｆ');
        return;
      }
      const payload: ShopProductManualMatchPayload = {
        sku_record_ids: selectedRowKeys,
        sku_id: values.sku_id
      };
      setManualLoading(true);
      await manualMatchShopProducts(tenantId, payload);
      message.success('鎵嬪姩鍖归厤瀹屾垚');
      setManualOpen(false);
      setSelectedRowKeys([]);
      setSelectedRows([]);
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'tasks', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'skus', tenantId] });
      setTaskDrawerOpen(true);
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message || '鎵嬪姩鍖归厤澶辫触');
      }
    } finally {
      setManualLoading(false);
    }
  };

  const handleDeleteMatch = async () => {
    try {
      if (!selectedRowKeys.length) {
        message.warning('璇峰厛閫夋嫨鍟嗗搧');
        return;
      }
      const payload: ShopProductDeleteMatchPayload = {
        sku_record_ids: selectedRowKeys
      };
      setDeleteLoading(true);
      await deleteShopProductMatches(tenantId, payload);
      message.success('匹配已删除');
      setSelectedRowKeys([]);
      setSelectedRows([]);
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'tasks', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['shop-products', 'skus', tenantId] });
      setTaskDrawerOpen(true);
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message || '鍒犻櫎鍖归厤澶辫触');
      }
    } finally {
      setDeleteLoading(false);
    }
  };

  return (
    <>
      <WorkbenchListPage
        query={
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            {apparelRuleSummary ? (
              <Alert
                type="info"
                showIcon
                data-testid="shop-products-apparel-rule-summary"
                message="鞋服模板字典已启用"
                description={`当前平台已启用 ${apparelRuleSummary.templateName}，颜色字典 ${apparelRuleSummary.colorCount} 项，尺码字典 ${apparelRuleSummary.sizeCount} 项，自动匹配会按规格文本联动治理。`}
              />
            ) : null}
            <Card size="small">
              <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
                <Row gutter={16}>
                <Col xs={24}>
                  <Form.Item name="platform" label="骞冲彴">
                    <Select
                      allowClear
                      options={platformOptions}
                      placeholder="閫夋嫨骞冲彴"
                      loading={shopLoading}
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="shop_ids" label="搴楅摵">
                    <Select
                      mode="multiple"
                      allowClear
                      options={shopOptions}
                      placeholder="閫夋嫨搴楅摵"
                      loading={shopLoading}
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="product_type" label="鍟嗗搧绫诲瀷">
                    <Input allowClear placeholder="杈撳叆鍟嗗搧绫诲瀷" />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="status" label="商品状态">
                    <Select
                      allowClear
                      options={statusOptions}
                      placeholder="选择商品状态"
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="match_status" label="匹配状态">
                    <Select
                      data-testid="shop-products-filter-match-status"
                      allowClear
                      options={matchStatusOptions}
                      placeholder="选择匹配状态"
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="mapping_status" label="映射状态">
                    <Select
                      data-testid="shop-products-filter-mapping-status"
                      allowClear
                      options={mappingStatusOptions}
                      placeholder="选择映射状态"
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="match_source" label="鍖归厤鏉ユ簮">
                    <Select
                      data-testid="shop-products-filter-match-source"
                      allowClear
                      options={matchSourceOptions}
                      placeholder="閫夋嫨鍖归厤鏉ユ簮"
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="conflict_reason" label="鍐茬獊鍘熷洜">
                    <Select
                      data-testid="shop-products-filter-conflict-reason"
                      allowClear
                      options={conflictReasonOptions}
                      placeholder="閫夋嫨鍐茬獊鍘熷洜"
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="keyword" label="关键词">
                    <Input
                      data-testid="shop-products-filter-keyword"
                      allowClear
                      placeholder="鏍囬/鍟嗗缂栫爜/瑙勬牸缂栫爜"
                    />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="updated_range" label="鏇存柊鏃堕棿">
                    <DatePicker.RangePicker style={{ width: '100%' }} />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Space>
                    <Button
                      data-testid="shop-products-filter-submit"
                      type="primary"
                      htmlType="submit"
                      size="small"
                    >
                      鏌ヨ
                    </Button>
                    <Button data-testid="shop-products-filter-reset" onClick={handleReset} size="small">
                      閲嶇疆
                    </Button>
                  </Space>
                </Col>
                </Row>
              </Form>
            </Card>
          </Space>
        }
        toolbarLeft={
          <Space size={8} wrap={false} align="center">
            <Typography.Title level={4} style={{ margin: 0 }}>
              {pageTitle}
            </Typography.Title>
            <Dropdown.Button
              type="primary"
              disabled={!canWrite}
              onClick={() => openDownloadModal(false)}
              menu={{
                items: [{ key: 'auto', label: '下载并匹配' }],
                onClick: () => openDownloadModal(true)
              }}
            >
              涓嬭浇鍟嗗搧
            </Dropdown.Button>
            <Dropdown
              menu={{
                items: [
                  { key: 'selected', label: '按勾选创建' },
                  { key: 'filtered', label: '按筛选创建' }
                ],
                onClick: ({ key }) => openCreateModal(key as 'selected' | 'filtered')
              }}
            >
              <Button disabled={!canWrite}>鍒涘缓璐у搧妗ｆ</Button>
            </Dropdown>
            <Dropdown
              menu={{
                items: [
                  {
                    key: 'selected',
                    label: <span data-testid="shop-products-auto-match-selected">按勾选匹配</span>
                  },
                  {
                    key: 'filtered',
                    label: <span data-testid="shop-products-auto-match-filtered">按筛选匹配</span>
                  }
                ],
                onClick: ({ key }) => handleAutoMatch(key as 'selected' | 'filtered')
              }}
            >
              <Button
                data-testid="shop-products-auto-match-button"
                loading={autoMatchLoading}
                disabled={!canWrite}
              >
                鑷姩鍖归厤
              </Button>
            </Dropdown>
            <Button
              data-testid="shop-products-manual-match-button"
              disabled={!canWrite}
              onClick={() => {
                manualForm.resetFields();
                setManualOpen(true);
              }}
            >
              鎵嬪姩鍖归厤
            </Button>
            <Popconfirm
              title="纭畾鍒犻櫎鍖归厤鍚楋紵"
              onConfirm={handleDeleteMatch}
              okButtonProps={{ loading: deleteLoading }}
            >
              <Button disabled={!canWrite}>鍒犻櫎鍖归厤</Button>
            </Popconfirm>
          </Space>
        }
        toolbarRight={
          <UiModule02ToolbarRight
            onExport={() => message.info('导出功能待接入')}
            onPrint={() => message.info('打印功能待接入')}
            onHelp={() => message.info('帮助功能待接入')}
          />
        }
        detailGroups={[
          {
            key: 'tasks',
            label: '浠诲姟涓績',
            children: (
              <Space direction="vertical" size={8}>
                <Typography.Text type="secondary">
                  涓嬭浇/鍒涘缓/鍖归厤浠诲姟浼氳褰曞湪浠诲姟鍒楄〃銆?
                </Typography.Text>
                <Button size="small" onClick={() => setTaskDrawerOpen(true)}>
                  鏌ョ湅浠诲姟
                </Button>
              </Space>
            )
          },
          {
            key: 'match',
            label: '匹配状态',
            children: (
              <Space direction="vertical" size={4}>
                <Typography.Text>鏈尮閰嶏細鏈叧鑱擡RP璐у搧</Typography.Text>
                <Typography.Text>自动匹配：系统按规则匹配 ERP 货品</Typography.Text>
                <Typography.Text>鎵嬪姩鍖归厤锛氫汉宸ユ寚瀹欵RP璐у搧</Typography.Text>
              </Space>
            )
          }
        ]}
        table={
          <ServerTable<ShopProductSku>
            rowKey="sku_record_id"
            columns={columns}
            data={data?.items ?? []}
            loading={isLoading || isFetching}
            error={error}
            onRetry={() => refetch()}
            errorTitle={`${pageTitle}鍔犺浇澶辫触`}
            emptyText={`鏆傛棤${pageTitle}`}
            total={data?.total ?? 0}
            pagination={pagination}
            onPaginationChange={setPagination}
            rowSelection={{
              selectedRowKeys,
              onChange: (keys, rows) => {
                setSelectedRowKeys(keys as string[]);
                setSelectedRows(rows as ShopProductSku[]);
              }
            }}
            columnSettingsKey="shop-products.table.columns"
          />
        }
      />
      <Modal
        title="自动匹配失败"
        open={autoMatchFailures.length > 0}
        onCancel={() => setAutoMatchFailures([])}
        onOk={() => setAutoMatchFailures([])}
        okText="知道了"
        cancelButtonProps={{ style: { display: 'none' } }}
      >
        <Space direction="vertical" size={8} style={{ width: '100%' }}>
          {autoMatchFailures.slice(0, 6).map((item) => (
            <Typography.Text key={item.sku_record_id}>{`${item.platform_sku_id}: ${item.reason}`}</Typography.Text>
          ))}
        </Space>
      </Modal>
      <Modal
        title={downloadAutoMatch ? '下载商品并匹配' : '下载商品'}
        open={downloadOpen}
        onCancel={() => setDownloadOpen(false)}
        onOk={handleDownload}
        confirmLoading={downloadLoading}
        okText="开始下载"
      >
        <Form form={downloadForm} layout="vertical">
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="platform" label="骞冲彴">
                <Select
                  allowClear
                  options={platformOptions}
                  placeholder="閫夋嫨骞冲彴"
                  loading={shopLoading}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item
                name="shop_ids"
                label="搴楅摵"
                rules={[{ required: true, message: '璇烽€夋嫨搴楅摵' }]}
              >
                <Select
                  mode="multiple"
                  allowClear
                  options={shopOptions}
                  placeholder="閫夋嫨搴楅摵"
                  loading={shopLoading}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="product_type" label="鍟嗗搧绫诲瀷">
                <Input allowClear placeholder="杈撳叆鍟嗗搧绫诲瀷" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="status" label="商品状态">
                <Select
                  allowClear
                  options={statusOptions}
                  placeholder="选择商品状态"
                />
              </Form.Item>
            </Col>
            <Col xs={24}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="鏍囬/鍟嗗缂栫爜/瑙勬牸缂栫爜" />
              </Form.Item>
            </Col>
            <Col xs={24}>
              <Form.Item name="updated_range" label="鏇存柊鏃堕棿">
                <DatePicker.RangePicker style={{ width: '100%' }} showTime />
              </Form.Item>
            </Col>
          </Row>
        </Form>
      </Modal>
      <Modal
        title={createScope === 'selected' ? '鍒涘缓璐у搧妗ｆ锛堝嬀閫夛級' : '鍒涘缓璐у搧妗ｆ锛堢瓫閫夛級'}
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={handleCreateSkus}
        okText="鎻愪氦鍒涘缓"
        confirmLoading={createLoading}
      >
        <Form form={createForm} layout="vertical">
          <Form.Item name="import_price" valuePropName="checked">
            <Checkbox>瀵煎叆缃戝簵浠锋牸</Checkbox>
          </Form.Item>
          <Form.Item name="import_image" valuePropName="checked">
            <Checkbox>瀵煎叆鍟嗗搧鍥剧墖</Checkbox>
          </Form.Item>
          <Form.Item name="import_stock" valuePropName="checked">
            <Checkbox>瀵煎叆缃戝簵搴撳瓨</Checkbox>
          </Form.Item>
          {importStock ? (
            <>
              <Alert
                type="warning"
                message="库存关系建立后可能难以解除，请谨慎选择仓库。"
                style={{ marginBottom: 12 }}
              />
              <Form.Item
                name="warehouse_id"
                label="瀛樻斁浠撳簱"
                rules={[{ required: true, message: '璇烽€夋嫨浠撳簱' }]}
              >
                <Select
                  allowClear
                  placeholder="閫夋嫨浠撳簱"
                  options={warehouseOptions}
                  loading={warehouseLoading}
                />
              </Form.Item>
            </>
          ) : null}
        </Form>
      </Modal>
      <Modal
        title="鎵嬪姩鍖归厤"
        open={manualOpen}
        onCancel={() => setManualOpen(false)}
        onOk={handleManualMatch}
        okText="鎻愪氦鍖归厤"
        confirmLoading={manualLoading}
      >
        <Form form={manualForm} layout="vertical">
          <Form.Item
            name="sku_id"
            label="閫夋嫨璐у搧妗ｆ"
            rules={[{ required: true, message: '璇烽€夋嫨璐у搧妗ｆ' }]}
          >
            <Select
              data-testid="shop-products-manual-match-sku-select"
              showSearch
              loading={skuLoading}
              optionFilterProp="label"
              placeholder="閫夋嫨ERP璐у搧"
              options={skuOptions.map((sku) => ({
                value: sku.sku_id,
                label: `${sku.sku_code} ${sku.name ?? ''}`
              }))}
            />
          </Form.Item>
          <Typography.Text type="secondary">
            宸插嬀閫?{selectedRows.length} 涓綉搴楄鏍硷紝灏嗘槧灏勫埌鍚屼竴璐у搧銆?
          </Typography.Text>
        </Form>
      </Modal>
      <Drawer
        title="鍟嗗搧涓嬭浇浠诲姟"
        open={taskDrawerOpen}
        onClose={() => setTaskDrawerOpen(false)}
        width={720}
      >
        <ServerTable<ShopProductTask>
          rowKey="task_id"
          columns={taskColumns}
          data={taskData?.items ?? []}
          loading={taskFetching}
          error={taskError}
          onRetry={() => refetchTasks()}
          errorTitle="浠诲姟鍔犺浇澶辫触"
          emptyText="鏆傛棤浠诲姟"
          total={taskData?.total ?? 0}
          pagination={taskPagination}
          onPaginationChange={setTaskPagination}
          columnSettingsKey="shop-products.task.columns"
        />
      </Drawer>
    </>
  );
};
