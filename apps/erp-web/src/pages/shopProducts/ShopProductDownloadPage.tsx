import {
  Button,
  Checkbox,
  Col,
  DatePicker,
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
  message,
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import dayjs from 'dayjs';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';

import {
  autoMatchShopProducts,
  createShopProductSkus,
  deleteShopProductMatches,
  downloadShopProducts,
  manualMatchShopProducts,
  type ShopProductAutoMatchPayload,
  type ShopProductCreateSkuPayload,
  type ShopProductDeleteMatchPayload,
  type ShopProductDownloadPayload,
  type ShopProductManualMatchPayload,
  type ShopProductMatchFailure,
  type ShopProductSku,
  type ShopProductSkuListQuery,
} from '@/api/shopProducts';
import { useShopProductSkus, useShopProductTasks, useSkusList } from '@/api/hooks';
import { listMappingRules } from '@/api/mappingRules';
import { listShops } from '@/api/shops';
import { listWarehouses } from '@/api/warehouses';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import {
  WorkbenchPageTemplate,
  WorkbenchToolbar,
} from '@/components/page-templates';
import type { PlatformCode } from '@/features/platform';
import { platformLabels } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

import { ShopProductWorkbenchBottomPanel } from './ShopProductWorkbenchBottomPanel';
import { ShopProductWorkbenchDetailPanel } from './ShopProductWorkbenchDetailPanel';
import { ShopProductWorkbenchLeftPanel } from './ShopProductWorkbenchLeftPanel';
import { ShopProductWorkbenchMainTable } from './ShopProductWorkbenchMainTable';
import {
  resolveActiveShopProductSkuRecordId,
  summarizeShopProductRows,
} from './shopProductWorkbenchModel';

import '../orders/orderReviewPage.css';
import './shopProductDownloadPage.css';

type StatusFilter = 'ACTIVE' | 'INACTIVE' | 'ALL';

type FilterValues = {
  platform?: PlatformCode;
  shop_ids?: string[];
  keyword?: string;
  status?: StatusFilter;
  match_status?: string;
  mapping_status?: string;
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

const statusOptions = [
  { label: 'Active', value: 'ACTIVE' },
  { label: 'Inactive', value: 'INACTIVE' },
  { label: 'All', value: 'ALL' },
];

const matchStatusOptions = [
  { label: 'Matched', value: 'MATCHED' },
  { label: 'Unmatched', value: 'UNMATCHED' },
  { label: 'Auto matched', value: 'AUTO' },
  { label: 'Manual matched', value: 'MANUAL' },
];

const mappingStatusOptions = [
  { label: 'Mapping active', value: 'ACTIVE' },
  { label: 'Conflict', value: 'CONFLICT' },
  { label: 'Mapping inactive', value: 'INACTIVE' },
];

const normalizeStatus = (status?: StatusFilter) => (!status || status === 'ALL' ? undefined : status);

const toFilterPayload = (value: FilterValues): ShopProductSkuListQuery => ({
  platform: value.platform,
  shop_ids: value.shop_ids,
  keyword: value.keyword?.trim() || undefined,
  status: normalizeStatus(value.status),
  match_status: value.match_status,
  mapping_status: value.mapping_status,
  updated_from: value.updated_range?.[0]?.toISOString(),
  updated_to: value.updated_range?.[1]?.toISOString(),
});

export type ShopProductDownloadPageProps = {
  title?: string;
};

export const ShopProductDownloadPage = ({ title }: ShopProductDownloadPageProps) => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const canWrite = useCan('shop_products.write');
  const {
    setLeftPanel,
    setLeftPanelCollapsed,
    setLeftPanelVisible,
  } = useWorkbenchPanels();

  const pageTitle = title ?? '网店商品下载';

  const [filters, setFilters] = useState<FilterValues>({});
  const [toolbarSearchInput, setToolbarSearchInput] = useState('');
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });
  const [taskPagination, setTaskPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10,
  });

  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [selectedRows, setSelectedRows] = useState<ShopProductSku[]>([]);
  const [activeSkuRecordId, setActiveSkuRecordId] = useState<string | null>(null);
  const [downloadOpen, setDownloadOpen] = useState(false);
  const [downloadAutoMatch, setDownloadAutoMatch] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [manualOpen, setManualOpen] = useState(false);
  const [autoMatchFailures, setAutoMatchFailures] = useState<ShopProductMatchFailure[]>([]);
  const [createScope, setCreateScope] = useState<'selected' | 'filtered'>('selected');

  const [downloadLoading, setDownloadLoading] = useState(false);
  const [createLoading, setCreateLoading] = useState(false);
  const [autoMatchLoading, setAutoMatchLoading] = useState(false);
  const [manualLoading, setManualLoading] = useState(false);
  const [deleteLoading, setDeleteLoading] = useState(false);

  const [filterForm] = Form.useForm<FilterValues>();
  const [downloadForm] = Form.useForm<FilterValues>();
  const [createForm] = Form.useForm<CreateFormValues>();
  const [manualForm] = Form.useForm<ManualFormValues>();

  const importStock = Form.useWatch('import_stock', createForm);

  const { data: shopData, isLoading: shopLoading } = useQuery({
    queryKey: ['shops', tenantId, 'shop-products'],
    queryFn: () => listShops(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const platformOptions = useMemo(
    () =>
      Array.from(new Set((shopData?.items ?? []).map((shop) => shop.platform))).map((platform) => ({
        value: platform,
        label: platformLabels[platform] ?? platform,
      })),
    [shopData?.items],
  );

  const shopOptions = useMemo(
    () =>
      (shopData?.items ?? []).map((shop) => ({
        value: shop.shop_id,
        label: `${shop.name} (${shop.shop_id})`,
      })),
    [shopData?.items],
  );

  const shopMap = useMemo(() => {
    const mapping = new Map<string, string>();
    (shopData?.items ?? []).forEach((shop) => {
      mapping.set(shop.shop_id, shop.name);
    });
    return mapping;
  }, [shopData?.items]);

  const activeRulePlatform = filters.platform ?? shopData?.items?.[0]?.platform;

  const { data: mappingRuleData } = useQuery({
    queryKey: ['mapping-rules', tenantId, activeRulePlatform, 'shop-products'],
    queryFn: () =>
      listMappingRules(tenantId, {
        platform: activeRulePlatform,
        entity_type: 'SHOP_PRODUCT',
        enabled: true,
        limit: 20,
      }),
    enabled: Boolean(tenantId && activeRulePlatform),
  });

  const apparelRule = useMemo(
    () => mappingRuleData?.items.find((rule) => rule.rule_json?.industry_profile === 'APPAREL'),
    [mappingRuleData?.items],
  );

  const skuQuery = useMemo<ShopProductSkuListQuery>(
    () => ({
      platform: filters.platform,
      shop_ids: filters.shop_ids,
      keyword: filters.keyword?.trim() || undefined,
      status: normalizeStatus(filters.status),
      match_status: filters.match_status,
      mapping_status: filters.mapping_status,
      updated_from: filters.updated_range?.[0]?.toISOString(),
      updated_to: filters.updated_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20,
    }),
    [filters, pagination.current, pagination.pageSize],
  );

  const { data, isLoading, isFetching, error, refetch } = useShopProductSkus({
    query: skuQuery,
    enabled: Boolean(tenantId),
  });

  const {
    data: taskData,
    isFetching: taskFetching,
    error: taskError,
  } = useShopProductTasks({
    query: {
      offset: ((taskPagination.current ?? 1) - 1) * (taskPagination.pageSize ?? 10),
      limit: taskPagination.pageSize ?? 10,
    },
    enabled: Boolean(tenantId),
  });

  const { data: warehouseData, isLoading: warehouseLoading } = useQuery({
    queryKey: ['warehouses', tenantId, 'shop-product-create'],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: createOpen && Boolean(tenantId),
  });

  const warehouseOptions = useMemo(
    () =>
      (warehouseData?.items ?? []).map((warehouse) => ({
        value: warehouse.warehouse_id,
        label: `${warehouse.name} (${warehouse.code})`,
      })),
    [warehouseData?.items],
  );

  const { data: skuData, isLoading: skuLoading } = useSkusList({
    pageSize: 200,
    enabled: manualOpen,
  });

  const rows = data?.items ?? [];
  const taskRows = taskData?.items ?? [];

  useEffect(() => {
    setActiveSkuRecordId((current) => resolveActiveShopProductSkuRecordId(rows, current));
  }, [rows]);

  const activeSku = useMemo(
    () => rows.find((row) => row.sku_record_id === activeSkuRecordId) ?? null,
    [activeSkuRecordId, rows],
  );

  const summary = useMemo(
    () =>
      summarizeShopProductRows({
        rows,
        total: data?.total ?? 0,
        selectedCount: selectedRowKeys.length,
      }),
    [data?.total, rows, selectedRowKeys.length],
  );

  const refreshData = async () => {
    await queryClient.invalidateQueries({ queryKey: ['shop-products', 'skus', tenantId] });
    await queryClient.invalidateQueries({ queryKey: ['shop-products', 'tasks', tenantId] });
  };

  const handleSearch = (value: FilterValues) => {
    setFilters({
      ...value,
      keyword: toolbarSearchInput.trim() || undefined,
    });
    setPagination((current) => ({ ...current, current: 1 }));
  };

  const handleToolbarSearch = () => {
    const currentValues = filterForm.getFieldsValue();
    handleSearch({
      ...currentValues,
      keyword: toolbarSearchInput.trim() || undefined,
    });
  };

  const handleResetFilters = () => {
    filterForm.resetFields();
    setToolbarSearchInput('');
    setFilters({});
    setPagination((current) => ({ ...current, current: 1 }));
  };

  const handleOpenDownload = (autoMatch: boolean) => {
    setDownloadAutoMatch(autoMatch);
    setDownloadOpen(true);
    downloadForm.setFieldsValue(filters);
  };

  const handleAutoMatch = async (scope: 'selected' | 'filtered') => {
    try {
      if (scope === 'selected' && !selectedRowKeys.length) {
        message.warning('Please select platform SKUs first');
        return;
      }

      setAutoMatchLoading(true);
      const payload: ShopProductAutoMatchPayload = {
        sku_record_ids: scope === 'selected' ? selectedRowKeys : undefined,
        filters: scope === 'filtered' ? toFilterPayload(filters) : undefined,
      };
      const result = await autoMatchShopProducts(tenantId, payload);
      setAutoMatchFailures(result.failures ?? []);
      setSelectedRowKeys([]);
      setSelectedRows([]);
      await refreshData();
      message.success('Auto match task submitted');
      if (result.failures?.length) {
        message.warning('Auto match has failures. Continue in the bottom "匹配失败" tab.');
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'Auto match failed');
    } finally {
      setAutoMatchLoading(false);
    }
  };

  const handleDeleteMatches = async () => {
    try {
      if (!selectedRowKeys.length) {
        message.warning('Please select platform SKUs first');
        return;
      }

      setDeleteLoading(true);
      const payload: ShopProductDeleteMatchPayload = {
        sku_record_ids: selectedRowKeys,
      };
      await deleteShopProductMatches(tenantId, payload);
      setSelectedRowKeys([]);
      setSelectedRows([]);
      await refreshData();
      message.success('Matches deleted');
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'Delete match failed');
    } finally {
      setDeleteLoading(false);
    }
  };

  const filterContent = (
    <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
      <Row gutter={[12, 0]}>
        <Col xs={24} md={12}>
          <Form.Item name="platform" label="Platform">
            <Select allowClear options={platformOptions} placeholder="Select platform" loading={shopLoading} />
          </Form.Item>
        </Col>
        <Col xs={24} md={12}>
          <Form.Item name="shop_ids" label="Shops">
            <Select
              mode="multiple"
              allowClear
              options={shopOptions}
              placeholder="Select shops"
              loading={shopLoading}
            />
          </Form.Item>
        </Col>
        <Col xs={24} md={12}>
          <Form.Item name="status" label="Product Status">
            <Select allowClear options={statusOptions} placeholder="Select status" />
          </Form.Item>
        </Col>
        <Col xs={24} md={12}>
          <Form.Item name="match_status" label="Match Status">
            <Select
              data-testid="shop-products-filter-match-status"
              allowClear
              options={matchStatusOptions}
              placeholder="Select match status"
            />
          </Form.Item>
        </Col>
        <Col xs={24}>
          <Form.Item name="mapping_status" label="Mapping Status">
            <Select
              data-testid="shop-products-filter-mapping-status"
              allowClear
              options={mappingStatusOptions}
              placeholder="Select mapping status"
            />
          </Form.Item>
        </Col>
        <Col xs={24}>
          <Form.Item name="updated_range" label="Platform Updated Range">
            <DatePicker.RangePicker style={{ width: '100%' }} />
          </Form.Item>
        </Col>
        <Col xs={24}>
          <Space>
            <Button data-testid="shop-products-filter-submit" type="primary" htmlType="submit">
              Search
            </Button>
            <Button data-testid="shop-products-filter-reset" onClick={handleResetFilters}>
              Reset
            </Button>
          </Space>
        </Col>
      </Row>
    </Form>
  );

  const leftPanel = useMemo(
    () => (
      <ShopProductWorkbenchLeftPanel
        filterContent={filterContent}
        summary={summary}
        apparelTemplateName={String(apparelRule?.rule_json?.template_name ?? '') || undefined}
      />
    ),
    [apparelRule?.rule_json, filterContent, summary],
  );

  useEffect(() => {
    setLeftPanelCollapsed(false);
    setLeftPanelVisible(true);
    setLeftPanel(leftPanel);
    return () => {
      setLeftPanel(undefined);
      setLeftPanelVisible(false);
    };
  }, [leftPanel, setLeftPanel, setLeftPanelCollapsed, setLeftPanelVisible]);

  const toolbar = (
    <WorkbenchToolbar
      className="order-review-toolbar"
      leftContent={
        <Space size={8} wrap className="shop-product-match-toolbar-left">
          <Typography.Title level={4} style={{ margin: 0 }}>
            {pageTitle}
          </Typography.Title>
          <Tag color="processing" data-testid="shop-products-toolbar-selected-count">
            已选 {selectedRowKeys.length}
          </Tag>
          <Dropdown.Button
            type="primary"
            disabled={!canWrite}
            onClick={() => handleOpenDownload(false)}
            menu={{
              items: [{ key: 'auto', label: '下载并自动匹配' }],
              onClick: () => handleOpenDownload(true),
            }}
          >
            下载平台 SKU
          </Dropdown.Button>
          <Dropdown
            menu={{
              items: [
                { key: 'selected', label: '按已选记录创建 ERP 档案' },
                { key: 'filtered', label: '按当前筛选创建 ERP 档案' },
              ],
              onClick: ({ key }) => {
                setCreateScope(key as 'selected' | 'filtered');
                setCreateOpen(true);
                createForm.setFieldsValue({
                  import_image: true,
                  import_price: true,
                  import_stock: false,
                });
              },
            }}
          >
            <Button disabled={!canWrite}>Create ERP Records</Button>
          </Dropdown>
          <Dropdown
            menu={{
              items: [
                { key: 'selected', label: <span data-testid="shop-products-auto-match-selected">自动匹配已选</span> },
                { key: 'filtered', label: <span data-testid="shop-products-auto-match-filtered">自动匹配当前筛选</span> },
              ],
              onClick: ({ key }) => {
                void handleAutoMatch(key as 'selected' | 'filtered');
              },
            }}
          >
            <Button
              data-testid="shop-products-auto-match-button"
              loading={autoMatchLoading}
              disabled={!canWrite}
            >
              自动匹配
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
            手动匹配
          </Button>
          <Popconfirm
            title="删除已选匹配关系？"
            onConfirm={() => handleDeleteMatches()}
            okButtonProps={{ loading: deleteLoading }}
          >
            <Button disabled={!canWrite}>删除匹配</Button>
          </Popconfirm>
        </Space>
      }
      searchValue={toolbarSearchInput}
      onSearchChange={setToolbarSearchInput}
      onSearch={handleToolbarSearch}
      searchPlaceholder="平台 SKU / 商家 SKU / 商品标题"
      searchInputClassName="order-review-toolbar-search"
      searchButtonClassName="order-review-toolbar-search-trigger"
      searchInputTestId="shop-products-filter-keyword"
      searchButtonTestId="shop-products-toolbar-search-submit"
      searchButtonAriaLabel="查询"
      onExport={() => message.info('导出功能暂未接入')}
      onPrint={() => message.info('打印功能暂未接入')}
      onHelp={() => message.info('请先使用左侧筛选，再在右侧和底部区域处理详情与任务反馈。')}
    />
  );

  const footerSummary = (
    <Space size={12} wrap>
      <Typography.Text>{`总计 ${summary.total} 条`}</Typography.Text>
      <Typography.Text type="secondary">{`已匹配 ${summary.matched} 条`}</Typography.Text>
      <Typography.Text type="secondary">{`未匹配 ${summary.unmatched} 条`}</Typography.Text>
      <Typography.Text type="secondary">{`冲突 ${summary.conflict} 条`}</Typography.Text>
      <Typography.Text type="secondary">{`已选 ${summary.selected} 条`}</Typography.Text>
    </Space>
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="shop-products-page"
        className="order-review-page shop-product-match-page"
        toolbar={toolbar}
        footerSummary={footerSummary}
        footerSyncKey={`shop-products-${pagination.current ?? 1}-${pagination.pageSize ?? 20}-${summary.total}-${summary.selected}`}
        total={summary.total}
        pagination={pagination}
        onPaginationChange={setPagination}
        showFooterScrollbarRow={true}
        tableContent={
          <ShopProductWorkbenchMainTable
            rows={rows}
            loading={isLoading || isFetching}
            error={error}
            selectedRowKeys={selectedRowKeys}
            activeSkuRecordId={activeSkuRecordId}
            shopNameResolver={(shopId) => shopMap.get(shopId) ?? shopId}
            onRetry={() => refetch()}
            onSelectionChange={(keys, nextRows) => {
              setSelectedRowKeys(keys);
              setSelectedRows(nextRows);
            }}
            onActiveRowChange={setActiveSkuRecordId}
          />
        }
        rightPanel={
          <ShopProductWorkbenchDetailPanel
            activeSku={activeSku}
            shopName={activeSku ? shopMap.get(activeSku.shop_id) ?? activeSku.shop_id : undefined}
          />
        }
        rightPanelSyncKey={activeSkuRecordId ?? 'none'}
        bottomPanel={
          <ShopProductWorkbenchBottomPanel
            tasks={taskRows}
            failures={autoMatchFailures}
            selectedCount={selectedRowKeys.length}
            createScope={createScope}
            taskLoading={taskFetching}
            taskError={taskError}
            taskTotal={taskData?.total ?? 0}
            taskPagination={taskPagination}
            onTaskPaginationChange={setTaskPagination}
          />
        }
        bottomPanelSyncKey={`${activeSkuRecordId ?? 'none'}-${selectedRowKeys.length}-${createScope}-${autoMatchFailures.length}`}
      />

      <Modal
        title={downloadAutoMatch ? 'Download Platform SKUs and Auto Match' : 'Download Platform SKUs'}
        open={downloadOpen}
        onCancel={() => setDownloadOpen(false)}
        onOk={async () => {
          try {
            const values = await downloadForm.validateFields();
            setDownloadLoading(true);
            const payload: ShopProductDownloadPayload = {
              platform: values.platform,
              shop_ids: values.shop_ids,
              keyword: values.keyword?.trim() || filters.keyword?.trim() || undefined,
              status: normalizeStatus(values.status),
              updated_from: values.updated_range?.[0]?.toISOString(),
              updated_to: values.updated_range?.[1]?.toISOString(),
              auto_match: downloadAutoMatch,
            };
            const result = await downloadShopProducts(tenantId, payload);
            setDownloadOpen(false);
            setAutoMatchFailures(result.auto_match_failures ?? []);
            await refreshData();
            message.success('Download task created');
            if (result.auto_match_failures?.length) {
              Modal.warning({
                title: 'Auto match failures',
                content: (
                  <div>
                    {result.auto_match_failures.slice(0, 6).map((item) => (
                      <div key={item.sku_record_id}>{`${item.platform_sku_id}: ${item.reason}`}</div>
                    ))}
                  </div>
                ),
              });
            }
          } catch (err) {
            message.error(err instanceof Error ? err.message : 'Download failed');
          } finally {
            setDownloadLoading(false);
          }
        }}
        confirmLoading={downloadLoading}
        okText="Start Download"
      >
        <Form form={downloadForm} layout="vertical">
          <Row gutter={16}>
            <Col xs={24} md={12}>
              <Form.Item name="platform" label="Platform">
                <Select allowClear options={platformOptions} placeholder="Select platform" loading={shopLoading} />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="shop_ids" label="Shops" rules={[{ required: true, message: 'Select at least one shop' }]}>
                <Select
                  mode="multiple"
                  allowClear
                  options={shopOptions}
                  placeholder="Select shops"
                  loading={shopLoading}
                />
              </Form.Item>
            </Col>
            <Col xs={24}>
              <Form.Item name="keyword" label="Keyword">
                <Input allowClear placeholder="Platform SKU name / merchant SKU code" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="status" label="Product Status">
                <Select allowClear options={statusOptions} placeholder="Select status" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="updated_range" label="Platform Updated Range">
                <DatePicker.RangePicker style={{ width: '100%' }} showTime />
              </Form.Item>
            </Col>
          </Row>
        </Form>
      </Modal>

      <Modal
        title={createScope === 'selected' ? 'Create ERP Records (Selected Rows)' : 'Create ERP Records (Filtered Rows)'}
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={async () => {
          try {
            const values = await createForm.validateFields();
            if (createScope === 'selected' && !selectedRowKeys.length) {
              message.warning('Please select platform SKUs first');
              return;
            }

            setCreateLoading(true);
            const payload: ShopProductCreateSkuPayload = {
              sku_record_ids: createScope === 'selected' ? selectedRowKeys : undefined,
              filters: createScope === 'filtered' ? toFilterPayload(filters) : undefined,
              import_price: values.import_price,
              import_image: values.import_image,
              import_stock: values.import_stock,
              warehouse_id: values.warehouse_id,
            };
            await createShopProductSkus(tenantId, payload);
            setCreateOpen(false);
            setSelectedRowKeys([]);
            setSelectedRows([]);
            await refreshData();
            message.success('Create ERP records task submitted');
          } catch (err) {
            message.error(err instanceof Error ? err.message : 'Create ERP records failed');
          } finally {
            setCreateLoading(false);
          }
        }}
        confirmLoading={createLoading}
        okText="Submit"
      >
        <Form form={createForm} layout="vertical">
          <Form.Item name="import_price" valuePropName="checked">
            <Checkbox>Import shop price</Checkbox>
          </Form.Item>
          <Form.Item name="import_image" valuePropName="checked">
            <Checkbox>Import product image</Checkbox>
          </Form.Item>
          <Form.Item name="import_stock" valuePropName="checked">
            <Checkbox>Import shop stock</Checkbox>
          </Form.Item>
          {importStock ? (
            <Form.Item name="warehouse_id" label="Warehouse" rules={[{ required: true, message: 'Select warehouse' }]}>
              <Select
                allowClear
                options={warehouseOptions}
                placeholder="Select warehouse"
                loading={warehouseLoading}
              />
            </Form.Item>
          ) : null}
        </Form>
      </Modal>

      <Modal
        title="Manual Match"
        open={manualOpen}
        onCancel={() => setManualOpen(false)}
        onOk={async () => {
          try {
            const values = await manualForm.validateFields();
            if (!selectedRowKeys.length) {
              message.warning('Please select platform SKUs first');
              return;
            }
            if (!values.sku_id) {
              message.warning('Please select an ERP SKU');
              return;
            }

            setManualLoading(true);
            const payload: ShopProductManualMatchPayload = {
              sku_record_ids: selectedRowKeys,
              sku_id: values.sku_id,
            };
            await manualMatchShopProducts(tenantId, payload);
            setManualOpen(false);
            setSelectedRowKeys([]);
            setSelectedRows([]);
            await refreshData();
            message.success('Manual match completed');
          } catch (err) {
            message.error(err instanceof Error ? err.message : 'Manual match failed');
          } finally {
            setManualLoading(false);
          }
        }}
        confirmLoading={manualLoading}
        okText="Submit"
      >
        <Form form={manualForm} layout="vertical">
          <Form.Item
            name="sku_id"
            label="ERP SKU"
            rules={[{ required: true, message: 'Please select an ERP SKU' }]}
          >
            <Select
              data-testid="shop-products-manual-match-sku-select"
              showSearch
              loading={skuLoading}
              optionFilterProp="label"
              placeholder="Select ERP SKU"
              options={(skuData?.data ?? []).map((sku) => ({
                value: sku.sku_id,
                label: `${sku.sku_code} ${sku.name ?? ''}`,
              }))}
            />
          </Form.Item>
          <Typography.Text type="secondary">Currently selected: {selectedRows.length} rows</Typography.Text>
        </Form>
      </Modal>
    </>
  );
};
