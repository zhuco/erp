import {
  Button,
  Card,
  Checkbox,
  Descriptions,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Tag,
  Tabs,
  Typography,
  message,
} from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useCallback, useEffect, useMemo, useState } from 'react';

import type { WarehouseArea } from '@/api/warehouseAreas';
import { listWarehouseAreas } from '@/api/warehouseAreas';
import type {
  WarehouseLocation,
  WarehouseLocationBatchItem,
  WarehouseLocationBatchPayload,
  WarehouseLocationCreatePayload,
  WarehouseLocationListQuery,
  WarehouseLocationUpdatePayload,
} from '@/api/warehouseLocations';
import {
  batchCreateWarehouseLocations,
  createWarehouseLocation,
  deleteWarehouseLocation,
  listWarehouseLocations,
  updateWarehouseLocation,
} from '@/api/warehouseLocations';
import type { ErpDataGridColumn } from '@/components/ErpDataGrid';
import { ErpDataGridNext } from '@/components/ErpDataGridNext';
import {
  WorkbenchLeftPanelTemplate,
  WorkbenchPageTemplate,
  WorkbenchToolbar,
  type WorkbenchLeftRenderRow,
  type WorkbenchLeftStatsItem,
  useWorkbenchTablePagination,
} from '@/components/page-templates';
import { UiModule05DetailPanel } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';
import {
  buildStatsPieBackground,
  normalizeSelectionKeys,
  renderDirectoryBottomTable,
  sliceRowsByPagination,
  trimText,
  type DirectoryBottomDetailRow,
} from '@/pages/shared/directoryWorkbenchUtils';

type LocationFormValues = {
  warehouse_id: string;
  area_id?: string;
  code: string;
  name?: string;
  barcode?: string;
  volume?: number;
  max_weight?: number;
  is_picking?: boolean;
  is_disabled?: boolean;
};

type BatchFormValues = {
  warehouse_id: string;
  area_id?: string;
  raw: string;
};

type GridFilters = {
  warehouse_id: string;
  code: string;
  barcode: string;
};

type LeftFilterKey =
  | 'view:all'
  | 'picking:yes'
  | 'disabled:yes'
  | 'disabled:no'
  | 'warehouse:WH-DEFAULT'
  | 'warehouse:WH-EAST';

const pageSizeOptions = ['20', '50', '100'];

const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

const createDefaultGridFilters = (): GridFilters => ({
  warehouse_id: '',
  code: '',
  barcode: '',
});

const buildGridFilterModel = (filters: GridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.warehouse_id)) {
    model.warehouse_id = { filterType: 'text', type: 'contains', filter: trimText(filters.warehouse_id) };
  }
  if (trimText(filters.code)) {
    model.code = { filterType: 'text', type: 'contains', filter: trimText(filters.code) };
  }
  if (trimText(filters.barcode)) {
    model.barcode = { filterType: 'text', type: 'contains', filter: trimText(filters.barcode) };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): GridFilters => ({
  warehouse_id:
    model?.warehouse_id && typeof model.warehouse_id === 'object' && 'filter' in model.warehouse_id
      ? trimText(String(model.warehouse_id.filter ?? ''))
      : '',
  code:
    model?.code && typeof model.code === 'object' && 'filter' in model.code
      ? trimText(String(model.code.filter ?? ''))
      : '',
  barcode:
    model?.barcode && typeof model.barcode === 'object' && 'filter' in model.barcode
      ? trimText(String(model.barcode.filter ?? ''))
      : '',
});

const parseBoolean = (value?: string) => {
  if (!value) return undefined;
  const normalized = value.trim().toLowerCase();
  if (['1', 'true', 'yes', 'y'].includes(normalized)) return true;
  if (['0', 'false', 'no', 'n'].includes(normalized)) return false;
  return undefined;
};

const parseNumber = (value?: string) => {
  if (!value) return undefined;
  const parsed = Number(value.trim());
  return Number.isFinite(parsed) ? parsed : undefined;
};

export const WarehouseLocationsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<LocationFormValues>();
  const [batchForm] = Form.useForm<BatchFormValues>();
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<LeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<GridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [batchOpen, setBatchOpen] = useState(false);
  const [editingLocation, setEditingLocation] = useState<WarehouseLocation | null>(null);
  const [activeLocationId, setActiveLocationId] = useState<string | null>(null);
  const [selectedLocationIds, setSelectedLocationIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantReady ? `wms.warehouse-locations.pagination.${tenantId}` : undefined,
  });

  const queryPayload = useMemo<WarehouseLocationListQuery>(
    () => ({ keyword: trimText(searchKeyword) || undefined, offset: 0, limit: 500 }),
    [searchKeyword],
  );

  const query = useQuery({
    queryKey: ['warehouse-locations', tenantId, queryPayload],
    queryFn: () => listWarehouseLocations(tenantId, queryPayload),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData,
  });

  const areasQuery = useQuery({
    queryKey: ['warehouse-areas-all', tenantId],
    queryFn: () => listWarehouseAreas(tenantId, { offset: 0, limit: 500 }),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData,
  });

  const areaMap = useMemo(() => {
    const map = new Map<string, WarehouseArea>();
    (areasQuery.data?.items ?? []).forEach((area) => map.set(area.area_id, area));
    return map;
  }, [areasQuery.data]);

  const areaOptions = useMemo(
    () =>
      (areasQuery.data?.items ?? []).map((area) => ({
        label: `${area.code} ${area.name}`,
        value: area.area_id,
      })),
    [areasQuery.data],
  );

  const saveMutation = useMutation({
    mutationFn: async (values: LocationFormValues) => {
      if (editingLocation) {
        const payload: WarehouseLocationUpdatePayload = {
          area_id: values.area_id?.trim() || undefined,
          name: values.name?.trim() || undefined,
          barcode: values.barcode?.trim() || undefined,
          volume: values.volume ?? undefined,
          max_weight: values.max_weight ?? undefined,
          is_picking: values.is_picking,
          is_disabled: values.is_disabled,
        };
        return updateWarehouseLocation(tenantId, editingLocation.location_id, payload);
      }
      const payload: WarehouseLocationCreatePayload = {
        warehouse_id: values.warehouse_id.trim(),
        area_id: values.area_id?.trim() || undefined,
        code: values.code.trim(),
        name: values.name?.trim() || undefined,
        barcode: values.barcode?.trim() || undefined,
        volume: values.volume ?? undefined,
        max_weight: values.max_weight ?? undefined,
        is_picking: values.is_picking ?? false,
        is_disabled: values.is_disabled ?? false,
      };
      return createWarehouseLocation(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editingLocation ? '货位已更新' : '货位已创建');
      setDrawerOpen(false);
      setEditingLocation(null);
      await queryClient.invalidateQueries({ queryKey: ['warehouse-locations', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存货位失败');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: async (locationIds: string[]) => {
      await Promise.all(locationIds.map((locationId) => deleteWarehouseLocation(tenantId, locationId)));
    },
    onSuccess: async (_data, locationIds) => {
      message.success(`已删除 ${locationIds.length} 个货位`);
      setSelectedLocationIds([]);
      await queryClient.invalidateQueries({ queryKey: ['warehouse-locations', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '删除货位失败');
    },
  });

  const batchMutation = useMutation({
    mutationFn: async (payload: WarehouseLocationBatchPayload) => batchCreateWarehouseLocations(tenantId, payload),
    onSuccess: async () => {
      message.success('货位批量导入完成');
      setBatchOpen(false);
      batchForm.resetFields();
      await queryClient.invalidateQueries({ queryKey: ['warehouse-locations', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '批量导入货位失败');
    },
  });

  const sourceRows = useMemo(() => query.data?.items ?? [], [query.data]);
  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部货位', value: sourceRows.length, color: '#2f6fed' },
      { key: 'picking', label: '拣货位', value: sourceRows.filter((row) => row.is_picking).length, color: '#0f9d58' },
      { key: 'disabled', label: '禁用', value: sourceRows.filter((row) => row.is_disabled).length, color: '#d48806' },
    ],
    [sourceRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      { key: 'view:all', label: '全部货位', count: sourceRows.length, nodeType: 'warehouse', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'view:all' },
      { key: 'picking:yes', label: '拣货位', count: sourceRows.filter((row) => row.is_picking).length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'picking:yes' },
      { key: 'disabled:yes', label: '已禁用', count: sourceRows.filter((row) => row.is_disabled).length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'disabled:yes' },
      { key: 'disabled:no', label: '启用中', count: sourceRows.filter((row) => !row.is_disabled).length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'disabled:no' },
      { key: 'warehouse:WH-DEFAULT', label: '默认仓货位', count: sourceRows.filter((row) => row.warehouse_id === 'WH-DEFAULT').length, nodeType: 'warehouse', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'warehouse:WH-DEFAULT' },
      { key: 'warehouse:WH-EAST', label: '华东仓货位', count: sourceRows.filter((row) => row.warehouse_id === 'WH-EAST').length, nodeType: 'warehouse', level: 0, hasChildren: false, expanded: false, checked: leftFilterKey === 'warehouse:WH-EAST' },
    ],
    [leftFilterKey, sourceRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return sourceRows.filter((row) => {
      if (leftFilterKey === 'picking:yes' && !row.is_picking) return false;
      if (leftFilterKey === 'disabled:yes' && !row.is_disabled) return false;
      if (leftFilterKey === 'disabled:no' && row.is_disabled) return false;
      if (leftFilterKey.startsWith('warehouse:') && row.warehouse_id !== leftFilterKey.replace('warehouse:', '')) return false;
      if (!keyword) return true;
      return [row.code, row.name ?? '', row.barcode ?? '', row.warehouse_id]
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword));
    });
  }, [leftFilterKey, leftKeyword, sourceRows]);

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (trimText(gridFilters.warehouse_id) && !row.warehouse_id.toLowerCase().includes(trimText(gridFilters.warehouse_id).toLowerCase())) return false;
        if (trimText(gridFilters.code) && !row.code.toLowerCase().includes(trimText(gridFilters.code).toLowerCase())) return false;
        if (trimText(gridFilters.barcode) && !(row.barcode ?? '').toLowerCase().includes(trimText(gridFilters.barcode).toLowerCase())) return false;
        return true;
      }),
    [gridFilters, leftFilteredRows],
  );

  const pageRows = useMemo(() => sliceRowsByPagination(displayRows, pagination), [displayRows, pagination]);

  useEffect(() => {
    setActiveLocationId((previous) =>
      previous && pageRows.some((row) => row.location_id === previous) ? previous : pageRows[0]?.location_id ?? null,
    );
    setSelectedLocationIds((previous) =>
      previous.filter((item) => pageRows.some((row) => row.location_id === item)),
    );
  }, [pageRows]);

  const activeLocation = useMemo(
    () => pageRows.find((row) => row.location_id === activeLocationId) ?? null,
    [activeLocationId, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<WarehouseLocation>[]>(
    () => [
      { key: 'code', field: 'code', title: '货位编码', width: 150, minWidth: 120, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'warehouse_id', field: 'warehouse_id', title: '仓库ID', width: 140, minWidth: 120, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'name', field: 'name', title: '货位名称', width: 160, minWidth: 140, renderCell: (value) => String(value ?? '-') },
      {
        key: 'area_id',
        field: 'area_id',
        title: '库区',
        width: 180,
        minWidth: 150,
        renderCell: (value) => {
          if (!value) return '-';
          const area = areaMap.get(String(value));
          return area ? `${area.code} ${area.name}` : String(value);
        },
      },
      { key: 'barcode', field: 'barcode', title: '条码', width: 150, minWidth: 130, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 }, renderCell: (value) => String(value ?? '-') },
      { key: 'is_picking', field: 'is_picking', title: '拣货位', width: 100, minWidth: 90, renderCell: (value) => (value ? <Tag color="blue">是</Tag> : '否') },
      { key: 'is_disabled', field: 'is_disabled', title: '禁用', width: 100, minWidth: 90, renderCell: (value) => (value ? <Tag color="red">禁用</Tag> : '启用') },
      { key: 'updated_at', field: 'updated_at', title: '更新时间', width: 170, minWidth: 160, renderCell: (_value, record) => formatDateTime(record.updated_at ?? record.created_at) },
    ],
    [areaMap],
  );

  const openCreateDrawer = useCallback(() => {
    setEditingLocation(null);
    form.resetFields();
    form.setFieldsValue({ is_picking: false, is_disabled: false });
    setDrawerOpen(true);
  }, [form]);

  const openEditDrawer = useCallback((location: WarehouseLocation) => {
    setEditingLocation(location);
    form.setFieldsValue({
      warehouse_id: location.warehouse_id,
      area_id: location.area_id ?? undefined,
      code: location.code,
      name: location.name ?? undefined,
      barcode: location.barcode ?? undefined,
      volume: location.volume ?? undefined,
      max_weight: location.max_weight ?? undefined,
      is_picking: location.is_picking,
      is_disabled: location.is_disabled,
    });
    setDrawerOpen(true);
  }, [form]);

  const handleSave = useCallback(async () => {
    try {
      const values = await form.validateFields();
      await saveMutation.mutateAsync(values);
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    }
  }, [form, saveMutation]);

  const handleDelete = useCallback(async (locationIds: string[]) => {
    if (!locationIds.length) {
      message.warning('请先选择要删除的货位');
      return;
    }
    await deleteMutation.mutateAsync(locationIds);
  }, [deleteMutation]);

  const handleBatchImport = useCallback(async () => {
    try {
      const values = await batchForm.validateFields();
      const lines = values.raw.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
      const locations: WarehouseLocationBatchItem[] = [];
      lines.forEach((line, index) => {
        const parts = line.split(',').map((part) => part.trim());
        if (!parts[0]) throw new Error(`第 ${index + 1} 行缺少货位编码`);
        locations.push({
          code: parts[0],
          name: parts[1] || undefined,
          barcode: parts[2] || undefined,
          volume: parseNumber(parts[3]),
          max_weight: parseNumber(parts[4]),
          is_picking: parseBoolean(parts[5]),
          is_disabled: parseBoolean(parts[6]),
          area_id: parts[7] || undefined,
        });
      });
      if (!locations.length) {
        message.error('未解析到有效货位');
        return;
      }
      const payload: WarehouseLocationBatchPayload = {
        warehouse_id: values.warehouse_id.trim(),
        area_id: values.area_id?.trim() || undefined,
        locations,
      };
      await batchMutation.mutateAsync(payload);
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    }
  }, [batchForm, batchMutation]);

  const targetIds = selectedLocationIds.length > 0 ? selectedLocationIds : activeLocationId ? [activeLocationId] : [];
  const targetLocation = pageRows.find((row) => row.location_id === targetIds[0]) ?? activeLocation;

  const rightPanel = activeLocation ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'ops']}
      panels={[
        {
          key: 'detail',
          label: '货位详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="货位编码">{activeLocation.code}</Descriptions.Item>
              <Descriptions.Item label="仓库ID">{activeLocation.warehouse_id}</Descriptions.Item>
              <Descriptions.Item label="货位名称">{activeLocation.name || '-'}</Descriptions.Item>
              <Descriptions.Item label="库区">
                {activeLocation.area_id ? (areaMap.get(activeLocation.area_id)?.name ?? activeLocation.area_id) : '-'}
              </Descriptions.Item>
              <Descriptions.Item label="条码">{activeLocation.barcode || '-'}</Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'ops',
          label: '操作约束',
          children: (
            <Typography.Text type="secondary">
              货位目录已切换到统一工业工作台结构，批量导入与新建、编辑统一由工具栏触发，主表保持紧凑规范。
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="货位详情">
      <Typography.Text type="secondary">请选择一条货位记录查看详情。</Typography.Text>
    </Card>
  );

  const profileRows: DirectoryBottomDetailRow[] = activeLocation
    ? [
        { key: 'code', field: '货位编码', value: activeLocation.code },
        { key: 'warehouse', field: '仓库ID', value: activeLocation.warehouse_id },
        { key: 'name', field: '货位名称', value: activeLocation.name || '-' },
      ]
    : [];
  const configRows: DirectoryBottomDetailRow[] = activeLocation
    ? [
        { key: 'barcode', field: '条码', value: activeLocation.barcode || '-' },
        { key: 'volume', field: '容积', value: String(activeLocation.volume ?? '-') },
        { key: 'weight', field: '承重', value: String(activeLocation.max_weight ?? '-') },
        { key: 'picking', field: '拣货位', value: activeLocation.is_picking ? '是' : '否' },
        { key: 'disabled', field: '禁用', value: activeLocation.is_disabled ? '是' : '否' },
      ]
    : [];
  const opsRows: DirectoryBottomDetailRow[] = activeLocation
    ? [
        { key: 'selected', field: '当前勾选数', value: String(selectedLocationIds.length) },
        { key: 'search', field: '当前搜索关键字', value: trimText(searchKeyword) || '-' },
        { key: 'import', field: '批量导入', value: canWrite ? '可执行' : '只读' },
      ]
    : [];

  const bottomPanel = (
    <Tabs
      size="small"
      type="card"
      activeKey={activeBottomTab}
      onChange={setActiveBottomTab}
      items={[
        { key: 'profile', label: `基础资料 (${profileRows.length})`, children: renderDirectoryBottomTable(Boolean(activeLocation), profileRows, '暂无基础资料', `warehouse-location-profile-${activeLocation?.location_id ?? 'empty'}`) },
        { key: 'config', label: `配置明细 (${configRows.length})`, children: renderDirectoryBottomTable(Boolean(activeLocation), configRows, '暂无配置明细', `warehouse-location-config-${activeLocation?.location_id ?? 'empty'}`) },
        { key: 'ops', label: `操作上下文 (${opsRows.length})`, children: renderDirectoryBottomTable(Boolean(activeLocation), opsRows, '暂无操作上下文', `warehouse-location-ops-${activeLocation?.location_id ?? 'empty'}`) },
      ]}
    />
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="warehouse-locations-workbench-page"
        className="order-review-page warehouse-locations-directory-page"
        leftPanel={
          <WorkbenchLeftPanelTemplate
            className="warehouse-locations-left-panel"
            dataTestId="warehouse-locations-left-panel"
            leftTab={leftTab}
            onLeftTabChange={setLeftTab}
            leftRowsForRender={leftRowsForRender}
            onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
              const nextKey = event.data?.key as LeftFilterKey | undefined;
              if (!nextKey) return;
              resetPagination();
              setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
            }}
            leftStatsItems={leftStatsItems}
            leftStatsPieBackground={buildStatsPieBackground(leftStatsItems)}
            globalOrderTotal={sourceRows.length}
            statsScopeText="口径：当前货位目录查询结果统计"
            screenContent={
              <Space direction="vertical" size={10} style={{ width: '100%' }}>
                <div>
                  <Typography.Text type="secondary">货位检索</Typography.Text>
                  <Input
                    size="small"
                    allowClear
                    value={leftKeyword}
                    placeholder="按编码 / 名称 / 条码缩小左侧结果"
                    onChange={(event) => {
                      resetPagination();
                      setLeftKeyword(event.target.value);
                    }}
                  />
                </div>
                <Button size="small" onClick={() => { resetPagination(); setLeftKeyword(''); setLeftFilterKey('view:all'); }}>
                  重置左侧筛选
                </Button>
              </Space>
            }
          />
        }
        leftPanelVisible={true}
        rightPanel={rightPanel}
        rightPanelVisible={true}
        rightPanelSyncKey={activeLocation?.location_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={
              <Space size={8} wrap>
                <Button
                  size="small"
                  disabled={!tenantReady || !canWrite}
                  onClick={() => {
                    batchForm.resetFields();
                    batchForm.setFieldsValue({
                      warehouse_id: activeLocation?.warehouse_id ?? '',
                      area_id: activeLocation?.area_id ?? undefined,
                    });
                    setBatchOpen(true);
                  }}
                >
                  批量导入
                </Button>
                <Button type="primary" size="small" disabled={!tenantReady || !canWrite} onClick={openCreateDrawer}>
                  新建货位
                </Button>
                <Button
                  size="small"
                  disabled={!tenantReady || !canWrite || targetIds.length !== 1 || !targetLocation}
                  onClick={() => targetLocation && openEditDrawer(targetLocation)}
                >
                  编辑
                </Button>
                <Button
                  size="small"
                  danger
                  disabled={!tenantReady || !canWrite || targetIds.length === 0}
                  loading={deleteMutation.isPending}
                  onClick={() => {
                    void Modal.confirm({
                      title: `确认删除选中的 ${targetIds.length} 个货位吗？`,
                      okText: '删除',
                      cancelText: '取消',
                      onOk: async () => handleDelete(targetIds),
                    });
                  }}
                >
                  删除
                </Button>
              </Space>
            }
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => { setSearchKeyword(trimText(searchInput)); resetPagination(); }}
            searchPlaceholder="搜索货位编码 / 名称 / 条码"
            onExport={() => message.info('货位目录导出能力待接入')}
            onPrint={() => message.info('货位目录打印能力待接入')}
            onHelp={() => message.info('货位目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<WarehouseLocation>
            rowKey={(record) => record.location_id}
            columns={columns}
            rows={pageRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="wms.warehouse-locations.main-grid"
            dataTestId="warehouse-locations-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            loading={query.isLoading || query.isFetching}
            error={query.error}
            onRetry={() => { void query.refetch(); }}
            errorTitle="加载货位失败"
            emptyText="暂无货位"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => { resetPagination(); setGridFilters(extractGridFilterModel(model)); }}
            selectedRowKeys={selectedLocationIds}
            activeRowKey={activeLocationId}
            onSelectionChange={(keys) => setSelectedLocationIds(normalizeSelectionKeys(keys))}
            onActiveRowChange={(record) => setActiveLocationId(record.location_id)}
            onRowDoubleClick={(record) => { if (canWrite) openEditDrawer(record); }}
          />
        }
        footerSummary={
          <>
            <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedLocationIds.length} 条`}</span>
            <span>{activeLocation ? `当前焦点：${activeLocation.code}` : '当前焦点：-'}</span>
          </>
        }
        footerSyncKey={`warehouse-locations-${displayRows.length}-${pageRows.length}-${selectedLocationIds.length}`}
        total={displayRows.length}
        pagination={pagination}
        onPaginationChange={handlePaginationChange}
        showFooterScrollbarRow={true}
      />
      <Drawer
        title={editingLocation ? '编辑货位' : '新建货位'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saveMutation.isPending} onClick={() => void handleSave()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={form} layout="vertical">
          <Form.Item name="warehouse_id" label="仓库ID" rules={[{ required: true, message: '请输入仓库ID' }]}>
            <Input disabled={Boolean(editingLocation)} />
          </Form.Item>
          <Form.Item name="area_id" label="库区">
            <Select allowClear options={areaOptions} placeholder="选择库区" />
          </Form.Item>
          <Form.Item name="code" label="货位编码" rules={[{ required: true, message: '请输入货位编码' }]}>
            <Input disabled={Boolean(editingLocation)} />
          </Form.Item>
          <Form.Item name="name" label="货位名称">
            <Input />
          </Form.Item>
          <Form.Item name="barcode" label="条码">
            <Input />
          </Form.Item>
          <Form.Item name="volume" label="容积">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="max_weight" label="承重">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="is_picking" valuePropName="checked">
            <Checkbox>拣货位</Checkbox>
          </Form.Item>
          <Form.Item name="is_disabled" valuePropName="checked">
            <Checkbox>禁用</Checkbox>
          </Form.Item>
        </Form>
      </Drawer>
      <Modal
        title="批量导入货位"
        open={batchOpen}
        onCancel={() => setBatchOpen(false)}
        onOk={() => void handleBatchImport()}
        confirmLoading={batchMutation.isPending}
        okText="导入"
        width={720}
      >
        <Form form={batchForm} layout="vertical">
          <Form.Item name="warehouse_id" label="仓库ID" rules={[{ required: true, message: '请输入仓库ID' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="area_id" label="默认库区ID">
            <Input placeholder="可选，所有行默认使用该库区" />
          </Form.Item>
          <Form.Item name="raw" label="货位列表" rules={[{ required: true, message: '请输入货位数据' }]}>
            <Input.TextArea rows={8} placeholder="编码,名称,条码,容积,承重,拣货位(1/0),禁用(1/0),库区ID" />
          </Form.Item>
          <Typography.Text type="secondary">
            每行一条，字段以英文逗号分隔，库区ID可留空使用默认值。
          </Typography.Text>
        </Form>
      </Modal>
    </>
  );
};
