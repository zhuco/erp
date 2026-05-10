import {
  Button,
  Card,
  Descriptions,
  Drawer,
  Empty,
  Form,
  Input,
  InputNumber,
  Select,
  Space,
  Table,
  Tabs,
  Tag,
  Typography,
  message,
} from 'antd';
import type { CellClickedEvent, FilterModel } from 'ag-grid-community';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';

import type { ErpDataGridColumn } from '@/components/ErpDataGrid';
import { ErpDataGridNext } from '@/components/ErpDataGridNext';
import {
  WorkbenchLeftPanelTemplate,
  WorkbenchPageTemplate,
  WorkbenchToolbar,
  type WorkbenchLeftRenderRow,
  type WorkbenchLeftStatsItem,
} from '@/components/page-templates';
import { UiModule04BottomTable, UiModule05DetailPanel } from '@/components/ui-modules';

type LogisticsFeeRow = {
  fee_id: string;
  rule_name: string;
  courier_name: string;
  warehouse_name: string;
  region_name: string;
  fee_type: 'WEIGHT' | 'ZONE';
  base_fee: number;
  extra_fee: number;
  status: 'ACTIVE' | 'DISABLED';
  updated_at: string;
};

type LogisticsFeeFormValues = {
  rule_name: string;
  courier_name: string;
  warehouse_name: string;
  region_name: string;
  fee_type: 'WEIGHT' | 'ZONE';
  base_fee: number;
  extra_fee: number;
  status: 'ACTIVE' | 'DISABLED';
};

type LogisticsFeeGridFilters = {
  ruleKeyword: string;
  courierKeyword: string;
  status: 'ALL' | 'ACTIVE' | 'DISABLED';
  feeType: 'ALL' | 'WEIGHT' | 'ZONE';
};

type LogisticsFeeDetailRow = {
  key: string;
  field: string;
  value: string;
};

const seedRows: LogisticsFeeRow[] = [
  {
    fee_id: 'LF-001',
    rule_name: '华东标准件',
    courier_name: '顺丰速运',
    warehouse_name: '上海一仓',
    region_name: '江浙沪',
    fee_type: 'WEIGHT',
    base_fee: 8,
    extra_fee: 1.5,
    status: 'ACTIVE',
    updated_at: '2026-05-08 10:20:00',
  },
  {
    fee_id: 'LF-002',
    rule_name: '华南偏远区',
    courier_name: '京东快递',
    warehouse_name: '广州仓',
    region_name: '海南 / 偏远区',
    fee_type: 'ZONE',
    base_fee: 14,
    extra_fee: 2.8,
    status: 'ACTIVE',
    updated_at: '2026-05-07 16:05:00',
  },
  {
    fee_id: 'LF-003',
    rule_name: '西南大件',
    courier_name: '德邦快递',
    warehouse_name: '成都仓',
    region_name: '云贵川渝',
    fee_type: 'WEIGHT',
    base_fee: 18,
    extra_fee: 4,
    status: 'DISABLED',
    updated_at: '2026-04-29 09:40:00',
  },
];

const createDefaultGridFilters = (): LogisticsFeeGridFilters => ({
  ruleKeyword: '',
  courierKeyword: '',
  status: 'ALL',
  feeType: 'ALL',
});

const trimText = (value?: string | null) => String(value ?? '').trim();
const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');
const formatMoney = (value: number) => `¥${value.toFixed(2)}`;

const buildGridFilterModel = (filters: LogisticsFeeGridFilters): FilterModel => {
  const model: FilterModel = {};
  if (trimText(filters.ruleKeyword)) {
    model.rule_name = { filterType: 'text', type: 'contains', filter: trimText(filters.ruleKeyword) };
  }
  if (trimText(filters.courierKeyword)) {
    model.courier_name = { filterType: 'text', type: 'contains', filter: trimText(filters.courierKeyword) };
  }
  if (filters.status !== 'ALL') {
    model.status = { filterType: 'set', values: [filters.status] };
  }
  if (filters.feeType !== 'ALL') {
    model.fee_type = { filterType: 'set', values: [filters.feeType] };
  }
  return model;
};

const extractGridFilterModel = (model: FilterModel | null | undefined): LogisticsFeeGridFilters => {
  const statusValues =
    model?.status && typeof model.status === 'object' && 'values' in model.status && Array.isArray(model.status.values)
      ? model.status.values.map((value) => String(value ?? '').trim()).filter(Boolean)
      : [];
  const feeTypeValues =
    model?.fee_type &&
    typeof model.fee_type === 'object' &&
    'values' in model.fee_type &&
    Array.isArray(model.fee_type.values)
      ? model.fee_type.values.map((value) => String(value ?? '').trim()).filter(Boolean)
      : [];
  return {
    ruleKeyword:
      model?.rule_name && typeof model.rule_name === 'object' && 'filter' in model.rule_name
        ? String(model.rule_name.filter ?? '').trim()
        : '',
    courierKeyword:
      model?.courier_name && typeof model.courier_name === 'object' && 'filter' in model.courier_name
        ? String(model.courier_name.filter ?? '').trim()
        : '',
    status: (statusValues[0] as LogisticsFeeGridFilters['status'] | undefined) ?? 'ALL',
    feeType: (feeTypeValues[0] as LogisticsFeeGridFilters['feeType'] | undefined) ?? 'ALL',
  };
};

export const LogisticsFeeDirectoryPage = () => {
  const [rows, setRows] = useState<LogisticsFeeRow[]>(seedRows);
  const [searchInput, setSearchInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftTab, setLeftTab] = useState<'filter' | 'screen'>('filter');
  const [leftKeyword, setLeftKeyword] = useState('');
  const [leftFilterId, setLeftFilterId] = useState<'view:all' | 'status:ACTIVE' | 'status:DISABLED' | 'type:WEIGHT' | 'type:ZONE'>('view:all');
  const [gridFilters, setGridFilters] = useState<LogisticsFeeGridFilters>(createDefaultGridFilters);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingRow, setEditingRow] = useState<LogisticsFeeRow | null>(null);
  const [activeRowId, setActiveRowId] = useState<string | null>(seedRows[0]?.fee_id ?? null);
  const [selectedRowIds, setSelectedRowIds] = useState<string[]>([]);
  const [activeBottomTab, setActiveBottomTab] = useState('profile');
  const [form] = Form.useForm<LogisticsFeeFormValues>();

  const gridFilterModel = useMemo(() => buildGridFilterModel(gridFilters), [gridFilters]);

  const searchScopedRows = useMemo(() => {
    const keyword = trimText(searchKeyword).toLowerCase();
    if (!keyword) {
      return rows;
    }
    return rows.filter((row) =>
      [row.rule_name, row.courier_name, row.warehouse_name, row.region_name]
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword)),
    );
  }, [rows, searchKeyword]);

  const leftFilteredRows = useMemo(() => {
    const keyword = trimText(leftKeyword).toLowerCase();
    return searchScopedRows.filter((row) => {
      if (leftFilterId === 'status:ACTIVE' && row.status !== 'ACTIVE') return false;
      if (leftFilterId === 'status:DISABLED' && row.status !== 'DISABLED') return false;
      if (leftFilterId === 'type:WEIGHT' && row.fee_type !== 'WEIGHT') return false;
      if (leftFilterId === 'type:ZONE' && row.fee_type !== 'ZONE') return false;
      if (!keyword) return true;
      return [row.rule_name, row.courier_name, row.region_name]
        .map((value) => value.toLowerCase())
        .some((value) => value.includes(keyword));
    });
  }, [leftFilterId, leftKeyword, searchScopedRows]);

  const displayRows = useMemo(
    () =>
      leftFilteredRows.filter((row) => {
        if (gridFilters.status !== 'ALL' && row.status !== gridFilters.status) return false;
        if (gridFilters.feeType !== 'ALL' && row.fee_type !== gridFilters.feeType) return false;
        if (trimText(gridFilters.ruleKeyword) && !row.rule_name.toLowerCase().includes(trimText(gridFilters.ruleKeyword).toLowerCase())) {
          return false;
        }
        if (trimText(gridFilters.courierKeyword) && !row.courier_name.toLowerCase().includes(trimText(gridFilters.courierKeyword).toLowerCase())) {
          return false;
        }
        return true;
      }),
    [gridFilters, leftFilteredRows],
  );

  const activeRow = useMemo(
    () => displayRows.find((row) => row.fee_id === activeRowId) ?? displayRows[0] ?? null,
    [activeRowId, displayRows],
  );

  const leftStatsItems = useMemo<WorkbenchLeftStatsItem[]>(
    () => [
      { key: 'total', label: '全部规则', value: searchScopedRows.length, color: '#2f6fed' },
      { key: 'active', label: '启用', value: searchScopedRows.filter((row) => row.status === 'ACTIVE').length, color: '#0f9d58' },
      { key: 'zone', label: '区域计费', value: searchScopedRows.filter((row) => row.fee_type === 'ZONE').length, color: '#d48806' },
    ],
    [searchScopedRows],
  );

  const leftRowsForRender = useMemo<WorkbenchLeftRenderRow[]>(
    () => [
      { key: 'view:all', label: '全部规则', count: searchScopedRows.length, nodeType: 'logistics', level: 0, hasChildren: false, expanded: false, checked: leftFilterId === 'view:all' },
      { key: 'status:ACTIVE', label: '启用中', count: searchScopedRows.filter((row) => row.status === 'ACTIVE').length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterId === 'status:ACTIVE' },
      { key: 'status:DISABLED', label: '已停用', count: searchScopedRows.filter((row) => row.status === 'DISABLED').length, nodeType: 'status', level: 0, hasChildren: false, expanded: false, checked: leftFilterId === 'status:DISABLED' },
      { key: 'type:WEIGHT', label: '按重量计费', count: searchScopedRows.filter((row) => row.fee_type === 'WEIGHT').length, nodeType: 'tag', level: 0, hasChildren: false, expanded: false, checked: leftFilterId === 'type:WEIGHT' },
      { key: 'type:ZONE', label: '按区域计费', count: searchScopedRows.filter((row) => row.fee_type === 'ZONE').length, nodeType: 'tag', level: 0, hasChildren: false, expanded: false, checked: leftFilterId === 'type:ZONE' },
    ],
    [leftFilterId, searchScopedRows],
  );

  const leftPanel = (
    <WorkbenchLeftPanelTemplate
      className="logistics-fee-left-panel"
      dataTestId="logistics-fee-left-panel"
      leftTab={leftTab}
      onLeftTabChange={setLeftTab}
      leftRowsForRender={leftRowsForRender}
      onLeftRowClick={(event: CellClickedEvent<WorkbenchLeftRenderRow>) => {
        const key = event.data?.key;
        if (!key) return;
        setLeftFilterId((current) => (current === key ? 'view:all' : (key as typeof leftFilterId)));
      }}
      leftStatsItems={leftStatsItems}
      leftStatsPieBackground="conic-gradient(#2f6fed 0deg 180deg, #0f9d58 180deg 280deg, #d48806 280deg 360deg)"
      globalOrderTotal={searchScopedRows.length}
      statsScopeText="口径：当前目录页规则样本"
      screenContent={
        <Space direction="vertical" size={10} style={{ width: '100%' }}>
          <div>
            <Typography.Text type="secondary">规则检索</Typography.Text>
            <Input size="small" allowClear value={leftKeyword} placeholder="筛选规则 / 快递 / 区域" onChange={(event) => setLeftKeyword(event.target.value)} />
          </div>
          <Button size="small" onClick={() => { setLeftKeyword(''); setLeftFilterId('view:all'); }}>
            重置左侧筛选
          </Button>
        </Space>
      }
    />
  );

  const columns = useMemo<ErpDataGridColumn<LogisticsFeeRow>[]>(
    () => [
      { key: 'rule_name', field: 'rule_name', title: '规则名称', width: 180, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'courier_name', field: 'courier_name', title: '物流公司', width: 140, filter: 'agTextColumnFilter', filterParams: { filterOptions: ['contains'], defaultOption: 'contains', maxNumConditions: 1 } },
      { key: 'warehouse_name', field: 'warehouse_name', title: '仓库', width: 120 },
      { key: 'fee_type', field: 'fee_type', title: '计费类型', width: 110, filter: 'agSetColumnFilter', filterParams: { values: ['WEIGHT', 'ZONE'] }, renderCell: (value) => <Tag color={value === 'ZONE' ? 'blue' : 'gold'}>{value === 'ZONE' ? '区域' : '重量'}</Tag> },
      { key: 'base_fee', field: 'base_fee', title: '首费', width: 100, renderCell: (value) => formatMoney(Number(value ?? 0)) },
      { key: 'extra_fee', field: 'extra_fee', title: '续费', width: 100, renderCell: (value) => formatMoney(Number(value ?? 0)) },
      { key: 'status', field: 'status', title: '状态', width: 100, filter: 'agSetColumnFilter', filterParams: { values: ['ACTIVE', 'DISABLED'] }, renderCell: (value) => <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>{value === 'ACTIVE' ? '启用' : '停用'}</Tag> },
      { key: 'updated_at', field: 'updated_at', title: '更新时间', width: 160, renderCell: (value) => formatDateTime(String(value ?? '')) },
    ],
    [],
  );

  const rightPanel = activeRow ? (
    <UiModule05DetailPanel
      defaultActiveKeys={['detail', 'price']}
      panels={[
        {
          key: 'detail',
          label: '规则详情',
          children: (
            <Descriptions column={1} size="small">
              <Descriptions.Item label="规则名称">{activeRow.rule_name}</Descriptions.Item>
              <Descriptions.Item label="物流公司">{activeRow.courier_name}</Descriptions.Item>
              <Descriptions.Item label="仓库">{activeRow.warehouse_name}</Descriptions.Item>
              <Descriptions.Item label="区域">{activeRow.region_name}</Descriptions.Item>
              <Descriptions.Item label="状态">{activeRow.status === 'ACTIVE' ? '启用' : '停用'}</Descriptions.Item>
            </Descriptions>
          ),
        },
        {
          key: 'price',
          label: '计费结构',
          children: (
            <Space direction="vertical" size={8} style={{ width: '100%' }}>
              <span>{`首费：${formatMoney(activeRow.base_fee)}`}</span>
              <span>{`续费：${formatMoney(activeRow.extra_fee)}`}</span>
              <span>{`类型：${activeRow.fee_type === 'ZONE' ? '区域计费' : '重量计费'}`}</span>
            </Space>
          ),
        },
      ]}
    />
  ) : (
    <Card size="small" title="规则详情">
      <Typography.Text type="secondary">请选择一条物流费规则查看详情。</Typography.Text>
    </Card>
  );

  const bottomColumns = [
    { title: '字段', dataIndex: 'field', key: 'field', width: 180 },
    { title: '值', dataIndex: 'value', key: 'value', width: 320 },
  ];

  const profileRows: LogisticsFeeDetailRow[] = activeRow
    ? [
        { key: 'rule', field: '规则名称', value: activeRow.rule_name },
        { key: 'courier', field: '物流公司', value: activeRow.courier_name },
        { key: 'warehouse', field: '仓库', value: activeRow.warehouse_name },
        { key: 'region', field: '区域范围', value: activeRow.region_name },
      ]
    : [];

  const pricingRows: LogisticsFeeDetailRow[] = activeRow
    ? [
        { key: 'type', field: '计费类型', value: activeRow.fee_type === 'ZONE' ? '区域计费' : '重量计费' },
        { key: 'base', field: '首费', value: formatMoney(activeRow.base_fee) },
        { key: 'extra', field: '续费', value: formatMoney(activeRow.extra_fee) },
        { key: 'status', field: '状态', value: activeRow.status === 'ACTIVE' ? '启用' : '停用' },
      ]
    : [];

  const timelineRows: LogisticsFeeDetailRow[] = activeRow
    ? [
        { key: 'updated', field: '更新时间', value: formatDateTime(activeRow.updated_at) },
        { key: 'search', field: '当前检索关键字', value: trimText(searchKeyword) || '-' },
      ]
    : [];

  const renderBottomTable = (dataSource: LogisticsFeeDetailRow[], emptyDescription: string, syncKey: string) =>
    activeRow ? (
      <UiModule04BottomTable syncKey={syncKey}>
        <Table<LogisticsFeeDetailRow>
          size="small"
          rowKey="key"
          columns={bottomColumns}
          dataSource={dataSource}
          pagination={false}
          scroll={{ x: 'max-content' }}
          locale={{ emptyText: <Empty description={emptyDescription} /> }}
        />
      </UiModule04BottomTable>
    ) : (
      <Empty description="请选择一条物流费规则查看底部明细" />
    );

  const bottomPanel = (
    <Tabs
      size="small"
      type="card"
      activeKey={activeBottomTab}
      onChange={setActiveBottomTab}
      items={[
        { key: 'profile', label: `基础资料 (${profileRows.length})`, children: renderBottomTable(profileRows, '暂无基础资料', `logistics-fee-profile-${activeRow?.fee_id ?? 'empty'}`) },
        { key: 'pricing', label: `计费结构 (${pricingRows.length})`, children: renderBottomTable(pricingRows, '暂无计费结构', `logistics-fee-pricing-${activeRow?.fee_id ?? 'empty'}`) },
        { key: 'timeline', label: `维护轨迹 (${timelineRows.length})`, children: renderBottomTable(timelineRows, '暂无维护轨迹', `logistics-fee-timeline-${activeRow?.fee_id ?? 'empty'}`) },
      ]}
    />
  );

  const openCreateDrawer = () => {
    setEditingRow(null);
    form.resetFields();
    form.setFieldsValue({ fee_type: 'WEIGHT', base_fee: 0, extra_fee: 0, status: 'ACTIVE' });
    setDrawerOpen(true);
  };

  const openEditDrawer = (row: LogisticsFeeRow) => {
    setEditingRow(row);
    form.setFieldsValue(row);
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    const values = await form.validateFields();
    if (editingRow) {
      setRows((current) => current.map((row) => (row.fee_id === editingRow.fee_id ? { ...row, ...values, updated_at: dayjs().format('YYYY-MM-DD HH:mm:ss') } : row)));
      message.success('物流费规则已更新');
    } else {
      const nextId = `LF-${String(rows.length + 1).padStart(3, '0')}`;
      const nextRow: LogisticsFeeRow = { fee_id: nextId, ...values, updated_at: dayjs().format('YYYY-MM-DD HH:mm:ss') };
      setRows((current) => [nextRow, ...current]);
      setActiveRowId(nextId);
      message.success('物流费规则已创建');
    }
    setDrawerOpen(false);
    setEditingRow(null);
  };

  const handleToggleStatus = (status: 'ACTIVE' | 'DISABLED') => {
    const targetIds = selectedRowIds.length > 0 ? selectedRowIds : activeRow ? [activeRow.fee_id] : [];
    if (!targetIds.length) {
      message.warning(status === 'ACTIVE' ? '请选择要启用的规则' : '请选择要停用的规则');
      return;
    }
    setRows((current) =>
      current.map((row) =>
        targetIds.includes(row.fee_id) ? { ...row, status, updated_at: dayjs().format('YYYY-MM-DD HH:mm:ss') } : row,
      ),
    );
    message.success(status === 'ACTIVE' ? '物流费规则已启用' : '物流费规则已停用');
  };

  const toolbarLeftContent = (
    <Space size={8} wrap>
      <Button type="primary" size="small" onClick={openCreateDrawer}>
        新建规则
      </Button>
      <Button size="small" disabled={!activeRow} onClick={() => activeRow && openEditDrawer(activeRow)}>
        编辑
      </Button>
      <Button size="small" disabled={!activeRow && !selectedRowIds.length} onClick={() => handleToggleStatus('ACTIVE')}>
        启用
      </Button>
      <Button size="small" disabled={!activeRow && !selectedRowIds.length} onClick={() => handleToggleStatus('DISABLED')}>
        停用
      </Button>
    </Space>
  );

  return (
    <>
      <WorkbenchPageTemplate
        pageTestId="logistics-fee-directory-workbench-page"
        className="order-review-page logistics-fee-directory-page"
        leftPanel={leftPanel}
        leftPanelVisible={true}
        rightPanel={rightPanel}
        rightPanelVisible={true}
        rightPanelSyncKey={activeRow?.fee_id ?? 'empty'}
        bottomPanel={bottomPanel}
        bottomPanelVisible={true}
        toolbar={
          <WorkbenchToolbar
            leftContent={toolbarLeftContent}
            searchValue={searchInput}
            onSearchChange={setSearchInput}
            onSearch={() => setSearchKeyword(searchInput.trim())}
            searchPlaceholder="搜索规则名称 / 物流公司 / 区域"
            onExport={() => message.info('物流费目录导出能力待接入')}
            onPrint={() => message.info('物流费目录打印能力待接入')}
            onHelp={() => message.info('物流费目录帮助文档待接入')}
          />
        }
        tableClassName="order-review-table order-workbench-table-module-03 order-review-table-external-scrollbar"
        tableContent={
          <ErpDataGridNext<LogisticsFeeRow>
            rowKey={(record) => record.fee_id}
            columns={columns}
            rows={displayRows}
            gridPreset="industrial-review-compact"
            columnSettingsKey="logistics.fee.directory.main-grid"
            dataTestId="logistics-fee-directory-grid"
            showColumnSettings
            showSelectionColumn
            selectionCellMode="custom"
            selectionHeaderMode="allInvert"
            gridFilterModel={gridFilterModel}
            onGridFilterModelChange={(model) => {
              setGridFilters(extractGridFilterModel(model));
            }}
            selectedRowKeys={selectedRowIds}
            activeRowKey={activeRow?.fee_id ?? null}
            onSelectionChange={(keys) => {
              setSelectedRowIds(Array.from(new Set(keys.map((key) => String(key ?? '').trim()).filter(Boolean))));
            }}
            onActiveRowChange={(record) => {
              setActiveRowId(record.fee_id);
            }}
            onRowDoubleClick={(record) => openEditDrawer(record)}
            emptyText="暂无物流费规则"
          />
        }
        footerSummary={
          <>
            <span>{`总数 ${displayRows.length} 条，已选 ${selectedRowIds.length} 条`}</span>
            <span>{activeRow ? `当前焦点：${activeRow.rule_name}` : '当前焦点：-'}</span>
          </>
        }
      />
      <Drawer
        title={editingRow ? '编辑物流费规则' : '新建物流费规则'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={560}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" onClick={() => void handleSave()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={form} layout="vertical">
          <Form.Item name="rule_name" label="规则名称" rules={[{ required: true, message: '请输入规则名称' }]}>
            <Input placeholder="例如：华东标准件" />
          </Form.Item>
          <Form.Item name="courier_name" label="物流公司" rules={[{ required: true, message: '请输入物流公司' }]}>
            <Input placeholder="例如：顺丰速运" />
          </Form.Item>
          <Form.Item name="warehouse_name" label="仓库" rules={[{ required: true, message: '请输入仓库' }]}>
            <Input placeholder="例如：上海一仓" />
          </Form.Item>
          <Form.Item name="region_name" label="区域" rules={[{ required: true, message: '请输入区域' }]}>
            <Input placeholder="例如：江浙沪" />
          </Form.Item>
          <Form.Item name="fee_type" label="计费类型">
            <Select options={[{ label: '按重量', value: 'WEIGHT' }, { label: '按区域', value: 'ZONE' }]} />
          </Form.Item>
          <Form.Item name="base_fee" label="首费">
            <InputNumber min={0} precision={2} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="extra_fee" label="续费">
            <InputNumber min={0} precision={2} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={[{ label: '启用', value: 'ACTIVE' }, { label: '停用', value: 'DISABLED' }]} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
