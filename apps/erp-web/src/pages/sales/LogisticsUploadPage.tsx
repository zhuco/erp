import {
  Badge,
  Button,
  Empty,
  Input,
  Select,
  Space,
  Table,
  Tabs,
  Tag,
  Tree,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { DataNode } from 'antd/es/tree';
import type { Key } from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { BellOutlined, DownloadOutlined, QuestionCircleOutlined } from '@ant-design/icons';

import type { ServerTableColumns } from '@/components/ServerTable';
import {
  UiModule03MainTableSection,
  UiModule03TableFooter,
  UiModule04BottomTable
} from '@/components/ui-modules';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';

import './logisticsUploadPage.css';

type UploadStatus =
  | '未上传'
  | '部分上传'
  | '上传成功'
  | '上传失败'
  | '停止上传'
  | '取消上传';

type LogisticsUploadLog = {
  id: string;
  time: string;
  operator: string;
  content: string;
};

type LogisticsUploadGoods = {
  id: string;
  sku: string;
  name: string;
  quantity: number;
};

type LogisticsUploadRow = {
  id: string;
  channel: string;
  shopOrderNo: string;
  orderTag?: string | null;
  logMessage?: string | null;
  uploadStatus: UploadStatus;
  carrier?: string | null;
  trackingNo?: string | null;
  logs: LogisticsUploadLog[];
  goods: LogisticsUploadGoods[];
};

const statusColors: Record<UploadStatus, string> = {
  未上传: 'red',
  部分上传: 'orange',
  上传成功: 'green',
  上传失败: 'red',
  停止上传: 'default',
  取消上传: 'default'
};

const filterTreeData: DataNode[] = [
  { key: 'custom', title: '自定义过滤' },
  {
    key: 'upload-status',
    title: '物流上传状态',
    children: [
      { key: 'status-pending', title: '未上传' },
      { key: 'status-success', title: '上传成功' },
      { key: 'status-partial', title: '部分上传' },
      { key: 'status-failed', title: '上传失败' },
      { key: 'status-stopped', title: '停止上传' },
      { key: 'status-cancelled', title: '取消上传' }
    ]
  },
  {
    key: 'ship-time',
    title: '距离待发货时间',
    children: [
      { key: 'ship-overdue', title: '已超时' },
      { key: 'ship-6h', title: '6小时内' },
      { key: 'ship-12h', title: '12小时内' },
      { key: 'ship-24h', title: '24小时内' }
    ]
  },
  {
    key: 'channel',
    title: '销售渠道',
    children: [
      { key: 'channel-douyin', title: '浩太-抖音' },
      { key: 'channel-pdd', title: '浩太-拼多多' },
      { key: 'channel-taobao', title: '浩太-淘宝' }
    ]
  },
  {
    key: 'refund-status',
    title: '退款状态',
    children: [
      { key: 'refund-none', title: '无退款' },
      { key: 'refund-partial', title: '部分退款' },
      { key: 'refund-all', title: '全额退款' }
    ]
  }
];

const treeStatusMap: Record<string, UploadStatus | undefined> = {
  'status-pending': '未上传',
  'status-success': '上传成功',
  'status-partial': '部分上传',
  'status-failed': '上传失败',
  'status-stopped': '停止上传',
  'status-cancelled': '取消上传'
};

const treeChannelMap: Record<string, string | undefined> = {
  'channel-douyin': '浩太-抖音',
  'channel-pdd': '浩太-拼多多',
  'channel-taobao': '浩太-淘宝'
};

const channelOptions = [
  { label: '浩太-抖音', value: '浩太-抖音' },
  { label: '浩太-拼多多', value: '浩太-拼多多' },
  { label: '浩太-淘宝', value: '浩太-淘宝' }
];

const orderTagOptions = [
  { label: '加急', value: '加急' },
  { label: '合并', value: '合并' },
  { label: '补发', value: '补发' }
];

const uploadStatusOptions = [
  { label: '未上传', value: '未上传' },
  { label: '部分上传', value: '部分上传' },
  { label: '上传成功', value: '上传成功' },
  { label: '上传失败', value: '上传失败' },
  { label: '停止上传', value: '停止上传' },
  { label: '取消上传', value: '取消上传' }
];

const carrierOptions = [
  { label: '顺丰', value: '顺丰' },
  { label: '圆通', value: '圆通' },
  { label: '申通', value: '申通' }
];

const baseRows: LogisticsUploadRow[] = [
  {
    id: '1',
    channel: '浩太-抖音',
    shopOrderNo: '6950018791763941234',
    orderTag: null,
    logMessage: null,
    uploadStatus: '未上传',
    carrier: null,
    trackingNo: null,
    logs: [
      {
        id: '1-1',
        time: '2026-01-19 06:01:59',
        operator: '系统自动',
        content: '[消息] 网店订单更新'
      },
      {
        id: '1-2',
        time: '2026-01-18 19:02:42',
        operator: '系统自动',
        content: '[消息] 网店订单更新'
      },
      {
        id: '1-3',
        time: '2026-01-18 01:33:13',
        operator: '系统自动',
        content: '网店订单导入成功（自动）'
      },
      {
        id: '1-4',
        time: '2026-01-18 01:33:08',
        operator: '系统自动',
        content: '[推单] 网店订单创建，交易状态：等待卖家发货'
      }
    ],
    goods: [
      { id: '1-1', sku: 'SKU-1001', name: '轻量托盘', quantity: 1 },
      { id: '1-2', sku: 'SKU-1002', name: '泡棉箱', quantity: 2 }
    ]
  },
  {
    id: '2',
    channel: '浩太-拼多多',
    shopOrderNo: '260113-054106529291808',
    orderTag: null,
    logMessage: null,
    uploadStatus: '未上传',
    carrier: null,
    trackingNo: null,
    logs: [
      {
        id: '2-1',
        time: '2026-01-19 02:24:40',
        operator: '系统自动',
        content: '网店订单导入成功（自动）'
      },
      {
        id: '2-2',
        time: '2026-01-19 01:58:11',
        operator: '系统自动',
        content: '[消息] 网店订单更新'
      }
    ],
    goods: [
      { id: '2-1', sku: 'SKU-2001', name: '加厚包装袋', quantity: 1 },
      { id: '2-2', sku: 'SKU-2002', name: '安全封条', quantity: 4 }
    ]
  },
  {
    id: '3',
    channel: '浩太-抖音',
    shopOrderNo: '6949680124265422701',
    orderTag: '加急',
    logMessage: '物流信息部分上传，待补齐物流单号',
    uploadStatus: '部分上传',
    carrier: '顺丰',
    trackingNo: 'SF103829384',
    logs: [
      {
        id: '3-1',
        time: '2026-01-19 08:45:12',
        operator: '物流专员',
        content: '上传顺丰运单号，等待回传确认'
      }
    ],
    goods: [
      { id: '3-1', sku: 'SKU-3001', name: '手持风扇', quantity: 1 },
      { id: '3-2', sku: 'SKU-3002', name: '便携充电线', quantity: 2 }
    ]
  },
  {
    id: '4',
    channel: '浩太-抖音',
    shopOrderNo: '6923433504665337793',
    orderTag: '合并',
    logMessage: '物流信息已回传平台',
    uploadStatus: '上传成功',
    carrier: '圆通',
    trackingNo: 'YT239845190',
    logs: [
      {
        id: '4-1',
        time: '2026-01-19 09:12:08',
        operator: '物流专员',
        content: '物流信息上传成功，平台已确认'
      }
    ],
    goods: [
      { id: '4-1', sku: 'SKU-4001', name: '折叠收纳箱', quantity: 1 }
    ]
  }
];

export const LogisticsUploadPage = () => {
  const { setLeftPanel, setLeftPanelVisible, setRightPanelVisible, clearPanels } =
    useWorkbenchPanels();
  const [dateRange, setDateRange] = useState('last-month');
  const [uploadScene, setUploadScene] = useState('logistics');
  const [searchValue, setSearchValue] = useState('');
  const [treeSelection, setTreeSelection] = useState<Key[]>(['status-pending']);
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [activeRowId, setActiveRowId] = useState<string | null>(null);
  const [activeBottomTab, setActiveBottomTab] = useState('logs');
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 1000,
    pageSizeOptions: [50, 100, 200, 500, 1000]
  });
  const [columnFilters, setColumnFilters] = useState({
    channel: undefined as string | undefined,
    orderNo: '',
    orderTag: undefined as string | undefined,
    uploadStatus: undefined as UploadStatus | undefined,
    carrier: undefined as string | undefined,
    trackingNo: ''
  });
  const tableContainerRef = useRef<HTMLDivElement>(null);

  const handleTreeSelect = useCallback((keys: Key[]) => {
    const nextKey = keys[0] ? String(keys[0]) : 'status-pending';
    setTreeSelection([nextKey]);
  }, []);

  const filterPanel = useMemo(
    () => (
      <div className="logistics-upload-filter-panel">
        <div className="logistics-upload-filter-header">
          <Typography.Text strong>过滤</Typography.Text>
          <Button size="small" type="text">
            筛选
          </Button>
        </div>
        <Tree
          blockNode
          defaultExpandAll
          selectedKeys={treeSelection}
          onSelect={handleTreeSelect}
          treeData={filterTreeData}
        />
      </div>
    ),
    [handleTreeSelect, treeSelection]
  );

  useEffect(() => {
    setLeftPanelVisible(true);
    setRightPanelVisible(false);
    setLeftPanel(filterPanel);
    return () => clearPanels();
  }, [
    clearPanels,
    filterPanel,
    setLeftPanel,
    setLeftPanelVisible,
    setRightPanelVisible
  ]);

  const activeTreeKey = useMemo(
    () => String(treeSelection[0] ?? 'status-pending'),
    [treeSelection]
  );
  const treeStatusFilter = useMemo(
    () => treeStatusMap[activeTreeKey],
    [activeTreeKey]
  );
  const treeChannelFilter = useMemo(
    () => treeChannelMap[activeTreeKey],
    [activeTreeKey]
  );

  const { channel, orderNo, orderTag, uploadStatus, carrier, trackingNo } = columnFilters;

  const filteredRows = useMemo(() => {
    const keyword = searchValue.trim().toLowerCase();
    return baseRows.filter((row) => {
      if (treeStatusFilter && row.uploadStatus !== treeStatusFilter) {
        return false;
      }
      if (treeChannelFilter && row.channel !== treeChannelFilter) {
        return false;
      }
      if (channel && row.channel !== channel) {
        return false;
      }
      if (uploadStatus && row.uploadStatus !== uploadStatus) {
        return false;
      }
      if (orderTag && row.orderTag !== orderTag) {
        return false;
      }
      if (carrier && row.carrier !== carrier) {
        return false;
      }
      if (orderNo && !row.shopOrderNo.includes(orderNo)) {
        return false;
      }
      if (trackingNo && !String(row.trackingNo ?? '').includes(trackingNo)) {
        return false;
      }
      if (keyword) {
        const content = [
          row.channel,
          row.shopOrderNo,
          row.carrier ?? '',
          row.trackingNo ?? '',
          row.logMessage ?? ''
        ]
          .join(' ')
          .toLowerCase();
        if (!content.includes(keyword)) {
          return false;
        }
      }
      return true;
    });
  }, [
    carrier,
    channel,
    orderNo,
    orderTag,
    searchValue,
    trackingNo,
    treeChannelFilter,
    treeStatusFilter,
    uploadStatus
  ]);

  useEffect(() => {
    setPagination((prev) => ({ ...prev, current: 1 }));
  }, [
    carrier,
    channel,
    orderNo,
    orderTag,
    searchValue,
    trackingNo,
    treeChannelFilter,
    treeStatusFilter,
    uploadStatus
  ]);

  useEffect(() => {
    if (!filteredRows.length) {
      setActiveRowId(null);
      return;
    }
    if (!activeRowId || !filteredRows.some((row) => row.id === activeRowId)) {
      setActiveRowId(filteredRows[0].id);
    }
  }, [activeRowId, filteredRows]);

  const pagedRows = useMemo(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    const start = (current - 1) * pageSize;
    return filteredRows.slice(start, start + pageSize);
  }, [filteredRows, pagination.current, pagination.pageSize]);

  const updateColumnFilters = useCallback(
    (next: Partial<typeof columnFilters>) =>
      setColumnFilters((prev) => ({
        ...prev,
        ...next
      })),
    []
  );

  const columns = useMemo<ServerTableColumns<LogisticsUploadRow>>(() => {
    const current = pagination.current ?? 1;
    const pageSize = pagination.pageSize ?? 20;
    return [
      {
        title: '序号',
        key: 'index',
        width: 60,
        render: (_: LogisticsUploadRow, __: LogisticsUploadRow, index: number) =>
          (current - 1) * pageSize + index + 1
      },
      {
        title: '销售渠道',
        dataIndex: 'channel',
        queryCell: (
          <Select
            allowClear
            size="small"
            placeholder="销售渠道"
            options={channelOptions}
            value={channel}
            onChange={(value) => updateColumnFilters({ channel: value ?? undefined })}
          />
        )
      },
      {
        title: '网店订单号',
        dataIndex: 'shopOrderNo',
        queryCell: (
          <Input
            allowClear
            size="small"
            placeholder="订单号"
            value={orderNo}
            onChange={(event) => updateColumnFilters({ orderNo: event.target.value })}
          />
        )
      },
      {
        title: '订单标记',
        dataIndex: 'orderTag',
        render: (value: string | null | undefined) => value ?? '-',
        queryCell: (
          <Select
            allowClear
            size="small"
            placeholder="订单标记"
            options={orderTagOptions}
            value={orderTag}
            onChange={(value) => updateColumnFilters({ orderTag: value ?? undefined })}
          />
        )
      },
      {
        title: '日志描述',
        dataIndex: 'logMessage',
        render: (value: string | null | undefined) => value || '-'
      },
      {
        title: '物流上传状态',
        dataIndex: 'uploadStatus',
        render: (value: UploadStatus) => (
          <Tag color={statusColors[value]}>{value}</Tag>
        ),
        queryCell: (
          <Select
            allowClear
            size="small"
            placeholder="上传状态"
            options={uploadStatusOptions}
            value={uploadStatus}
            onChange={(value) => updateColumnFilters({ uploadStatus: value ?? undefined })}
          />
        )
      },
      {
        title: '物流公司',
        dataIndex: 'carrier',
        render: (value: string | null | undefined) => value ?? '-',
        queryCell: (
          <Select
            allowClear
            size="small"
            placeholder="物流公司"
            options={carrierOptions}
            value={carrier}
            onChange={(value) => updateColumnFilters({ carrier: value ?? undefined })}
          />
        )
      },
      {
        title: '物流单号',
        dataIndex: 'trackingNo',
        render: (value: string | null | undefined) => value ?? '-',
        queryCell: (
          <Input
            allowClear
            size="small"
            placeholder="物流单号"
            value={trackingNo}
            onChange={(event) => updateColumnFilters({ trackingNo: event.target.value })}
          />
        )
      }
    ];
  }, [
    carrier,
    channel,
    orderNo,
    orderTag,
    pagination.current,
    pagination.pageSize,
    trackingNo,
    updateColumnFilters,
    uploadStatus
  ]);

  const failedCount = useMemo(
    () => baseRows.filter((row) => row.uploadStatus === '上传失败').length,
    []
  );

  const activeRow = useMemo(
    () => filteredRows.find((row) => row.id === activeRowId) ?? filteredRows[0],
    [activeRowId, filteredRows]
  );

  const logColumns = useMemo<ColumnsType<LogisticsUploadLog>>(
    () => [
      { title: '时间', dataIndex: 'time', width: 160 },
      { title: '操作员', dataIndex: 'operator', width: 120 },
      { title: '内容', dataIndex: 'content' }
    ],
    []
  );

  const goodsColumns = useMemo<ColumnsType<LogisticsUploadGoods>>(
    () => [
      { title: 'SKU', dataIndex: 'sku', width: 120 },
      { title: '货品名称', dataIndex: 'name' },
      { title: '数量', dataIndex: 'quantity', width: 80 }
    ],
    []
  );

  const logTable = useMemo(
    () => (
      <UiModule04BottomTable
        syncKey={`${activeRowId ?? 'none'}-${activeBottomTab}-${
          activeRow?.logs.length ?? 0
        }`}
      >
        <Table
          size="small"
          rowKey="id"
          columns={logColumns}
          dataSource={activeRow?.logs ?? []}
          pagination={false}
          scroll={{ x: 'max-content' }}
          locale={{ emptyText: <Empty description="暂无日志" /> }}
        />
      </UiModule04BottomTable>
    ),
    [activeBottomTab, activeRow?.logs, activeRowId, logColumns]
  );

  const goodsTable = useMemo(
    () => (
      <UiModule04BottomTable
        syncKey={`${activeRowId ?? 'none'}-goods-${activeRow?.goods.length ?? 0}`}
      >
        <Table
          size="small"
          rowKey="id"
          columns={goodsColumns}
          dataSource={activeRow?.goods ?? []}
          pagination={false}
          scroll={{ x: 'max-content' }}
          locale={{ emptyText: <Empty description="暂无货品" /> }}
        />
      </UiModule04BottomTable>
    ),
    [activeRow?.goods, activeRowId, goodsColumns]
  );

  const bottomTabs = useMemo(
    () => (
      <Tabs
        size="small"
        type="card"
        activeKey={activeBottomTab}
        onChange={setActiveBottomTab}
        items={[
          { key: 'logs', label: '日志', children: logTable },
          { key: 'goods', label: '货品', children: goodsTable }
        ]}
      />
    ),
    [activeBottomTab, goodsTable, logTable]
  );

  const toolbarLeft = (
    <Space size={12} align="center" className="logistics-upload-toolbar-left">
      <Select
        size="small"
        value={dateRange}
        onChange={setDateRange}
        options={[
          { label: '近一个月', value: 'last-month' },
          { label: '近三个月', value: 'last-quarter' },
          { label: '近半年', value: 'last-half-year' }
        ]}
      />
      <Select
        size="small"
        value={uploadScene}
        onChange={setUploadScene}
        options={[
          { label: '物流信息上传', value: 'logistics' },
          { label: '平台回传', value: 'platform' }
        ]}
      />
      <Space size={6} align="center">
        <Badge count={failedCount} showZero color="#ff4d4f" />
        <Typography.Text type="secondary">个失败</Typography.Text>
        <Button type="link" size="small" onClick={() => message.info('已完成清理')}>
          一键清理
        </Button>
      </Space>
    </Space>
  );

  const toolbarRight = (
    <Space size={8} align="center" className="logistics-upload-toolbar-right" wrap>
      <Input.Search
        allowClear
        placeholder="网店订单号/客户账号/交易名称/物流"
        value={searchValue}
        onChange={(event) => setSearchValue(event.target.value)}
        onSearch={(value) => setSearchValue(value)}
        className="logistics-upload-search"
      />
      <Typography.Link
        className="logistics-upload-alert"
        onClick={() => message.info('排查面板待接入')}
      >
        物流未上传？点击排查
      </Typography.Link>
      <Button
        size="small"
        icon={<BellOutlined />}
        onClick={() => message.info('失败提醒设置待接入')}
      >
        设置物流上传失败提醒
      </Button>
      <Button
        size="small"
        icon={<DownloadOutlined />}
        onClick={() => message.info('导出功能待接入')}
      >
        导出
      </Button>
      <Button
        size="small"
        icon={<QuestionCircleOutlined />}
        onClick={() => message.info('帮助中心待接入')}
      >
        帮助
      </Button>
    </Space>
  );

  const tableFooter = (
    <UiModule03TableFooter
      tableContainerRef={tableContainerRef}
      syncKey={`${pagination.current ?? 1}-${pagination.pageSize ?? 0}-${filteredRows.length}`}
      summary={
        <Space size={12}>
          <span>
            共 {filteredRows.length} 条，已勾选 {selectedRowKeys.length} 条
          </span>
          <Typography.Text type="secondary">查询耗时 0.116 秒</Typography.Text>
        </Space>
      }
      total={filteredRows.length}
      pagination={pagination}
      onPaginationChange={(next) => setPagination((prev) => ({ ...prev, ...next }))}
    />
  );

  return (
    <UiModule03MainTableSection<LogisticsUploadRow>
      className="logistics-upload-page"
      tableClassName="logistics-upload-table"
      tableContainerRef={tableContainerRef}
      toolbarLeft={toolbarLeft}
      toolbarRight={toolbarRight}
      columns={columns}
      data={pagedRows}
      loading={false}
      error={null}
      errorTitle="物流上传列表加载失败"
      emptyText="暂无待上传记录"
      onRetry={() => {}}
      total={filteredRows.length}
      pagination={pagination}
      onPaginationChange={(next) => setPagination((prev) => ({ ...prev, ...next }))}
      rowKey="id"
      rowSelection={{
        selectedRowKeys,
        onChange: (keys) => setSelectedRowKeys(keys)
      }}
      onRow={(record) => ({
        onClick: () => setActiveRowId(record.id)
      })}
      rowClassName={(record) =>
        record.id === activeRowId ? 'logistics-upload-row-active' : ''
      }
      tableFooter={tableFooter}
      bottomPanel={bottomTabs}
    />
  );
};
