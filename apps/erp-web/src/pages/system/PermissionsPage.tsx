import { Button, Card, Descriptions, Input, Space, Tabs, Typography, message } from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import { useQuery } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';

import { listPermissions, type PermissionDefinition } from '@/api/system';
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
import { isTenantSelected, useTenantId } from '@/features/tenant';
import {
  buildStatsPieBackground,
  normalizeSelectionKeys,
  renderDirectoryBottomTable,
  sliceRowsByPagination,
  trimText,
  type DirectoryBottomDetailRow,
} from '@/pages/shared/directoryWorkbenchUtils';

type PermissionsGridFilters = {
  code: string;
  module: string;
  name: string;
};

type PermissionLeftFilterKey =
  | 'view:all'
  | 'module:订单中心'
  | 'module:WMS'
  | 'module:物流中心'
  | 'module:采购中心'
  | 'module:系统管理'
  | 'module:渠道中心';

const pageSizeOptions = ['20', '50', '100'];

const createDefaultGridFilters = (): PermissionsGridFilters => ({
  code: '',
  module: '',
  name: '',
});

const buildGridFilterModel = (filters: PermissionsGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.code)) {
    model.code = { filterType: 'text', type: 'contains', filter: trimText(filters.code) };
  }
  if (trimText(filters.module)) {
    model.module = { filterType: 'text', type: 'contains', filter: trimText(filters.module) };
  }
  if (trimText(filters.name)) {
    model.name = { filterType: 'text', type: 'contains', filter: trimText(filters.name) };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): PermissionsGridFilters => ({
  code:
    model?.code && typeof model.code === 'object' && 'filter' in model.code
      ? trimText(String(model.code.filter ?? ''))
      : '',
  module:
    model?.module && typeof model.module === 'object' && 'filter' in model.module
      ? trimText(String(model.module.filter ?? ''))
      : '',
  name:
    model?.name && typeof model.name === 'object' && 'filter' in model.name
      ? trimText(String(model.name.filter ?? ''))
      : '',
});

export const PermissionsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftFilterKey, setLeftFilterKey] = useState<PermissionLeftFilterKey>('view:all');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [gridFilters, setGridFilters] = useState<PermissionsGridFilters>(createDefaultGridFilters);
  const [activePermissionCode, setActivePermissionCode] = useState<string | null>(null);
  const [selectedPermissionCodes, setSelectedPermissionCodes] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');

  const { pagination, handlePaginationChange, resetPagination } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions,
    storageKey: tenantReady ? `system.permissions.pagination.${tenantId}` : undefined,
  });

  const query = useQuery({
    queryKey: ['permissions', tenantId],
    queryFn: () => listPermissions(tenantId),
    enabled: tenantReady,
  });

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const sourceRows = useMemo(() => {
    const keyword = trimText(searchKeyword).toLowerCase();
    const rows = query.data ?? [];
    if (!keyword) {
      return rows;
    }
    return rows.filter((row) =>
      [row.code, row.module, row.name, row.description]
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword)),
    );
  }, [query.data, searchKeyword]);

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部权限', value: sourceRows.length, color: '#2f6fed' },
      {
        key: 'system',
        label: '系统管理',
        value: sourceRows.filter((row) => row.module === '系统管理').length,
        color: '#0f9d58',
      },
      {
        key: 'wms',
        label: 'WMS',
        value: sourceRows.filter((row) => row.module === 'WMS').length,
        color: '#d48806',
      },
    ],
    [sourceRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      {
        key: 'view:all',
        label: '全部权限',
        count: sourceRows.length,
        nodeType: 'tag',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'view:all',
      },
      {
        key: 'module:订单中心',
        label: '订单中心',
        count: sourceRows.filter((row) => row.module === '订单中心').length,
        nodeType: 'group',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'module:订单中心',
      },
      {
        key: 'module:WMS',
        label: 'WMS',
        count: sourceRows.filter((row) => row.module === 'WMS').length,
        nodeType: 'warehouse',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'module:WMS',
      },
      {
        key: 'module:物流中心',
        label: '物流中心',
        count: sourceRows.filter((row) => row.module === '物流中心').length,
        nodeType: 'logistics',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'module:物流中心',
      },
      {
        key: 'module:采购中心',
        label: '采购中心',
        count: sourceRows.filter((row) => row.module === '采购中心').length,
        nodeType: 'shop',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'module:采购中心',
      },
      {
        key: 'module:系统管理',
        label: '系统管理',
        count: sourceRows.filter((row) => row.module === '系统管理').length,
        nodeType: 'status',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'module:系统管理',
      },
      {
        key: 'module:渠道中心',
        label: '渠道中心',
        count: sourceRows.filter((row) => row.module === '渠道中心').length,
        nodeType: 'shop',
        level: 0,
        hasChildren: false,
        expanded: false,
        checked: leftFilterKey === 'module:渠道中心',
      },
    ],
    [leftFilterKey, sourceRows],
  );

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return sourceRows.filter((row) => {
      if (leftFilterKey !== 'view:all') {
        const targetModule = leftFilterKey.replace('module:', '');
        if (row.module !== targetModule) {
          return false;
        }
      }
      if (!keyword) {
        return true;
      }
      return [row.code, row.module, row.name, row.description]
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword));
    });
  }, [leftFilterKey, leftKeyword, sourceRows]);

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (
          trimText(gridFilters.code) &&
          !row.code.toLowerCase().includes(trimText(gridFilters.code).toLowerCase())
        ) {
          return false;
        }
        if (
          trimText(gridFilters.module) &&
          !row.module.toLowerCase().includes(trimText(gridFilters.module).toLowerCase())
        ) {
          return false;
        }
        if (
          trimText(gridFilters.name) &&
          !row.name.toLowerCase().includes(trimText(gridFilters.name).toLowerCase())
        ) {
          return false;
        }
        return true;
      }),
    [gridFilters, leftFilteredRows],
  );

  const pageRows = useMemo(
    () => sliceRowsByPagination(displayRows, pagination),
    [displayRows, pagination],
  );

  useEffect(() => {
    setActivePermissionCode((previous) =>
      previous && pageRows.some((item) => item.code === previous) ? previous : pageRows[0]?.code ?? null,
    );
    setSelectedPermissionCodes((previous) =>
      previous.filter((item) => pageRows.some((row) => row.code === item)),
    );
  }, [pageRows]);

  const activePermission = useMemo(
    () => pageRows.find((item) => item.code === activePermissionCode) ?? null,
    [activePermissionCode, pageRows],
  );

  const columns = useMemo<ErpDataGridColumn<PermissionDefinition>[]>(
    () => [
      {
        key: 'code',
        field: 'code',
        title: '权限编码',
        width: 220,
        minWidth: 180,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'module',
        field: 'module',
        title: '模块',
        width: 140,
        minWidth: 120,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'name',
        field: 'name',
        title: '名称',
        width: 160,
        minWidth: 140,
        filter: 'agTextColumnFilter',
        filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 },
      },
      {
        key: 'description',
        field: 'description',
        title: '描述',
        width: 360,
        minWidth: 280,
      },
    ],
    [],
  );

  const rightPanel = activePermission ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'rules']}
      panels={[
        {
          key: 'detail',
          label: '权限详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="权限编码">{activePermission.code}</Descriptions.Item>
              <Descriptions.Item label="模块">{activePermission.module}</Descriptions.Item>
              <Descriptions.Item label="名称">{activePermission.name}</Descriptions.Item>
              <Descriptions.Item label="描述">{activePermission.description}</Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'rules',
          label: '工作台约束',
          children: (
            <Typography.Text type="secondary">
              权限目录与订单审核共享同一套工业工作台结构，保留左侧筛选、主表、右侧详情和底部明细，只替换为系统权限数据源与只读操作模式。
            </Typography.Text>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="权限详情">
      <Typography.Text type="secondary">请选择一条权限记录查看详情。</Typography.Text>
    </Card>
  );

  const profileRows: DirectoryBottomDetailRow[] = activePermission
    ? [
        { key: 'code', field: '权限编码', value: activePermission.code },
        { key: 'module', field: '模块', value: activePermission.module },
        { key: 'name', field: '名称', value: activePermission.name },
      ]
    : [];

  const descRows: DirectoryBottomDetailRow[] = activePermission
    ? [{ key: 'description', field: '描述', value: activePermission.description }]
    : [];

  const queryRows: DirectoryBottomDetailRow[] = activePermission
    ? [
        { key: 'search', field: '当前搜索关键字', value: trimText(searchKeyword) || '-' },
        { key: 'selected', field: '当前勾选数', value: String(selectedPermissionCodes.length) },
        { key: 'layout', field: '工作台结构', value: '左筛选 / 主表 / 右详情 / 底部明细' },
      ]
    : [];

  const bottomPanel = (
    <Tabs
      size="small"
      type="card"
      activeKey={activeBottomTab}
      onChange={setActiveBottomTab}
      items={[
        {
          key: 'profile',
          label: `基础资料 (${profileRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activePermission),
            profileRows,
            '暂无基础资料',
            `permissions-profile-${activePermission?.code ?? 'empty'}`,
          ),
        },
        {
          key: 'description',
          label: `描述说明 (${descRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activePermission),
            descRows,
            '暂无描述说明',
            `permissions-description-${activePermission?.code ?? 'empty'}`,
          ),
        },
        {
          key: 'query',
          label: `检索上下文 (${queryRows.length})`,
          children: renderDirectoryBottomTable(
            Boolean(activePermission),
            queryRows,
            '暂无检索上下文',
            `permissions-query-${activePermission?.code ?? 'empty'}`,
          ),
        },
      ]}
    />
  );

  return (
    <WorkbenchPageTemplate
      pageTestId="permissions-workbench-page"
      className="order-review-page permissions-directory-page"
      leftPanel={
        <WorkbenchLeftPanelTemplate
          className="permissions-left-panel"
          dataTestId="permissions-left-panel"
          leftTab={leftTab}
          onLeftTabChange={setLeftTab}
          leftRowsForRender={leftRowsForRender}
          onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
            const nextKey = event.data?.key as PermissionLeftFilterKey | undefined;
            if (!nextKey) {
              return;
            }
            resetPagination();
            setLeftFilterKey((current) => (current === nextKey ? 'view:all' : nextKey));
          }}
          leftStatsItems={leftStatsItems}
          leftStatsPieBackground={buildStatsPieBackground(leftStatsItems)}
          globalOrderTotal={sourceRows.length}
          statsScopeText="口径：当前权限目录查询结果统计"
          screenContent={
            <Space direction="vertical" size={10} style={{ width: '100%' }}>
              <div>
                <Typography.Text type="secondary">权限检索</Typography.Text>
                <Input
                  size="small"
                  allowClear
                  value={leftKeyword}
                  placeholder="按编码 / 模块 / 名称缩小左侧结果"
                  onChange={(event) => {
                    resetPagination();
                    setLeftKeyword(event.target.value);
                  }}
                />
              </div>
              <Button
                size="small"
                onClick={() => {
                  resetPagination();
                  setLeftKeyword('');
                  setLeftFilterKey('view:all');
                }}
              >
                重置左侧筛选
              </Button>
            </Space>
          }
        />
      }
      leftPanelVisible={true}
      rightPanel={rightPanel}
      rightPanelVisible={true}
      rightPanelSyncKey={activePermission?.code ?? 'empty'}
      bottomPanel={bottomPanel}
      bottomPanelVisible={true}
      toolbar={
        <WorkbenchToolbar
          leftContent={
            <Space size={8} wrap>
              <Button size="small" onClick={() => void query.refetch()} disabled={!tenantReady}>
                刷新
              </Button>
              <Button
                size="small"
                onClick={() =>
                  message.info('权限目录当前为只读页，变更入口保留在系统授权流程中。')
                }
              >
                变更说明
              </Button>
            </Space>
          }
          searchValue={searchInput}
          onSearchChange={setSearchInput}
          onSearch={() => {
            setSearchKeyword(trimText(searchInput));
            resetPagination();
          }}
          searchPlaceholder="搜索权限编码 / 模块 / 名称 / 描述"
          onExport={() => message.info('权限目录导出能力待接入')}
          onPrint={() => message.info('权限目录打印能力待接入')}
          onHelp={() => message.info('权限目录帮助文档待接入')}
        />
      }
      tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
      tableContent={
        <ErpDataGridNext<PermissionDefinition>
          rowKey={(record) => record.code}
          columns={columns}
          rows={pageRows}
          gridPreset="industrial-review-compact"
          columnSettingsKey="system.permissions.main-grid"
          dataTestId="permissions-directory-grid"
          showColumnSettings
          showSelectionColumn
          selectionCellMode="custom"
          selectionHeaderMode="allInvert"
          loading={query.isFetching}
          error={query.error}
          onRetry={() => {
            void query.refetch();
          }}
          errorTitle="加载权限失败"
          emptyText="暂无权限"
          gridFilterModel={gridFilterModel}
          onGridFilterModelChange={(model) => {
            resetPagination();
            setGridFilters(extractGridFilterModel(model));
          }}
          selectedRowKeys={selectedPermissionCodes}
          activeRowKey={activePermissionCode}
          onSelectionChange={(keys) => {
            setSelectedPermissionCodes(normalizeSelectionKeys(keys));
          }}
          onActiveRowChange={(record) => {
            setActivePermissionCode(record.code);
          }}
        />
      }
      footerSummary={
        <>
          <span>{`总数 ${displayRows.length} 条，本页 ${pageRows.length} 条，已选 ${selectedPermissionCodes.length} 条`}</span>
          <span>{activePermission ? `当前焦点：${activePermission.name}` : '当前焦点：-'}</span>
        </>
      }
      footerSyncKey={`permissions-${displayRows.length}-${pageRows.length}-${selectedPermissionCodes.length}`}
      total={displayRows.length}
      pagination={pagination}
      onPaginationChange={handlePaginationChange}
      showFooterScrollbarRow={true}
    />
  );
};
