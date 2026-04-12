import {
  Button,
  Card,
  DatePicker,
  Divider,
  Empty,
  Input,
  Select,
  Space,
  Table,
  Tabs,
  Tag,
  Typography,
  Tree,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { DataNode } from 'antd/es/tree';
import dayjs from 'dayjs';
import type { Key } from 'react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useQueries, useQuery } from '@tanstack/react-query';

import { useOrdersList } from '@/api/hooks';
import type { OrderDetail, OrderListQuery, OrderSummary } from '@/api/orders';
import { getOrderDetail } from '@/api/orders';
import { listShops } from '@/api/shops';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import {
  useWorkbenchTablePagination,
} from '@/components/workbench/templates';
import { buildOrderQuickFilterCells } from '@/components/ErpDataGrid';
import { useTenantId } from '@/features/tenant';

import './shopOrderQueryPage.css';

type ShopOrderSummary = OrderSummary & {
  platform_status?: string | null;
  order_status?: string | null;
  fulfillment_status?: string | null;
  exception_status?: string | null;
  payment_status?: string | null;
  sales_channel?: string | null;
  order_source?: string | null;
  logistics_company?: string | null;
};

type ShopOrderGoodsRow = {
  id: string;
  item_code: string;
  spec_code: string;
  title: string;
  spec: string;
  quantity: number;
  unit_price: number | null;
  discount: number | null;
  amount: number | null;
  trade_status: string;
  refund_status: string;
  ship_status: string;
  shipped_at?: string;
};

type ColumnFilters = {
  platformOrderId?: string;
  logisticsCompany?: string | string[];
  trackingNo?: string;
  tradeStatus?: string | string[];
  processingStatus?: string | string[];
  shopId?: string | string[];
  orderedRange?: [dayjs.Dayjs, dayjs.Dayjs];
};

type LeftFilters = {
  platformOrderId?: string;
  customerKeyword?: string;
  shopId?: string;
};

type ExtendedOrderListQuery = OrderListQuery & {
  archive_status?: string;
  order_status?: string;
  fulfillment_status?: string;
};

const timeRangeOptions = [
  { label: '近一个月', value: 'last_30_days', days: 30 },
  { label: '近七天', value: 'last_7_days', days: 7 },
  { label: '近三个月', value: 'last_90_days', days: 90 }
];

const archiveStatusOptions = [
  { label: '未归档数据', value: 'ACTIVE' },
  { label: '已归档数据', value: 'ARCHIVED' }
];

const logisticsOptions = [
  { label: '圆通速递', value: '圆通速递' },
  { label: '韵达快递', value: '韵达快递' },
  { label: '中通快递', value: '中通快递' },
  { label: '申通快递', value: '申通快递' }
];

const tradeStatusOptions = [
  { label: '待付款', value: 'PENDING_PAYMENT' },
  { label: '待发货', value: 'PENDING_SHIPMENT' },
  { label: '部分发货', value: 'PARTIALLY_SHIPPED' },
  { label: '已发货', value: 'SHIPPED' },
  { label: '已完成', value: 'COMPLETED' },
  { label: '交易关闭', value: 'CLOSED' },
  { label: '已退款', value: 'REFUNDED' }
];

const processingStatusOptions = [
  { label: '待复核', value: 'REVIEW_PENDING' },
  { label: '待发货', value: 'PENDING_SHIPMENT' },
  { label: '已分配', value: 'ALLOCATED' },
  { label: '拣货中', value: 'PICKING' },
  { label: '已打包', value: 'PACKED' },
  { label: '已发货', value: 'SHIPPED' },
  { label: '已完成', value: 'DONE' },
  { label: '已取消', value: 'CANCELLED' }
];

const filterTreeData: DataNode[] = [
  { key: 'all', title: '全部' },
  {
    key: 'refund',
    title: '退款状态',
    children: [
      { key: 'refund-none', title: '未退款' },
      { key: 'refund-partial', title: '部分退款' },
      { key: 'refund-done', title: '已退款' }
    ]
  },
  {
    key: 'reconcile',
    title: '对账状态',
    children: [
      { key: 'reconcile-none', title: '未对账' },
      { key: 'reconcile-done', title: '已对账' }
    ]
  },
  {
    key: 'upload',
    title: '物流上传状态',
    children: [
      { key: 'upload-none', title: '未上传' },
      { key: 'upload-done', title: '已上传' }
    ]
  },
  {
    key: 'trade',
    title: '网店订单交易状态',
    children: [
      { key: 'trade-pending', title: '待发货' },
      { key: 'trade-closed', title: '交易关闭' }
    ]
  },
  {
    key: 'promise',
    title: '承诺发货时间',
    children: [
      { key: 'promise-24', title: '24小时内' },
      { key: 'promise-48', title: '48小时内' },
      { key: 'promise-expired', title: '已超时' }
    ]
  },
  {
    key: 'channel',
    title: '销售渠道',
    children: [
      { key: 'channel-pdd', title: '拼多多' },
      { key: 'channel-dy', title: '抖音' },
      { key: 'channel-jd', title: '京东' }
    ]
  },
  {
    key: 'flag',
    title: '订单标记',
    children: [
      { key: 'flag-pre', title: '预' },
      { key: 'flag-return', title: '退' }
    ]
  }
];

const statusLabelMap: Record<string, string> = {
  NEW: '新建',
  REVIEW_PENDING: '待复核',
  PENDING_SHIPMENT: '待发货',
  ALLOCATED: '已分配',
  PICKING: '拣货中',
  PACKED: '已打包',
  SHIPPED: '已发货',
  DONE: '已完成',
  CANCELLED: '已取消',
  PENDING_PAYMENT: '待付款',
  PARTIALLY_SHIPPED: '部分发货',
  COMPLETED: '已完成',
  CLOSED: '交易关闭',
  REFUNDED: '已退款'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : (value / 100).toFixed(2);

const resolveStatusTag = (value?: string | null) => {
  if (!value) {
    return '-';
  }
  const normalized = value.toUpperCase();
  const label = statusLabelMap[normalized] ?? value;
  if (['CANCELLED', 'CLOSED', 'REFUNDED'].includes(normalized)) {
    return <Tag color="default">{label}</Tag>;
  }
  if (['PENDING_PAYMENT', 'REVIEW_PENDING', 'PENDING_SHIPMENT'].includes(normalized)) {
    return <Tag color="orange">{label}</Tag>;
  }
  if (['SHIPPED', 'DONE', 'COMPLETED'].includes(normalized)) {
    return <Tag color="green">{label}</Tag>;
  }
  return <Tag color="blue">{label}</Tag>;
};

const resolveFlagTag = (record: ShopOrderSummary) => {
  const orderStatus = (record.order_status ?? record.platform_status ?? '').toUpperCase();
  if (['REFUNDED', 'CANCELLED', 'CLOSED'].includes(orderStatus)) {
    return <Tag color="red">退</Tag>;
  }
  if ((record.exception_flags ?? []).length > 0) {
    return <Tag color="purple">预</Tag>;
  }
  return '-';
};

const resolveTimeRange = (value: string) => {
  const target = timeRangeOptions.find((option) => option.value === value);
  if (!target) {
    return undefined;
  }
  const end = dayjs();
  const start = end.subtract(target.days, 'day');
  return [start, end] as const;
};

const resolveMultiValue = (value?: string | string[]) => {
  if (!value) {
    return undefined;
  }
  if (Array.isArray(value)) {
    const filtered = value.filter((item) => item);
    return filtered.length ? filtered.join(',') : undefined;
  }
  return value;
};

export const ShopOrderQueryPage = () => {
  const tenantId = useTenantId();
  const { setLeftPanel, setLeftPanelVisible, setRightPanel, setRightPanelVisible, clearPanels } =
    useWorkbenchPanels();
  const [selectedKeys, setSelectedKeys] = useState<string[]>(['all']);
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [activeOrderId, setActiveOrderId] = useState<string | null>(null);
  const [timeRange, setTimeRange] = useState('last_30_days');
  const [archiveStatus, setArchiveStatus] = useState('ACTIVE');
  const { pagination, resetPagination, handlePaginationChange } = useWorkbenchTablePagination({
    defaultPageSize: 20,
    pageSizeOptions: ['20', '50', '100', '200'],
  });
  const tableContainerRef = useRef<HTMLDivElement | null>(null);
  const [searchValue, setSearchValue] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const [leftPlatformOrderId, setLeftPlatformOrderId] = useState('');
  const [leftCustomerKeyword, setLeftCustomerKeyword] = useState('');
  const [leftShopId, setLeftShopId] = useState<string | undefined>(undefined);
  const [appliedLeftFilters, setAppliedLeftFilters] = useState<LeftFilters>({});
  const [columnFilters, setColumnFilters] = useState<ColumnFilters>({});
  const [debouncedColumnFilters, setDebouncedColumnFilters] = useState<ColumnFilters>({});

  const shopQuery = useQuery({
    queryKey: ['shops', tenantId, 'shop-order-query'],
    queryFn: () => listShops(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000
  });
  const shopOptions = useMemo(
    () =>
      shopQuery.data?.items.map((shop) => ({
        label: `${shop.name} (${shop.shop_id})`,
        value: shop.shop_id
      })) ?? [],
    [shopQuery.data?.items]
  );
  const shopMap = useMemo(() => {
    const map = new Map<string, string>();
    shopQuery.data?.items.forEach((shop) => map.set(shop.shop_id, shop.name));
    return map;
  }, [shopQuery.data?.items]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedColumnFilters(columnFilters);
    }, 300);
    return () => window.clearTimeout(timer);
  }, [columnFilters]);

  useEffect(() => {
    resetPagination();
  }, [appliedLeftFilters, debouncedColumnFilters, searchKeyword, timeRange, archiveStatus]);

  const query = useMemo<ExtendedOrderListQuery>(() => {
    const range = debouncedColumnFilters.orderedRange ?? resolveTimeRange(timeRange);
    const resolvedShopId =
      resolveMultiValue(debouncedColumnFilters.shopId) ?? appliedLeftFilters.shopId;
    return {
      platform_order_id:
        debouncedColumnFilters.platformOrderId?.trim() ||
        appliedLeftFilters.platformOrderId?.trim() ||
        undefined,
      logistics_company: resolveMultiValue(debouncedColumnFilters.logisticsCompany),
      tracking_no: debouncedColumnFilters.trackingNo?.trim() || undefined,
      order_status: resolveMultiValue(debouncedColumnFilters.tradeStatus),
      fulfillment_status: resolveMultiValue(debouncedColumnFilters.processingStatus),
      shop_id: resolvedShopId,
      keyword: searchKeyword.trim() || appliedLeftFilters.customerKeyword?.trim() || undefined,
      created_from: range?.[0]?.toISOString(),
      created_to: range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20,
      archive_status: archiveStatus
    };
  }, [
    appliedLeftFilters.customerKeyword,
    appliedLeftFilters.platformOrderId,
    appliedLeftFilters.shopId,
    archiveStatus,
    debouncedColumnFilters.logisticsCompany,
    debouncedColumnFilters.orderedRange,
    debouncedColumnFilters.platformOrderId,
    debouncedColumnFilters.processingStatus,
    debouncedColumnFilters.shopId,
    debouncedColumnFilters.tradeStatus,
    debouncedColumnFilters.trackingNo,
    pagination.current,
    pagination.pageSize,
    searchKeyword,
    timeRange
  ]);

  const { data, isFetching, error, refetch } = useOrdersList({
    query: query as OrderListQuery
  });
  const listItems = (data?.items ?? []) as ShopOrderSummary[];
  const total = data?.total ?? 0;

  const orderIds = useMemo(() => listItems.map((item) => item.id), [listItems]);
  const detailQueries = useQueries({
    queries: orderIds.map((orderId) => ({
      queryKey: ['order-detail', tenantId, orderId],
      queryFn: () => getOrderDetail(tenantId, orderId),
      enabled: Boolean(tenantId && orderId),
      staleTime: 60_000
    }))
  });
  const detailRevision = detailQueries
    .map((queryResult) => queryResult.dataUpdatedAt ?? 0)
    .join('|');
  const detailMap = useMemo(() => {
    const map = new Map<string, OrderDetail>();
    detailQueries.forEach((queryResult, index) => {
      const orderId = orderIds[index];
      if (orderId && queryResult.data) {
        map.set(orderId, queryResult.data);
      }
    });
    return map;
  }, [detailRevision, orderIds]);

  const activeOrderDetail = useMemo(
    () => (activeOrderId ? detailMap.get(activeOrderId) ?? null : null),
    [activeOrderId, detailMap]
  );

  useEffect(() => {
    if (listItems.length === 0) {
      setSelectedRowKeys([]);
      setActiveOrderId(null);
      return;
    }
    if (activeOrderId && listItems.some((item) => item.id === activeOrderId)) {
      return;
    }
    const nextId = listItems[0]?.id ?? null;
    setActiveOrderId(nextId);
    setSelectedRowKeys(nextId ? [nextId] : []);
  }, [activeOrderId, listItems]);

  const goodsRows = useMemo<ShopOrderGoodsRow[]>(() => {
    if (!activeOrderDetail) {
      return [];
    }
    return activeOrderDetail.items.map((item) => {
      const amountCent = item.price_cent * item.quantity;
      return {
        id: item.id,
        item_code: item.platform_sku_id ?? item.outer_sku_id ?? '-',
        spec_code: item.outer_sku_id ?? item.platform_sku_id ?? '-',
        title: item.title ?? '-',
        spec: item.outer_sku_id ?? '-',
        quantity: item.quantity,
        unit_price: item.price_cent ?? null,
        discount: null,
        amount: amountCent ?? null,
        trade_status: activeOrderDetail.order.platform_status ?? '-',
        refund_status: '-',
        ship_status: activeOrderDetail.order.oms_status ?? '-',
        shipped_at: activeOrderDetail.order.ship_at ?? undefined
      };
    });
  }, [activeOrderDetail]);

  const resolveLogisticsCompany = (record: ShopOrderSummary) =>
    record.logistics_company ?? detailMap.get(record.id)?.order.logistics_company ?? '-';

  const resolveChannel = (record: ShopOrderSummary) =>
    record.sales_channel ??
    record.order_source ??
    shopMap.get(record.shop_id) ??
    record.shop_id ??
    '-';

  const resolvePayAt = (record: ShopOrderSummary) =>
    formatDateTime(detailMap.get(record.id)?.order.pay_at ?? null);

  const resolvePromisedShipAt = (record: ShopOrderSummary) =>
    formatDateTime(detailMap.get(record.id)?.order.ship_at ?? null);

const resolveUploadStatus = (record: ShopOrderSummary, detailMap: Map<string, OrderDetail>) => {
  const trackingNo =
    record.tracking_no ?? detailMap.get(record.id)?.order.tracking_no ?? null;
  const label = trackingNo ? '已上传' : '未上传';
  const color = trackingNo ? 'green' : 'red';
  return <Tag color={color}>{label}</Tag>;
};

  const columns = useMemo<ColumnsType<ShopOrderSummary>>(
    () => [
      {
        title: '订单标记',
        key: 'flag',
        width: 80,
        render: (_, record) => resolveFlagTag(record)
      },
      {
        title: '网店订单号',
        dataIndex: 'platform_order_id',
        key: 'platform_order_id',
        width: 200
      },
      {
        title: '物流公司',
        dataIndex: 'logistics_company',
        key: 'logistics_company',
        width: 140,
        render: (_, record) => resolveLogisticsCompany(record)
      },
      {
        title: '物流单号',
        dataIndex: 'tracking_no',
        key: 'tracking_no',
        width: 150,
        render: (value?: string | null) => value || '-'
      },
      {
        title: '物流上传状态',
        key: 'logistics_upload_status',
        width: 120,
        render: (_, record) => resolveUploadStatus(record, detailMap)
      },
      {
        title: '承诺发货时间',
        key: 'promised_ship_at',
        width: 160,
        render: (_, record) => resolvePromisedShipAt(record)
      },
      {
        title: '销售渠道',
        dataIndex: 'shop_id',
        key: 'shop_id',
        width: 180,
        render: (_, record) => resolveChannel(record)
      },
      {
        title: '网店订单交易状态',
        key: 'trade_status',
        width: 160,
        render: (_, record) =>
          resolveStatusTag(record.platform_status ?? record.order_status ?? null)
      },
      {
        title: '处理状态',
        key: 'processing_status',
        width: 140,
        render: (_, record) =>
          resolveStatusTag(record.fulfillment_status ?? record.oms_status ?? null)
      },
      {
        title: '下单时间',
        dataIndex: 'created_at',
        key: 'ordered_at',
        width: 160,
        render: (value?: string | null) => formatDateTime(value ?? null)
      },
      {
        title: '付款时间',
        key: 'paid_at',
        width: 160,
        render: (_, record) => resolvePayAt(record)
      }
    ],
    [detailMap, shopMap]
  );

  const goodsColumns = useMemo<ColumnsType<ShopOrderGoodsRow>>(
    () => [
      { title: '网店商品编码', dataIndex: 'item_code', key: 'item_code', width: 120 },
      { title: '网店规格编码', dataIndex: 'spec_code', key: 'spec_code', width: 120 },
      { title: '交易名称', dataIndex: 'title', key: 'title', width: 220 },
      { title: '交易规格', dataIndex: 'spec', key: 'spec', width: 140 },
      {
        title: '数量',
        dataIndex: 'quantity',
        key: 'quantity',
        width: 80,
        align: 'right'
      },
      {
        title: '单价',
        dataIndex: 'unit_price',
        key: 'unit_price',
        width: 80,
        align: 'right',
        render: (value: number | null) => formatCurrency(value == null ? null : value)
      },
      {
        title: '优惠',
        dataIndex: 'discount',
        key: 'discount',
        width: 80,
        align: 'right',
        render: (value: number | null) => formatCurrency(value == null ? null : value)
      },
      {
        title: '金额',
        dataIndex: 'amount',
        key: 'amount',
        width: 80,
        align: 'right',
        render: (value: number | null) => formatCurrency(value == null ? null : value)
      },
      { title: '交易状态', dataIndex: 'trade_status', key: 'trade_status', width: 120 },
      { title: '退款状态', dataIndex: 'refund_status', key: 'refund_status', width: 120 },
      { title: '发货状态', dataIndex: 'ship_status', key: 'ship_status', width: 120 },
      {
        title: '发货时间',
        dataIndex: 'shipped_at',
        key: 'shipped_at',
        width: 140,
        render: (value?: string) => formatDateTime(value ?? null)
      }
    ],
    []
  );

  const updateColumnFilter = (
    key: keyof ColumnFilters,
    value: string | string[] | undefined
  ) => {
    const normalized = Array.isArray(value)
      ? value.length
        ? value
        : undefined
      : value || undefined;
    setColumnFilters((prev) => ({ ...prev, [key]: normalized }));
  };

  const queryCells = useMemo(
    () =>
      buildOrderQuickFilterCells([
        {
          key: 'flag',
          type: 'custom',
          node: (
            <Select
              size="small"
              allowClear
              mode="multiple"
              style={{ width: '100%' }}
              options={[
                { label: '预', value: 'PRE' },
                { label: '退', value: 'RETURN' }
              ]}
              placeholder="标记"
            />
          )
        },
        {
          key: 'platform_order_id',
          type: 'input',
          placeholder: '模糊查询',
          value: searchValue,
          onChange: (value) => {
            const nextValue = String(value ?? '');
            handleKeywordChange(nextValue);
            updateColumnFilter('platformOrderId', nextValue);
          },
          onSearch: (value) => {
            handleKeywordSearch(value);
            updateColumnFilter('platformOrderId', value);
          }
        },
        {
          key: 'logistics_company',
          type: 'select',
          placeholder: '物流公司',
          options: logisticsOptions,
          multiple: true,
          value: columnFilters.logisticsCompany,
          onChange: (value) =>
            updateColumnFilter('logisticsCompany', value as string | string[] | undefined)
        },
        {
          key: 'tracking_no',
          type: 'input',
          placeholder: '物流单号',
          value: columnFilters.trackingNo ?? '',
          onChange: (value) => updateColumnFilter('trackingNo', value as string)
        },
        {
          key: 'logistics_upload_status',
          type: 'custom',
          node: <Input size="small" disabled placeholder="已上传/未上传" style={{ width: '100%' }} />
        },
        {
          key: 'promised_ship_at',
          type: 'custom',
          node: (
            <DatePicker size="small" style={{ width: '100%' }} placeholder="承诺发货时间" />
          )
        },
        {
          key: 'shop_id',
          type: 'select',
          placeholder: '销售渠道',
          options: shopOptions,
          multiple: true,
          value: columnFilters.shopId,
          onChange: (value) => updateColumnFilter('shopId', value as string | string[] | undefined)
        },
        {
          key: 'trade_status',
          type: 'select',
          placeholder: '交易状态',
          options: tradeStatusOptions,
          multiple: true,
          value: columnFilters.tradeStatus,
          onChange: (value) =>
            updateColumnFilter('tradeStatus', value as string | string[] | undefined)
        },
        {
          key: 'processing_status',
          type: 'select',
          placeholder: '处理状态',
          options: processingStatusOptions,
          multiple: true,
          value: columnFilters.processingStatus,
          onChange: (value) =>
            updateColumnFilter('processingStatus', value as string | string[] | undefined)
        },
        {
          key: 'ordered_at',
          type: 'date-time-range',
          rangePlaceholder: ['下单开始', '下单结束'],
          value: columnFilters.orderedRange,
          onChange: (value) =>
            setColumnFilters((prev) => ({
              ...prev,
              orderedRange: value as [dayjs.Dayjs, dayjs.Dayjs] | undefined
            }))
        },
        {
          key: 'paid_at',
          type: 'custom',
          node: <DatePicker size="small" style={{ width: '100%' }} placeholder="付款时间" />
        }
      ]),
    [
      columnFilters,
      handleKeywordChange,
      handleKeywordSearch,
      logisticsOptions,
      processingStatusOptions,
      searchValue,
      shopOptions,
      tradeStatusOptions
    ]
  );

  const tableFooterSyncKey = useMemo(
    () => `${columns.length}-${listItems.length}`,
    [columns.length, listItems.length]
  );

  const handleApplyLeftFilters = () => {
    setAppliedLeftFilters({
      platformOrderId: leftPlatformOrderId.trim() || undefined,
      customerKeyword: leftCustomerKeyword.trim() || undefined,
      shopId: leftShopId
    });
  };

  const handleResetLeftFilters = () => {
    setLeftPlatformOrderId('');
    setLeftCustomerKeyword('');
    setLeftShopId(undefined);
    setAppliedLeftFilters({});
  };

  const handleKeywordChange = (value: string) => {
    setSearchValue(value);
    if (!value.trim()) {
      setSearchKeyword('');
    }
  };

  const handleKeywordSearch = (value: string) => {
    setSearchKeyword(value.trim());
  };

  const handleSearch = (value: string) => {
    handleKeywordSearch(value);
  };

  const handleSearchChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    handleKeywordChange(event.target.value);
  };

  const leftPanel = useMemo(
    () => (
      <Card size="small" className="shop-order-query-left">
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Typography.Text strong>过滤</Typography.Text>
          <Tree
            blockNode
            defaultExpandAll
            selectedKeys={selectedKeys}
            onSelect={(keys) => setSelectedKeys([String(keys[0] ?? 'all')])}
            treeData={filterTreeData}
          />
          <Divider style={{ margin: '4px 0' }} />
          <Typography.Text strong>筛选</Typography.Text>
          <Space direction="vertical" size={8} style={{ width: '100%' }}>
            <Input
              size="small"
              allowClear
              placeholder="网店订单号"
              value={leftPlatformOrderId}
              onChange={(event) => setLeftPlatformOrderId(event.target.value)}
            />
            <Input
              size="small"
              allowClear
              placeholder="客户账号/手机号"
              value={leftCustomerKeyword}
              onChange={(event) => setLeftCustomerKeyword(event.target.value)}
            />
            <Select
              size="small"
              allowClear
              options={shopOptions}
              value={leftShopId}
              placeholder="销售渠道"
              style={{ width: '100%' }}
              onChange={(value) => setLeftShopId(value)}
            />
            <Space size={8}>
              <Button size="small" type="primary" onClick={handleApplyLeftFilters}>
                应用筛选
              </Button>
              <Button size="small" onClick={handleResetLeftFilters}>
                重置
              </Button>
            </Space>
          </Space>
        </Space>
      </Card>
    ),
    [
      handleApplyLeftFilters,
      handleResetLeftFilters,
      leftCustomerKeyword,
      leftPlatformOrderId,
      leftShopId,
      selectedKeys,
      shopOptions
    ]
  );

  const bottomTabs = useMemo(
    () => (
      <div className="shop-order-query-bottom-tabs">
        <Tabs
          size="small"
          type="card"
          items={[
            {
              key: 'goods',
              label: '货品',
              children: activeOrderDetail ? (
                <Table
                  size="small"
                  rowKey="id"
                  columns={goodsColumns}
                  dataSource={goodsRows}
                  pagination={false}
                  scroll={{ x: 'max-content' }}
                />
              ) : (
                <Empty description="请选择订单查看货品明细" />
              )
            },
            {
              key: 'logs',
              label: '日志',
              children: <Empty description="暂无日志" />
            },
            {
              key: 'sales',
              label: '销售单',
              children: <Empty description="暂无销售单" />
            },
            {
              key: 'shipments',
              label: '发货单',
              children: <Empty description="暂无发货单" />
            },
            {
              key: 'aftersales',
              label: '售后单',
              children: <Empty description="暂无售后单" />
            }
          ]}
        />
      </div>
    ),
    [activeOrderDetail, goodsColumns, goodsRows]
  );

  useEffect(() => {
    setLeftPanel(leftPanel);
  }, [leftPanel, setLeftPanel]);

  useEffect(() => {
    setLeftPanelVisible(true);
    setRightPanel(undefined);
    setRightPanelVisible(false);
    return () => clearPanels();
  }, [clearPanels, setLeftPanelVisible, setRightPanel, setRightPanelVisible]);

  const toolbarLeft = (
    <Space size={12} align="center">
      <Typography.Title level={4} style={{ margin: 0 }}>
        网店订单查询
      </Typography.Title>
      <Select
        size="small"
        value={timeRange}
        options={timeRangeOptions}
        onChange={setTimeRange}
        style={{ width: 120 }}
      />
      <Select
        size="small"
        value={archiveStatus}
        options={archiveStatusOptions}
        onChange={setArchiveStatus}
        style={{ width: 120 }}
      />
    </Space>
  );

  const toolbarCenter = (
    <Input.Search
      size="small"
      allowClear
      value={searchValue}
      onChange={handleSearchChange}
      onSearch={handleSearch}
      className="shop-order-query-search"
      placeholder="网店订单号/客户账号/交易名称/手机号"
    />
  );

  const toolbarRight = (
    <Space size={8}>
      <Button size="small" onClick={() => message.info('导出功能待接入')}>
        导出
      </Button>
      <Button size="small" onClick={() => message.info('帮助文档待接入')}>
        帮助
      </Button>
    </Space>
  );

  const tableHeader = (
    <div className="shop-order-query-header">
      <Button type="link" size="small" onClick={() => message.info('排查入口待接入')}>
        订单数据加载中（点击排查）
      </Button>
    </div>
  );

  return (
    <UiModule03MainTableSection<ShopOrderSummary>
      className="shop-order-query-page"
      tableContainerRef={tableContainerRef}
      toolbarLeft={toolbarLeft}
      toolbarCenter={toolbarCenter}
      toolbarRight={toolbarRight}
      header={tableHeader}
      columns={columns}
      data={listItems}
      loading={isFetching && listItems.length === 0}
      error={error}
      errorTitle="网店订单加载失败"
      onRetry={() => refetch()}
      total={total}
      pagination={pagination}
      onPaginationChange={handlePaginationChange}
      columnSettingsKey="sales.shop-order-query.main-grid"
      showColumnSettings={false}
      queryCells={queryCells}
      rowKey="id"
      rowSelection={{
        selectedRowKeys,
        onChange: (keys, rows) => {
          setSelectedRowKeys(keys);
          setActiveOrderId(rows[0]?.id ?? null);
        }
      }}
      onRow={(record) => ({
        onClick: () => {
          setSelectedRowKeys([record.id]);
          setActiveOrderId(record.id);
        }
      })}
      footerSummary={`共 ${total} 条，已勾选 ${selectedRowKeys.length} 条`}
      showStatusBar={false}
      bottomPanel={bottomTabs}
      enablePanels={false}
      />
  );
};

