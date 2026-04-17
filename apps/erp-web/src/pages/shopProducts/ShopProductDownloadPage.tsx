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
  Statistic,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

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
  type ShopProductTask
} from '@/api/shopProducts';
import { useShopProductSkus, useShopProductTasks, useSkusList } from '@/api/hooks';
import { listMappingRules } from '@/api/mappingRules';
import { listShops } from '@/api/shops';
import { listWarehouses } from '@/api/warehouses';
import { PlatformTag } from '@/components/PlatformTag';
import { ServerTable } from '@/components/ServerTable';
import { UiModule02ToolbarRight } from '@/components/ui-modules';
import { WorkbenchListPage } from '@/components/workbench/WorkbenchListPage';
import type { PlatformCode } from '@/features/platform';
import { platformLabels } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

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

const normalizeStatus = (s?: StatusFilter) => (!s || s === 'ALL' ? undefined : s);
const toDate = (v?: string | null) => (v ? dayjs(v).format('YYYY-MM-DD HH:mm') : '-');
const toMoney = (v?: number | null) => (v == null ? '-' : (v / 100).toFixed(2));
const matchTag = (row: ShopProductSku) => {
  if (row.mapping_status === 'CONFLICT') return <Tag color="red">匹配冲突</Tag>;
  if (!row.mapping_id) return <Tag>未匹配</Tag>;
  if (row.match_source === 'MANUAL') return <Tag color="blue">手动匹配</Tag>;
  if (row.match_source === 'AUTO') return <Tag color="green">自动匹配</Tag>;
  return <Tag color="purple">已匹配</Tag>;
};

export type ShopProductDownloadPageProps = { title?: string };

export const ShopProductDownloadPage = ({ title }: ShopProductDownloadPageProps) => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const canWrite = useCan('shop_products.write');
  const pageTitle = title ?? '网店商品匹配';

  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({ current: 1, pageSize: 20 });
  const [taskPagination, setTaskPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [selectedRowKeys, setSelectedRowKeys] = useState<string[]>([]);
  const [selectedRows, setSelectedRows] = useState<ShopProductSku[]>([]);
  const [downloadOpen, setDownloadOpen] = useState(false);
  const [downloadAutoMatch, setDownloadAutoMatch] = useState(false);
  const [createOpen, setCreateOpen] = useState(false);
  const [manualOpen, setManualOpen] = useState(false);
  const [taskDrawerOpen, setTaskDrawerOpen] = useState(false);
  const [autoMatchFailures, setAutoMatchFailures] = useState<ShopProductMatchFailure[]>([]);
  const [createScope, setCreateScope] = useState<'selected' | 'filtered'>('selected');

  const [downloadLoading, setDownloadLoading] = useState(false);
  const [createLoading, setCreateLoading] = useState(false);
  const [autoMatchLoading, setAutoMatchLoading] = useState(false);
  const [manualLoading, setManualLoading] = useState(false);
  const [deleteLoading, setDeleteLoading] = useState(false);

  const [filterForm] = Form.useForm<FilterValues>();
  const [downloadForm] = Form.useForm<FilterValues>();
  const [createForm] = Form.useForm<{ import_price?: boolean; import_image?: boolean; import_stock?: boolean; warehouse_id?: string }>();
  const [manualForm] = Form.useForm<{ sku_id?: string }>();
  const importStock = Form.useWatch('import_stock', createForm);

  const { data: shopData, isLoading: shopLoading } = useQuery({
    queryKey: ['shops', tenantId, 'shop-products'],
    queryFn: () => listShops(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000
  });
  const platformOptions = useMemo(
    () =>
      Array.from(new Set((shopData?.items ?? []).map((s) => s.platform))).map((p) => ({
        value: p,
        label: platformLabels[p] ?? p
      })),
    [shopData?.items]
  );
  const shopOptions = useMemo(
    () =>
      (shopData?.items ?? []).map((s) => ({
        value: s.shop_id,
        label: `${s.name} (${s.shop_id})`
      })),
    [shopData?.items]
  );
  const shopMap = useMemo(() => {
    const m = new Map<string, string>();
    (shopData?.items ?? []).forEach((s) => m.set(s.shop_id, s.name));
    return m;
  }, [shopData?.items]);

  const activeRulePlatform = filters.platform ?? shopData?.items?.[0]?.platform;
  const { data: mappingRuleData } = useQuery({
    queryKey: ['mapping-rules', tenantId, activeRulePlatform, 'shop-products'],
    queryFn: () => listMappingRules(tenantId, { platform: activeRulePlatform, entity_type: 'SHOP_PRODUCT', enabled: true, limit: 20 }),
    enabled: Boolean(tenantId && activeRulePlatform)
  });
  const apparelRule = useMemo(() => mappingRuleData?.items.find((r) => r.rule_json?.industry_profile === 'APPAREL'), [mappingRuleData?.items]);

  const query = useMemo<ShopProductSkuListQuery>(
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
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );
  const { data, isLoading, isFetching, error, refetch } = useShopProductSkus({ query, enabled: Boolean(tenantId) });

  const { data: taskData, isFetching: taskFetching, error: taskError, refetch: refetchTasks } = useShopProductTasks({
    query: {
      offset: ((taskPagination.current ?? 1) - 1) * (taskPagination.pageSize ?? 10),
      limit: taskPagination.pageSize ?? 10
    },
    enabled: taskDrawerOpen
  });

  const { data: warehouseData, isLoading: warehouseLoading } = useQuery({
    queryKey: ['warehouses', tenantId, 'shop-product-create'],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: createOpen && Boolean(tenantId)
  });
  const warehouseOptions = useMemo(
    () => (warehouseData?.items ?? []).map((w) => ({ value: w.warehouse_id, label: `${w.name} (${w.code})` })),
    [warehouseData?.items]
  );
  const { data: skuData, isLoading: skuLoading } = useSkusList({ pageSize: 200, enabled: manualOpen });

  const stats = useMemo(() => {
    const rows = data?.items ?? [];
    const matched = rows.filter((r) => r.mapping_id && r.mapping_status !== 'CONFLICT').length;
    const conflict = rows.filter((r) => r.mapping_status === 'CONFLICT').length;
    return { total: data?.total ?? 0, matched, conflict, unmatched: rows.length - matched - conflict, selected: selectedRowKeys.length };
  }, [data?.items, data?.total, selectedRowKeys.length]);

  const refreshData = async () => {
    await queryClient.invalidateQueries({ queryKey: ['shop-products', 'skus', tenantId] });
    await queryClient.invalidateQueries({ queryKey: ['shop-products', 'tasks', tenantId] });
  };

  const columns = useMemo<ColumnsType<ShopProductSku>>(
    () => [
      { title: '平台', dataIndex: 'platform', width: 90, render: (_, r) => <PlatformTag platform={r.platform} /> },
      { title: '店铺', dataIndex: 'shop_id', width: 170, ellipsis: true, render: (v: string) => shopMap.get(v) ?? v },
      { title: '平台SKU名称', dataIndex: 'spec_text', width: 220, ellipsis: true, render: (v: string | null | undefined, r) => v ?? r.sku_title ?? '-' },
      { title: 'SKU商家编码', dataIndex: 'merchant_sku_code', width: 160, render: (v: string | null | undefined, r) => v ?? r.platform_sku_id },
      { title: '匹配ERP编码', dataIndex: 'mapped_sku_code', width: 220, ellipsis: true, render: (_, r) => (r.mapped_sku_code ? `${r.mapped_sku_code} ${r.mapped_sku_name ?? ''}` : '-') },
      { title: '匹配状态', dataIndex: 'match_source', width: 120, render: (_, r) => matchTag(r) },
      { title: '价格(元)', dataIndex: 'price_cent', width: 100, align: 'right', render: (v: number | null | undefined) => toMoney(v) },
      { title: '平台更新时间', dataIndex: 'platform_updated_at', width: 160, render: (v: string | null | undefined, r) => toDate(v ?? r.updated_at) }
    ],
    [shopMap]
  );

  const taskColumns = useMemo<ColumnsType<ShopProductTask>>(
    () => [
      { title: '任务类型', dataIndex: 'task_type' },
      { title: '状态', dataIndex: 'status', render: (v: string) => <Tag>{v}</Tag> },
      { title: '总数', dataIndex: 'total_count' },
      { title: '成功', dataIndex: 'success_count' },
      { title: '失败', dataIndex: 'failure_count' },
      { title: '开始时间', dataIndex: 'started_at', render: (v: string) => toDate(v) }
    ],
    []
  );

  const toFilterPayload = (v: FilterValues): ShopProductSkuListQuery => ({ platform: v.platform, shop_ids: v.shop_ids, keyword: v.keyword?.trim() || undefined, status: normalizeStatus(v.status), match_status: v.match_status, mapping_status: v.mapping_status, updated_from: v.updated_range?.[0]?.toISOString(), updated_to: v.updated_range?.[1]?.toISOString() });

  return (
    <>
      <WorkbenchListPage
        className="shop-product-match-page"
        query={<Space direction="vertical" size={16} style={{ width: '100%' }}><Card size="small" className="shop-product-match-overview-card"><Row gutter={[12, 12]}><Col xs={12} md={8} xl={4}><Statistic title="查询总量" value={stats.total} /></Col><Col xs={12} md={8} xl={4}><Statistic title="已匹配" value={stats.matched} valueStyle={{ color: '#1677ff' }} /></Col><Col xs={12} md={8} xl={4}><Statistic title="未匹配" value={stats.unmatched} valueStyle={{ color: '#fa8c16' }} /></Col><Col xs={12} md={8} xl={4}><Statistic title="冲突" value={stats.conflict} valueStyle={{ color: '#cf1322' }} /></Col><Col xs={12} md={8} xl={4}><Statistic title="已勾选" value={stats.selected} valueStyle={{ color: '#13a8a8' }} /></Col></Row><Typography.Text type="secondary" className="shop-product-match-overview-hint">使用平台 SKU 名称与 SKU 商家编码，匹配 ERP 商品/组合装编码。</Typography.Text></Card><Card size="small" title="查询条件" className="shop-product-match-filter-card"><Form form={filterForm} layout="vertical" onFinish={(v) => { setFilters(v); setPagination((p) => ({ ...p, current: 1 })); }}><Row gutter={[12, 0]}><Col xs={24} md={12}><Form.Item name="platform" label="平台"><Select allowClear options={platformOptions} placeholder="选择平台" loading={shopLoading} /></Form.Item></Col><Col xs={24} md={12}><Form.Item name="shop_ids" label="店铺"><Select mode="multiple" allowClear options={shopOptions} placeholder="选择店铺" loading={shopLoading} /></Form.Item></Col><Col xs={24}><Form.Item name="keyword" label="关键词"><Input data-testid="shop-products-filter-keyword" allowClear placeholder="平台SKU名称 / SKU商家编码" /></Form.Item></Col><Col xs={24} md={8}><Form.Item name="status" label="商品状态"><Select allowClear options={statusOptions} placeholder="选择商品状态" /></Form.Item></Col><Col xs={24} md={8}><Form.Item name="match_status" label="匹配状态"><Select data-testid="shop-products-filter-match-status" allowClear options={matchStatusOptions} placeholder="选择匹配状态" /></Form.Item></Col><Col xs={24} md={8}><Form.Item name="mapping_status" label="映射状态"><Select data-testid="shop-products-filter-mapping-status" allowClear options={mappingStatusOptions} placeholder="选择映射状态" /></Form.Item></Col><Col xs={24}><Form.Item name="updated_range" label="平台更新时间"><DatePicker.RangePicker style={{ width: '100%' }} /></Form.Item></Col><Col xs={24}><Space><Button data-testid="shop-products-filter-submit" type="primary" htmlType="submit">查询</Button><Button data-testid="shop-products-filter-reset" onClick={() => { filterForm.resetFields(); setFilters({}); setPagination((p) => ({ ...p, current: 1 })); }}>重置</Button></Space></Col></Row></Form></Card>{apparelRule ? <Alert type="info" showIcon data-testid="shop-products-apparel-rule-summary" message="鞋服模板字典已启用" description={`模板：${String(apparelRule.rule_json?.template_name ?? '鞋服模板')}`} /> : null}</Space>}
        toolbarLeft={<Space size={8} wrap className="shop-product-match-toolbar-left"><Typography.Title level={4} style={{ margin: 0 }}>{pageTitle}</Typography.Title><Tag color="processing">已选择 {selectedRowKeys.length} 条</Tag><Dropdown.Button type="primary" disabled={!canWrite} onClick={() => { setDownloadAutoMatch(false); setDownloadOpen(true); downloadForm.setFieldsValue(filters); }} menu={{ items: [{ key: 'auto', label: '下载并自动匹配' }], onClick: () => { setDownloadAutoMatch(true); setDownloadOpen(true); downloadForm.setFieldsValue(filters); } }}>下载平台SKU</Dropdown.Button><Dropdown menu={{ items: [{ key: 'selected', label: '按勾选创建ERP档案' }, { key: 'filtered', label: '按筛选创建ERP档案' }], onClick: ({ key }) => { setCreateScope(key as 'selected' | 'filtered'); setCreateOpen(true); createForm.setFieldsValue({ import_image: true, import_price: true, import_stock: false }); } }}><Button disabled={!canWrite}>创建ERP档案</Button></Dropdown><Dropdown menu={{ items: [{ key: 'selected', label: <span data-testid="shop-products-auto-match-selected">按勾选自动匹配</span> }, { key: 'filtered', label: <span data-testid="shop-products-auto-match-filtered">按筛选自动匹配</span> }], onClick: async ({ key }) => { try { if (key === 'selected' && !selectedRowKeys.length) { message.warning('请先选择需要处理的平台 SKU'); return; } setAutoMatchLoading(true); const payload: ShopProductAutoMatchPayload = { sku_record_ids: key === 'selected' ? selectedRowKeys : undefined, filters: key === 'filtered' ? toFilterPayload(filters) : undefined }; const res = await autoMatchShopProducts(tenantId, payload); setAutoMatchFailures(res.failures ?? []); setSelectedRowKeys([]); setSelectedRows([]); await refreshData(); setTaskDrawerOpen(!(res.failures?.length ?? 0)); message.success('自动匹配任务已提交'); } catch (e) { if (e instanceof Error) message.error(e.message || '自动匹配失败'); } finally { setAutoMatchLoading(false); } } }}><Button data-testid="shop-products-auto-match-button" loading={autoMatchLoading} disabled={!canWrite}>自动匹配</Button></Dropdown><Button data-testid="shop-products-manual-match-button" disabled={!canWrite} onClick={() => { manualForm.resetFields(); setManualOpen(true); }}>手动匹配</Button><Popconfirm title="确认删除所选匹配关系吗？" onConfirm={async () => { try { if (!selectedRowKeys.length) { message.warning('请先选择需要处理的平台 SKU'); return; } setDeleteLoading(true); const payload: ShopProductDeleteMatchPayload = { sku_record_ids: selectedRowKeys }; await deleteShopProductMatches(tenantId, payload); setSelectedRowKeys([]); setSelectedRows([]); await refreshData(); setTaskDrawerOpen(true); message.success('匹配关系已删除'); } catch (e) { if (e instanceof Error) message.error(e.message || '删除匹配失败'); } finally { setDeleteLoading(false); } }} okButtonProps={{ loading: deleteLoading }}><Button disabled={!canWrite}>删除匹配</Button></Popconfirm></Space>}
        toolbarRight={<UiModule02ToolbarRight onExport={() => message.info('导出功能待接入')} onPrint={() => message.info('打印功能待接入')} onHelp={() => message.info('帮助功能待接入')} />}
        detailGroups={[{ key: 'workflow', label: '处理流程', children: <Space direction="vertical" size={4}><Typography.Text>1. 下载平台SKU</Typography.Text><Typography.Text>2. 自动匹配批量记录</Typography.Text><Typography.Text>3. 手动处理冲突与未匹配</Typography.Text></Space> }, { key: 'tasks', label: '任务中心', children: <Button size="small" onClick={() => setTaskDrawerOpen(true)}>查看任务</Button> }]}
        table={<ServerTable<ShopProductSku> rowKey="sku_record_id" columns={columns} data={data?.items ?? []} loading={isLoading || isFetching} error={error} onRetry={() => refetch()} errorTitle={`${pageTitle}加载失败`} emptyText={`暂无${pageTitle}`} total={data?.total ?? 0} pagination={pagination} onPaginationChange={setPagination} rowSelection={{ selectedRowKeys, onChange: (keys, rows) => { setSelectedRowKeys(keys as string[]); setSelectedRows(rows as ShopProductSku[]); } }} columnSettingsKey="shop-products.table.columns" />}
      />

      <Modal title="自动匹配失败" open={autoMatchFailures.length > 0} onCancel={() => setAutoMatchFailures([])} onOk={() => setAutoMatchFailures([])} okText="知道了" cancelButtonProps={{ style: { display: 'none' } }}><Space direction="vertical" size={8} style={{ width: '100%' }}>{autoMatchFailures.slice(0, 6).map((x) => <Typography.Text key={x.sku_record_id}>{`${x.platform_sku_id}: ${x.reason}`}</Typography.Text>)}</Space></Modal>

      <Modal title={downloadAutoMatch ? '下载平台SKU并自动匹配' : '下载平台SKU'} open={downloadOpen} onCancel={() => setDownloadOpen(false)} onOk={async () => { try { const values = await downloadForm.validateFields(); setDownloadLoading(true); const payload: ShopProductDownloadPayload = { platform: values.platform, shop_ids: values.shop_ids, keyword: values.keyword?.trim() || undefined, status: normalizeStatus(values.status), updated_from: values.updated_range?.[0]?.toISOString(), updated_to: values.updated_range?.[1]?.toISOString(), auto_match: downloadAutoMatch }; const res = await downloadShopProducts(tenantId, payload); setDownloadOpen(false); await refreshData(); setTaskDrawerOpen(true); message.success('下载任务已创建'); if (res.auto_match_failures?.length) { Modal.warning({ title: '自动匹配失败', content: <div>{res.auto_match_failures.slice(0, 6).map((item) => <div key={item.sku_record_id}>{`${item.platform_sku_id}: ${item.reason}`}</div>)}</div> }); } } catch (e) { if (e instanceof Error) message.error(e.message || '下载失败'); } finally { setDownloadLoading(false); } }} confirmLoading={downloadLoading} okText="开始下载"><Form form={downloadForm} layout="vertical"><Row gutter={16}><Col xs={24} md={12}><Form.Item name="platform" label="平台"><Select allowClear options={platformOptions} placeholder="选择平台" loading={shopLoading} /></Form.Item></Col><Col xs={24} md={12}><Form.Item name="shop_ids" label="店铺" rules={[{ required: true, message: '请选择店铺' }]}><Select mode="multiple" allowClear options={shopOptions} placeholder="选择店铺" loading={shopLoading} /></Form.Item></Col><Col xs={24}><Form.Item name="keyword" label="关键词"><Input allowClear placeholder="平台SKU名称 / SKU商家编码" /></Form.Item></Col><Col xs={24} md={12}><Form.Item name="status" label="商品状态"><Select allowClear options={statusOptions} placeholder="选择商品状态" /></Form.Item></Col><Col xs={24} md={12}><Form.Item name="updated_range" label="平台更新时间"><DatePicker.RangePicker style={{ width: '100%' }} showTime /></Form.Item></Col></Row></Form></Modal>

      <Modal title={createScope === 'selected' ? '创建ERP档案（按勾选）' : '创建ERP档案（按筛选）'} open={createOpen} onCancel={() => setCreateOpen(false)} onOk={async () => { try { const values = await createForm.validateFields(); if (createScope === 'selected' && !selectedRowKeys.length) { message.warning('请先选择需要处理的平台 SKU'); return; } setCreateLoading(true); const payload: ShopProductCreateSkuPayload = { sku_record_ids: createScope === 'selected' ? selectedRowKeys : undefined, filters: createScope === 'filtered' ? toFilterPayload(filters) : undefined, import_price: values.import_price, import_image: values.import_image, import_stock: values.import_stock, warehouse_id: values.warehouse_id }; await createShopProductSkus(tenantId, payload); setCreateOpen(false); setSelectedRowKeys([]); setSelectedRows([]); await refreshData(); setTaskDrawerOpen(true); message.success('ERP档案创建任务已提交'); } catch (e) { if (e instanceof Error) message.error(e.message || '创建失败'); } finally { setCreateLoading(false); } }} confirmLoading={createLoading} okText="提交创建"><Form form={createForm} layout="vertical"><Form.Item name="import_price" valuePropName="checked"><Checkbox>导入网店价格</Checkbox></Form.Item><Form.Item name="import_image" valuePropName="checked"><Checkbox>导入商品图片</Checkbox></Form.Item><Form.Item name="import_stock" valuePropName="checked"><Checkbox>导入网店库存</Checkbox></Form.Item>{importStock ? <Form.Item name="warehouse_id" label="存放仓库" rules={[{ required: true, message: '请选择仓库' }]}><Select allowClear options={warehouseOptions} placeholder="选择仓库" loading={warehouseLoading} /></Form.Item> : null}</Form></Modal>

      <Modal title="手动匹配" open={manualOpen} onCancel={() => setManualOpen(false)} onOk={async () => { try { const values = await manualForm.validateFields(); if (!selectedRowKeys.length) { message.warning('请先选择需要处理的平台 SKU'); return; } if (!values.sku_id) { message.warning('请选择ERP商品编码'); return; } setManualLoading(true); const payload: ShopProductManualMatchPayload = { sku_record_ids: selectedRowKeys, sku_id: values.sku_id }; await manualMatchShopProducts(tenantId, payload); setManualOpen(false); setSelectedRowKeys([]); setSelectedRows([]); await refreshData(); setTaskDrawerOpen(true); message.success('手动匹配完成'); } catch (e) { if (e instanceof Error) message.error(e.message || '手动匹配失败'); } finally { setManualLoading(false); } }} confirmLoading={manualLoading} okText="提交匹配"><Form form={manualForm} layout="vertical"><Form.Item name="sku_id" label="选择ERP商品编码" rules={[{ required: true, message: '请选择ERP商品编码' }]}><Select data-testid="shop-products-manual-match-sku-select" showSearch loading={skuLoading} optionFilterProp="label" placeholder="选择ERP商品" options={(skuData?.data ?? []).map((s) => ({ value: s.sku_id, label: `${s.sku_code} ${s.name ?? ''}` }))} /></Form.Item><Typography.Text type="secondary">当前已勾选 {selectedRows.length} 条平台SKU。</Typography.Text></Form></Modal>

      <Drawer title="商品处理任务" open={taskDrawerOpen} onClose={() => setTaskDrawerOpen(false)} width={720}><ServerTable<ShopProductTask> rowKey="task_id" columns={taskColumns} data={taskData?.items ?? []} loading={taskFetching} error={taskError} onRetry={() => refetchTasks()} errorTitle="任务加载失败" emptyText="暂无任务" total={taskData?.total ?? 0} pagination={taskPagination} onPaginationChange={setTaskPagination} columnSettingsKey="shop-products.task.columns" /></Drawer>
    </>
  );
};
