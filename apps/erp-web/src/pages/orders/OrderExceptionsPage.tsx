import './orderReviewPage.css';
import './orderExceptionsPage.css';

import { ReloadOutlined } from '@ant-design/icons';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Button, Descriptions, Empty, List, Spin, Tag, Typography, message } from 'antd';
import type { MenuProps, SelectProps } from 'antd';
import type { GridApi, Key } from 'ag-grid-community';
import dayjs from 'dayjs';
import { useCallback, useDeferredValue, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useNavigate } from 'react-router-dom';

import { listCouriers } from '@/api/couriers';
import {
  addOrderTags,
  submitOrdersForFulfillment,
  getOrderExceptionDetail,
  listOrderExceptions,
  resolveOrderException,
  updateOrder,
  type FulfillmentPrecheckResult,
  type OrderExceptionDetail,
  type OrderExceptionSummary,
} from '@/api/orders';
import { listOrderTagDefinitions } from '@/api/orderTagDefinitions';
import { listWarehouses } from '@/api/warehouses';
import { useWorkbenchPanels } from '@/app/layout/workbenchPanels';
import {
  WorkbenchMainTableTemplate,
  WorkbenchToolbarTemplate,
  useWorkbenchTablePagination,
} from '@/components/workbench/templates';
import { UiModule03ContextMenu, UiModule05OrderReviewRightPanelTemplate } from '@/components/ui-modules';
import { auditStatusLabels, exceptionStatusLabels } from '@/features/oms-wms-main-chain/statuses';
import { useSessionStore } from '@/features/sessionStore';
import { useTenantId } from '@/features/tenant';
import { buildCourierOptionLabel, resolveCourierOptionValue } from './shared/courierDisplay';

import { OrderExceptionsMainGrid } from './exceptions-native/OrderExceptionsMainGrid';
import { toOrderExceptionsGridRows } from './exceptions-native/orderExceptionsGridRows';
import {
  filterOrderExceptionSelectionKeys,
  findOrderExceptionRowIndex,
  resolveNextOrderExceptionActiveKey,
  resolveOrderExceptionRowKey,
} from './exceptions-native/orderExceptionsRowKey';
import { useOrderExceptionsLeftModule } from './exceptions-native/useOrderExceptionsLeftModule';
import { OrderReviewBottomPanel } from './review-native/OrderReviewBottomPanel';
import { defaultPageSize, pageSizeOptions } from './review-native/helpers';
import { exportOrderReviewGridAsExcel, printOrderReviewGrid } from './review-native/orderReviewGridActions';

const overdueThresholdMinutes = 15;

type OrderExceptionsPageMode = 'pool' | 'sla';

type OrderExceptionsPageProps = {
  mode?: OrderExceptionsPageMode;
};

type DetailSelectOption = NonNullable<SelectProps['options']>;

type ContextMenuState<T> = {
  open: boolean;
  position: { x: number; y: number } | null;
  row: T | null;
};

type GoodsGridRow = {
  __localKey: string;
  id?: string;
  platform_sku_id?: string | null;
  outer_sku_id?: string | null;
  title?: string | null;
  quantity?: number | null;
  price_cent?: number | null;
};

const toGoodsRows = (rows: Record<string, unknown>[]) =>
  rows.map((row, index) => {
    const id = row.id == null ? undefined : String(row.id);
    const fallbackId = `${String(row.platform_sku_id ?? row.outer_sku_id ?? 'goods')}-${index}`;
    const localKey = id ?? fallbackId;
    return {
      __localKey: localKey,
      id,
      platform_sku_id: row.platform_sku_id == null ? null : String(row.platform_sku_id),
      outer_sku_id: row.outer_sku_id == null ? null : String(row.outer_sku_id),
      title: row.title == null ? null : String(row.title),
      quantity: row.quantity == null ? 1 : Number(row.quantity),
      price_cent: row.price_cent == null ? 0 : Number(row.price_cent),
    } as GoodsGridRow;
  });

const formatCurrency = (value?: number | null) => (value == null ? '-' : `¥${(value / 100).toFixed(2)}`);

const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

const resolveAuditLabel = (status?: string | null) => {
  const normalized = String(status ?? '').trim().toUpperCase();
  if (normalized === 'REVIEW_REQUIRED' || normalized === 'UNDER_REVIEW') {
    return '待复核';
  }
  return auditStatusLabels[(status ?? '') as keyof typeof auditStatusLabels] ?? status ?? '-';
};

const resolveExceptionLabel = (status?: string | null) =>
  exceptionStatusLabels[(status ?? '') as keyof typeof exceptionStatusLabels] ?? status ?? '-';

const resolveExceptionTagColor = (status?: string | null) => {
  switch (status) {
    case 'OPEN':
      return 'orange';
    case 'ASSIGNED':
      return 'blue';
    case 'PROCESSING':
      return 'processing';
    case 'ESCALATED':
      return 'red';
    case 'RESOLVED':
      return 'success';
    case 'CLOSED':
      return 'default';
    default:
      return 'default';
  }
};

const isOverdue = (item: OrderExceptionSummary) => {
  if (!item.latest_action_at) {
    return false;
  }

  return dayjs().diff(dayjs(item.latest_action_at), 'minute') >= overdueThresholdMinutes;
};

const buildActionPayload = (userId: string, userName: string, reason?: string) => ({
  operator_id: userId || null,
  operator_name: userName || null,
  reason: reason ?? null,
});

const renderDetailSection = (title: string, content: ReactNode, testId?: string) => (
  <section className="order-exceptions-detail-section" data-testid={testId}>
    <div className="order-exceptions-detail-section-header">
      <Typography.Text strong>{title}</Typography.Text>
    </div>
    <div className="order-exceptions-detail-section-body">{content}</div>
  </section>
);

const renderDetailContent = (activeRow: OrderExceptionSummary | null, detail: OrderExceptionDetail | undefined, loading: boolean) => {
  if (!activeRow) {
    return (
      <div className="order-exceptions-detail-empty">
        <Empty description="请选择异常订单查看复核上下文" />
      </div>
    );
  }

  if (loading) {
    return (
      <div className="order-exceptions-detail-empty">
        <Spin />
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="order-exceptions-detail-empty">
        <Empty description="未获取到异常详情" />
      </div>
    );
  }

  return (
    <>
      <div className="order-exceptions-detail-summary">
        <div>
          <Typography.Title level={5} className="order-exceptions-detail-title">
            {detail.order_no}
          </Typography.Title>
          <Typography.Text type="secondary">
            {detail.receiver_name || '-'} / {detail.target_warehouse_id || '未指定仓库'}
          </Typography.Text>
        </div>
        <Tag color={resolveExceptionTagColor(detail.exception_status)}>{resolveExceptionLabel(detail.exception_status)}</Tag>
      </div>

      {renderDetailSection(
        '主链上下文',
        <Descriptions column={1} size="small">
          <Descriptions.Item label="审核状态">{resolveAuditLabel(detail.audit_status)}</Descriptions.Item>
          <Descriptions.Item label="履约状态">{detail.fulfillment_status}</Descriptions.Item>
          <Descriptions.Item label="库存占用状态">{detail.reserve_status || '未预占'}</Descriptions.Item>
          <Descriptions.Item label="责任人">{detail.assignee_name || '待领取'}</Descriptions.Item>
          <Descriptions.Item label="最近处理时间">{formatDateTime(detail.latest_action_at)}</Descriptions.Item>
        </Descriptions>,
      )}

      {renderDetailSection(
        '阻断原因',
        <List
          size="small"
          dataSource={detail.blocking_reasons}
          locale={{ emptyText: '暂无阻断原因' }}
          renderItem={(item) => (
            <List.Item>
              <Typography.Text>{item}</Typography.Text>
            </List.Item>
          )}
        />,
      )}

      {renderDetailSection(
        '最近处理记录',
        <List
          size="small"
          dataSource={detail.action_logs.slice().reverse()}
          locale={{ emptyText: '暂无处理记录' }}
          renderItem={(record) => (
            <List.Item>
              <div className="order-exceptions-log-item">
                <div className="order-exceptions-log-main">
                  <Typography.Text>{record.message}</Typography.Text>
                  <Typography.Text type="secondary">{record.operator_name || record.operator_id || '系统'}</Typography.Text>
                </div>
                <Typography.Text type="secondary">{formatDateTime(record.created_at)}</Typography.Text>
              </div>
            </List.Item>
          )}
        />,
      )}
    </>
  );
};

export const OrderExceptionsPage = ({ mode = 'pool' }: OrderExceptionsPageProps = {}) => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const userId = useSessionStore((state) => state.userId);
  const userName = useSessionStore((state) => state.userName);
  const queryClient = useQueryClient();
  const { setRightPanel, setRightPanelVisible, setBottomPanel, setBottomPanelVisible } = useWorkbenchPanels();

  const [keywordInput, setKeywordInput] = useState('');
  const [searchKeyword, setSearchKeyword] = useState('');
  const deferredKeyword = useDeferredValue(searchKeyword.trim());
  const [quickFilterExpanded, setQuickFilterExpanded] = useState(true);
  const { pagination, setPagination, resetPagination, handlePaginationChange } = useWorkbenchTablePagination({
    defaultPageSize,
    pageSizeOptions,
  });
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [activeOrderId, setActiveOrderId] = useState<string | null>(null);
  const [focusedOrderKey, setFocusedOrderKey] = useState<string | null>(null);
  const [mainContextMenu, setMainContextMenu] = useState<ContextMenuState<OrderExceptionSummary>>({
    open: false,
    position: null,
    row: null,
  });
  const tableRef = useRef<HTMLDivElement | null>(null);
  const mainGridApiRef = useRef<GridApi<OrderExceptionSummary> | null>(null);

  const exceptionQuery = useQuery({
    queryKey: ['order-exceptions', tenantId, deferredKeyword],
    queryFn: () =>
      listOrderExceptions(tenantId, {
        keyword: deferredKeyword || undefined,
      }),
    enabled: Boolean(tenantId),
    refetchOnMount: 'always',
  });

  const rows = useMemo(() => toOrderExceptionsGridRows(exceptionQuery.data?.items ?? []), [exceptionQuery.data?.items]);
  const mainGridLoading = exceptionQuery.isLoading || (exceptionQuery.isFetching && rows.length === 0);
  const { filteredRows } = useOrderExceptionsLeftModule({
    rows,
    currentUserId: userId,
    isFetching: exceptionQuery.isFetching,
    overdueThresholdMinutes,
    resolveExceptionLabel,
    isOverdue,
  });

  const sortedRows = useMemo(
    () =>
      mode === 'sla'
        ? filteredRows
            .slice()
            .sort((left, right) => {
              const leftOverdue = isOverdue(left) ? 1 : 0;
              const rightOverdue = isOverdue(right) ? 1 : 0;
              if (rightOverdue !== leftOverdue) {
                return rightOverdue - leftOverdue;
              }
              const leftTime = left.latest_action_at ? dayjs(left.latest_action_at).valueOf() : 0;
              const rightTime = right.latest_action_at ? dayjs(right.latest_action_at).valueOf() : 0;
              return leftTime - rightTime;
            })
        : filteredRows,
    [filteredRows, mode],
  );

  const displayRows = sortedRows;
  const resolvedPageSize = useMemo(() => {
    const rawPageSize = Number(pagination.pageSize ?? defaultPageSize);
    return Number.isFinite(rawPageSize) && rawPageSize > 0 ? rawPageSize : defaultPageSize;
  }, [pagination.pageSize]);
  const resolvedCurrentPage = useMemo(() => {
    const rawCurrentPage = Number(pagination.current ?? 1);
    return Number.isFinite(rawCurrentPage) && rawCurrentPage > 0 ? rawCurrentPage : 1;
  }, [pagination.current]);
  const pagedRows = useMemo(() => {
    const start = Math.max(0, (resolvedCurrentPage - 1) * resolvedPageSize);
    return displayRows.slice(start, start + resolvedPageSize);
  }, [displayRows, resolvedCurrentPage, resolvedPageSize]);
  const activeRow = pagedRows.find((item) => resolveOrderExceptionRowKey(item) === activeOrderId) ?? null;

  useEffect(() => {
    const totalPages = Math.max(1, Math.ceil(displayRows.length / resolvedPageSize));
    if (resolvedCurrentPage <= totalPages) {
      return;
    }
    setPagination((current) => ({
      ...current,
      current: totalPages,
      pageSize: resolvedPageSize,
      pageSizeOptions,
    }));
  }, [displayRows.length, resolvedCurrentPage, resolvedPageSize]);

  useEffect(() => {
    if (displayRows.length === 0) {
      setSelectedRowKeys([]);
      setActiveOrderId(null);
      setFocusedOrderKey(null);
      return;
    }

    setSelectedRowKeys((current) => filterOrderExceptionSelectionKeys(displayRows, current));

    setActiveOrderId((current) => {
      return resolveNextOrderExceptionActiveKey(pagedRows, current);
    });

    setFocusedOrderKey((current) => {
      return resolveNextOrderExceptionActiveKey(pagedRows, current);
    });
  }, [displayRows, pagedRows]);

  useEffect(() => {
    if (!tenantId) {
      return;
    }
    if (exceptionQuery.fetchStatus !== 'idle' || exceptionQuery.data !== undefined) {
      return;
    }
    void exceptionQuery.refetch();
  }, [tenantId, exceptionQuery.data, exceptionQuery.fetchStatus, exceptionQuery.refetch]);

  const detailQuery = useQuery({
    queryKey: ['order-exception-detail', tenantId, activeOrderId],
    queryFn: () => getOrderExceptionDetail(tenantId, activeOrderId!),
    enabled: Boolean(tenantId && activeOrderId),
    staleTime: 20_000,
    gcTime: 5 * 60_000,
  });

  useEffect(() => {
    if (!tenantId || !activeOrderId || !pagedRows.length) {
      return;
    }
    const currentIndex = findOrderExceptionRowIndex(pagedRows, activeOrderId);
    if (currentIndex < 0) {
      return;
    }
    const neighborIndices = [currentIndex - 1, currentIndex + 1].filter((index) => index >= 0 && index < pagedRows.length);
    neighborIndices.forEach((index) => {
      const record = pagedRows[index];
      if (!record) return;
      void queryClient.prefetchQuery({
        queryKey: ['order-exception-detail', tenantId, record.order_id],
        queryFn: () => getOrderExceptionDetail(tenantId, record.order_id),
        staleTime: 20_000,
        gcTime: 5 * 60_000,
      });
    });
  }, [activeOrderId, pagedRows, queryClient, tenantId]);

  const detailPackage = (detailQuery.data?.order ?? null) as
    | {
        order?: Record<string, unknown> | null;
        items?: Record<string, unknown>[];
        tags?: Array<{ tag?: string | null }>;
      }
    | null;
  const detailOrder = (detailPackage?.order ?? null) as Record<string, unknown> | null;
  const detailOrderId = String(detailOrder?.id ?? activeOrderId ?? '').trim();
  const detailLoadedId = detailOrderId;
  const goodsRows = useMemo(() => toGoodsRows((detailPackage?.items ?? []) as Record<string, unknown>[]), [detailPackage?.items]);
  const logRows = useMemo(
    () =>
      (detailQuery.data?.action_logs ?? []).map((log, index) => ({
        id: `${detailOrderId || 'exception'}-${index}`,
        created_at: log.created_at,
        action: log.message ?? '-',
        operator: log.operator_name || log.operator_id || '绯荤粺',
        detail: log.reason || log.message || '-',
      })),
    [detailOrderId, detailQuery.data?.action_logs],
  );
  const [goodsRowsState, setGoodsRowsState] = useState<GoodsGridRow[]>([]);
  const [goodsSaving, setGoodsSaving] = useState(false);
  const [detailWarehouseId, setDetailWarehouseId] = useState<string | undefined>(undefined);
  const [detailLogisticsCompany, setDetailLogisticsCompany] = useState<string | undefined>(undefined);
  const [detailUpdatingField, setDetailUpdatingField] = useState<'warehouse' | 'logistics' | null>(null);
  const [orderTagsApplying, setOrderTagsApplying] = useState(false);

  useEffect(() => {
    setGoodsRowsState(goodsRows);
  }, [detailOrderId, goodsRows]);

  useEffect(() => {
    const nextWarehouseId = String(detailOrder?.ship_warehouse_id ?? '').trim();
    const nextLogisticsCompany = String(detailOrder?.logistics_company ?? '').trim();
    setDetailWarehouseId(nextWarehouseId || undefined);
    setDetailLogisticsCompany(nextLogisticsCompany || undefined);
  }, [detailOrder?.id, detailOrder?.logistics_company, detailOrder?.ship_warehouse_id]);

  const orderTagDefinitionsQuery = useQuery({
    queryKey: ['order-tag-definitions', tenantId, 'order-exceptions'],
    queryFn: () =>
      listOrderTagDefinitions(tenantId, {
        status: 'ACTIVE',
        offset: 0,
        limit: 200,
      }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'order-exceptions'],
    queryFn: () => listWarehouses(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const courierQuery = useQuery({
    queryKey: ['couriers', tenantId, 'order-exceptions'],
    queryFn: () => listCouriers(tenantId, { limit: 200, status: 'ACTIVE' }),
    enabled: Boolean(tenantId),
    staleTime: 60_000,
  });

  const mergedWarehouseOptions = useMemo<DetailSelectOption>(() => {
    const map = new Map<string, { label: string; value: string }>();
    (warehouseQuery.data?.items ?? []).forEach((warehouse) => {
      const value = String(warehouse.warehouse_id ?? '').trim();
      if (!value) return;
      map.set(value, {
        label: String(warehouse.name ?? value),
        value,
      });
    });
    if (detailWarehouseId && !map.has(detailWarehouseId)) {
      map.set(detailWarehouseId, { label: detailWarehouseId, value: detailWarehouseId });
    }
    return Array.from(map.values());
  }, [detailWarehouseId, warehouseQuery.data?.items]);

  const mergedCourierOptions = useMemo<DetailSelectOption>(() => {
    const map = new Map<string, { label: string; value: string }>();
    (courierQuery.data?.items ?? []).forEach((courier) => {
      const value = String(courier.name ?? '').trim();
      if (!value) return;
      const code = String(courier.code ?? '').trim();
      const optionValue = resolveCourierOptionValue(value, code);
      if (!optionValue) return;
      map.set(optionValue, {
        label: buildCourierOptionLabel(value, code),
        value: optionValue,
      });
    });
    if (detailLogisticsCompany && !map.has(detailLogisticsCompany)) {
      map.set(detailLogisticsCompany, { label: detailLogisticsCompany, value: detailLogisticsCompany });
    }
    return Array.from(map.values());
  }, [courierQuery.data?.items, detailLogisticsCompany]);

  const orderTagOptions = useMemo<DetailSelectOption>(
    () =>
      (orderTagDefinitionsQuery.data?.items ?? []).map((tag) => ({
        label: tag.name,
        value: tag.name,
      })),
    [orderTagDefinitionsQuery.data?.items],
  );

  const currentOrderTags = useMemo(
    () =>
      (detailPackage?.tags ?? [])
        .map((item) => String(item.tag ?? '').trim())
        .filter(Boolean),
    [detailPackage?.tags],
  );

  const normalizedOrderTags = useMemo(
    () => Array.from(new Set(currentOrderTags.map((item) => item.trim()).filter(Boolean))),
    [currentOrderTags],
  );

  const tagMenuItems = useMemo<NonNullable<MenuProps['items']>>(() => {
    const items = orderTagOptions
      .map((option) => {
        const key = String(option?.value ?? '').trim();
        if (!key) return null;
        return {
          key,
          label: String(option?.label ?? key).trim() || key,
        };
      })
      .filter((item): item is { key: string; label: string } => item != null);
    if (items.length > 0) {
      return items;
    }
    return [{ key: '__empty__', label: '暂无订单记号', disabled: true }];
  }, [orderTagOptions]);

  const detailSelectDisabled = !detailOrderId;

  const refreshExceptions = useCallback(async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['order-exceptions', tenantId] }),
      queryClient.invalidateQueries({ queryKey: ['order-exception-detail', tenantId] }),
      queryClient.invalidateQueries({ queryKey: ['orders', tenantId] }),
    ]);
  }, [queryClient, tenantId]);

  const handleDetailWarehouseChange = useCallback(
    async (nextValue: string) => {
      const nextWarehouseId = String(nextValue ?? '').trim() || undefined;
      if (nextWarehouseId === detailWarehouseId) {
        return;
      }

      if (!tenantId || !detailOrderId) {
        setDetailWarehouseId(nextWarehouseId);
        return;
      }

      const previousWarehouseId = detailWarehouseId;
      setDetailWarehouseId(nextWarehouseId);
      setDetailUpdatingField('warehouse');
      try {
        await updateOrder(tenantId, detailOrderId, { ship_warehouse_id: nextWarehouseId });
        message.success('发货仓库已更新');
        await Promise.all([refreshExceptions(), detailQuery.refetch()]);
      } catch (error) {
        setDetailWarehouseId(previousWarehouseId);
        message.error(error instanceof Error ? error.message : '更新发货仓库失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [detailOrderId, detailQuery, detailWarehouseId, refreshExceptions, tenantId],
  );

  const handleDetailLogisticsChange = useCallback(
    async (nextValue: string) => {
      const normalized = String(nextValue ?? '').trim();
      const nextLogistics = normalized || undefined;
      if (nextLogistics === detailLogisticsCompany) {
        return;
      }

      if (!tenantId || !detailOrderId) {
        setDetailLogisticsCompany(nextLogistics);
        return;
      }

      const previousLogistics = detailLogisticsCompany;
      setDetailLogisticsCompany(nextLogistics);
      setDetailUpdatingField('logistics');
      try {
        await updateOrder(tenantId, detailOrderId, { logistics_company: nextLogistics });
        message.success('物流公司已更新');
        await Promise.all([refreshExceptions(), detailQuery.refetch()]);
      } catch (error) {
        setDetailLogisticsCompany(previousLogistics);
        message.error(error instanceof Error ? error.message : '更新物流公司失败');
      } finally {
        setDetailUpdatingField(null);
      }
    },
    [detailLogisticsCompany, detailOrderId, detailQuery, refreshExceptions, tenantId],
  );

  const handleApplyOrderTags = useCallback(
    async (tags: string[]) => {
      const normalizedTags = Array.from(new Set(tags.map((item) => item.trim()).filter(Boolean)));
      if (!normalizedTags.length) {
        message.warning('请至少选择一个订单记号');
        return false;
      }

      if (!tenantId || !detailOrderId) {
        message.warning('请先选中订单');
        return false;
      }

      setOrderTagsApplying(true);
      try {
        await addOrderTags(tenantId, detailOrderId, normalizedTags);
        message.success('订单记号已应用');
        await Promise.all([refreshExceptions(), detailQuery.refetch()]);
        return true;
      } catch (error) {
        message.error(error instanceof Error ? error.message : '应用订单记号失败');
        return false;
      } finally {
        setOrderTagsApplying(false);
      }
    },
    [detailOrderId, detailQuery, refreshExceptions, tenantId],
  );

  const handleMarkSelect = useCallback(
    (tagValue: string) => {
      const normalized = String(tagValue ?? '').trim();
      if (!normalized || detailSelectDisabled || orderTagsApplying) {
        return;
      }
      void handleApplyOrderTags([...normalizedOrderTags, normalized]);
    },
    [detailSelectDisabled, handleApplyOrderTags, normalizedOrderTags, orderTagsApplying],
  );

  const persistGoodsRows = useCallback(
    async (nextRows: GoodsGridRow[], successText: string) => {
      const previousRows = goodsRowsState;
      setGoodsRowsState(nextRows);

      if (!tenantId || !detailOrderId) {
        message.success(successText);
        return;
      }

      const payloadItems = nextRows.map((row, index) => ({
        line_id: row.id ?? `${detailOrderId}-LINE-${index + 1}`,
        sku_id: row.platform_sku_id ?? undefined,
        title: row.title ?? '',
        qty: Math.max(1, Number(row.quantity ?? 1)),
        price_cent: Math.max(0, Number(row.price_cent ?? 0)),
      }));

      setGoodsSaving(true);
      try {
        await updateOrder(tenantId, detailOrderId, { items: payloadItems });
        message.success(successText);
        await Promise.all([refreshExceptions(), detailQuery.refetch()]);
      } catch (error) {
        setGoodsRowsState(previousRows);
        message.error(error instanceof Error ? error.message : '货品操作失败');
      } finally {
        setGoodsSaving(false);
      }
    },
    [detailOrderId, detailQuery, goodsRowsState, refreshExceptions, tenantId],
  );

  const resolveOrderIds = useCallback(() => {
    const selectedIds = Array.from(new Set(selectedRowKeys.map((item) => String(item).trim()).filter(Boolean)));
    if (selectedIds.length > 0) {
      return selectedIds;
    }
    const activeId = String(activeOrderId ?? '').trim();
    return activeId ? [activeId] : [];
  }, [activeOrderId, selectedRowKeys]);

  const resolveMutation = useMutation({
    mutationFn: async (orderIds: string[]): Promise<FulfillmentPrecheckResult[]> => {
      await Promise.all(
        orderIds.map((orderId) =>
          resolveOrderException(tenantId, orderId, buildActionPayload(userId, userName, '订单已复核通过，准备递交WMS发货')),
        ),
      );
      return submitOrdersForFulfillment(tenantId, orderIds);
    },
    onSuccess: async (submitResults, orderIds) => {
      const failedItems = submitResults.filter((item) => !item.passed);
      const successCount = submitResults.length - failedItems.length;

      if (failedItems.length === 0) {
        message.success(`已完成 ${orderIds.length} 条订单复核，并已递交到WMS发货`);
      } else {
        message.warning(`已复核 ${orderIds.length} 条，成功递交WMS ${successCount} 条，失败 ${failedItems.length} 条`);
        const failedSample = failedItems[0];
        const failedReason = failedSample?.reasons?.[0] ?? '请检查复核前置条件';
        message.info(`递交失败示例：${failedSample?.order_no ?? failedSample?.order_id ?? '-'}，原因：${failedReason}`);
      }

      const failedOrderIdSet = new Set(failedItems.map((item) => item.order_id));
      const failedOrderIds = orderIds.filter((orderId) => failedOrderIdSet.has(orderId));
      setSelectedRowKeys(failedOrderIds);
      setActiveOrderId(failedOrderIds[0] ?? null);
      setFocusedOrderKey(failedOrderIds[0] ?? null);
      await refreshExceptions();
    },
    onError: () => {
      message.error('复核或递交WMS失败');
    },
  });

  const handleExportGrid = useCallback(() => {
    const api = mainGridApiRef.current;
    if (!api) {
      message.warning('表格尚未准备完成');
      return;
    }

    try {
      exportOrderReviewGridAsExcel(api as never, '订单复核');
    } catch (error) {
      if (error instanceof Error && error.message === 'NO_EXPORTABLE_COLUMNS') {
        message.warning('当前没有可导出的列');
        return;
      }
      message.error(error instanceof Error ? error.message : '导出失败');
    }
  }, []);

  const handlePrintGrid = useCallback(() => {
    const api = mainGridApiRef.current;
    if (!api) {
      message.warning('表格尚未准备完成');
      return;
    }

    try {
      printOrderReviewGrid(api as never, '订单复核');
    } catch (error) {
      if (error instanceof Error) {
        if (error.message === 'NO_PRINTABLE_COLUMNS') {
          message.warning('当前没有可打印的列');
          return;
        }
        if (error.message === 'NO_PRINTABLE_ROWS') {
          message.warning('当前没有可打印的数据');
          return;
        }
        if (error.message === 'PRINT_WINDOW_BLOCKED') {
          message.error('打印窗口被浏览器拦截，请允许弹窗后重试');
          return;
        }
      }
      message.error(error instanceof Error ? error.message : '打印失败');
    }
  }, []);

  const handleMainGridApiChange = useCallback((api: GridApi<OrderExceptionSummary> | null) => {
    mainGridApiRef.current = api;
  }, []);

  const handleMainContextMenu = useCallback(
    (record: OrderExceptionSummary, event: MouseEvent) => {
      const rowKey = String(record.order_id ?? '').trim() || null;
      setActiveOrderId(rowKey);
      setFocusedOrderKey(rowKey);
      if (rowKey && !selectedRowKeys.some((item) => String(item) === rowKey)) {
        setSelectedRowKeys([rowKey]);
      }
      setMainContextMenu({
        open: true,
        position: { x: event.clientX, y: event.clientY },
        row: record,
      });
    },
    [selectedRowKeys],
  );

  const mainContextMenuConfig = useMemo<MenuProps>(
    () => ({
      items: [
        { key: 'copy-order-no', label: '复制订单号' },
        { key: 'focus', label: '定位详情' },
        { key: 'clear-selection', label: '清空勾选' },
      ],
      onClick: ({ key }) => {
        const current = mainContextMenu.row;
        if (key === 'copy-order-no' && current) {
          const orderNo = current.order_no || current.order_id;
          if (orderNo && navigator?.clipboard) {
            void navigator.clipboard.writeText(orderNo);
          }
          message.success(orderNo ? `已复制：${orderNo}` : '无可复制订单号');
        }
        if (key === 'focus' && current) {
          const rowKey = resolveOrderExceptionRowKey(current) || null;
          setActiveOrderId(rowKey);
          setFocusedOrderKey(rowKey);
        }
        if (key === 'clear-selection') {
          setSelectedRowKeys([]);
        }
        setMainContextMenu({ open: false, position: null, row: null });
      },
    }),
    [mainContextMenu.row],
  );

  const rightPanel = useMemo(
    () => (
      <div className="order-exceptions-detail-panel" data-testid="order-exceptions-detail-panel">
        {activeRow ? (
          <UiModule05OrderReviewRightPanelTemplate
            detailError={detailQuery.error}
            detailLoadedId={detailLoadedId}
            orderId={detailOrderId || resolveOrderExceptionRowKey(activeRow)}
            platformOrderId={String(detailOrder?.platform_order_id ?? '') || undefined}
            platformText={String(detailOrder?.platform ?? activeRow.platform ?? '') || undefined}
            shopId={String(detailOrder?.shop_id ?? activeRow.shop_id ?? '') || undefined}
            receiverName={String(detailOrder?.receiver_name ?? activeRow.receiver_name ?? '') || undefined}
            receiverPhone={String(detailOrder?.receiver_phone ?? '') || undefined}
            logisticsCompany={detailLogisticsCompany}
            warehouseId={detailWarehouseId}
            totalAmountText={formatCurrency(Number(detailOrder?.total_amount_cent ?? activeRow.total_amount_cent ?? 0))}
            courierOptions={mergedCourierOptions}
            warehouseOptions={mergedWarehouseOptions}
            logisticsUpdating={detailUpdatingField === 'logistics'}
            warehouseUpdating={detailUpdatingField === 'warehouse'}
            detailSelectDisabled={detailSelectDisabled}
            orderTagsApplying={orderTagsApplying}
            markTags={normalizedOrderTags}
            markMenuItems={tagMenuItems}
            onMarkSelect={handleMarkSelect}
            onLogisticsChange={handleDetailLogisticsChange}
            onWarehouseChange={handleDetailWarehouseChange}
          />
        ) : null}
        {renderDetailContent(activeRow, detailQuery.data, detailQuery.isLoading)}
      </div>
    ),
    [
      activeRow,
      detailLoadedId,
      detailLogisticsCompany,
      detailOrder,
      detailOrderId,
      detailQuery.data,
      detailQuery.error,
      detailQuery.isLoading,
      detailSelectDisabled,
      detailUpdatingField,
      detailWarehouseId,
      handleDetailLogisticsChange,
      handleDetailWarehouseChange,
      handleMarkSelect,
      mergedCourierOptions,
      mergedWarehouseOptions,
      normalizedOrderTags,
      orderTagsApplying,
      tagMenuItems,
    ],
  );

  const bottomPanel = useMemo(
    () => (
      <OrderReviewBottomPanel
        activeOrderId={detailOrderId || 'none'}
        goodsRows={goodsRowsState}
        logRows={logRows}
        goodsSaving={goodsSaving}
        onPersistGoodsRows={persistGoodsRows}
      />
    ),
    [detailOrderId, goodsRowsState, goodsSaving, logRows, persistGoodsRows],
  );

  useEffect(() => {
    setRightPanelVisible(true);
    return () => {
      setRightPanel(undefined);
      setRightPanelVisible(false);
    };
  }, [setRightPanel, setRightPanelVisible]);

  useEffect(() => {
    setRightPanel(rightPanel);
    setRightPanelVisible(true);
  }, [rightPanel, setRightPanel, setRightPanelVisible]);

  useEffect(() => {
    setBottomPanelVisible(true);
    return () => {
      setBottomPanel(undefined);
      setBottomPanelVisible(false);
    };
  }, [setBottomPanel, setBottomPanelVisible]);

  useEffect(() => {
    setBottomPanel(bottomPanel);
    setBottomPanelVisible(true);
  }, [bottomPanel, setBottomPanel, setBottomPanelVisible]);

  const toolbar = (
    <WorkbenchToolbarTemplate
      className="order-exceptions-toolbar"
      leftContent={
        <div className="order-exceptions-toolbar-left">
          <Button
            data-testid="order-exceptions-resolve-action"
            type="primary"
            loading={resolveMutation.isPending}
            onClick={() => {
              const orderIds = resolveOrderIds();
              if (orderIds.length === 0) {
                message.warning('请先选中或定位一条订单');
                return;
              }
              resolveMutation.mutate(orderIds);
            }}
          >
            复核
          </Button>
          <Button
            icon={<ReloadOutlined />}
            loading={exceptionQuery.isFetching && !exceptionQuery.isLoading}
            onClick={() => {
              void refreshExceptions();
            }}
          >
            刷新
          </Button>
          <Button size="small" onClick={() => setQuickFilterExpanded((prev) => !prev)}>
            {quickFilterExpanded ? '收起列筛选' : '展开列筛选'}
          </Button>
        </div>
      }
      searchValue={keywordInput}
      onSearchChange={setKeywordInput}
      onSearch={() => {
        setSearchKeyword(keywordInput.trim());
        resetPagination();
      }}
      searchPlaceholder="搜索订单号/收件人/阻断原因"
      searchInputTestId="order-exceptions-search-input"
      searchButtonTestId="order-exceptions-search-trigger"
      searchButtonAriaLabel="查询"
      onExport={handleExportGrid}
      onPrint={handlePrintGrid}
      onHelp={() => message.info('帮助功能待接入')}
    />
  );

  const footerSummary = `总数 ${displayRows.length} 条，本页 ${pagedRows.length} 条，已选 ${selectedRowKeys.length} 条`;

  const mainContent = (
    <WorkbenchMainTableTemplate
      className="order-exceptions-page"
      toolbar={toolbar}
      tableContainerRef={tableRef}
      tableClassName="order-exceptions-grid-panel order-review-table order-workbench-table-module-03"
      tableContent={
        <div className="order-exceptions-main-stack">
          <div className="order-exceptions-grid-main order-review-grid-main">
            <OrderExceptionsMainGrid
              rows={pagedRows}
              quickFilterExpanded={quickFilterExpanded}
              selectedRowKeys={selectedRowKeys}
              focusedOrderKey={focusedOrderKey}
              onGridApiChange={handleMainGridApiChange}
              onSelectionChange={setSelectedRowKeys}
              onRowActive={(record) => {
                setActiveOrderId(resolveOrderExceptionRowKey(record) || null);
              }}
              onRowDoubleClick={(record) => {
                const orderId = String(record.order_id ?? '').trim();
                if (!orderId) {
                  message.warning('订单ID为空，无法打开编辑页');
                  return;
                }
                navigate(`/orders/${encodeURIComponent(orderId)}`);
              }}
              onFocus={(record) => {
                setFocusedOrderKey(resolveOrderExceptionRowKey(record) || null);
              }}
              onContextMenu={handleMainContextMenu}
              resolveAuditLabel={resolveAuditLabel}
              resolveExceptionLabel={resolveExceptionLabel}
              loading={mainGridLoading}
              error={exceptionQuery.error}
              onRetry={() => {
                void exceptionQuery.refetch();
              }}
            />
          </div>
        </div>
      }
      footerSyncKey={`order-exceptions-${displayRows.length}-${resolvedCurrentPage}-${resolvedPageSize}`}
      footerSummary={footerSummary}
      total={displayRows.length}
      pagination={pagination}
      onPaginationChange={handlePaginationChange}
      showFooterScrollbarRow={false}
    />
  );
  return (
    <div
      data-testid="order-exceptions-page"
      style={{
        display: 'flex',
        flexDirection: 'column',
        flex: 1,
        minHeight: 0,
        width: '100%',
        maxWidth: '100%',
        height: '100%',
      }}
    >
      {mainContent}
      <UiModule03ContextMenu
        open={mainContextMenu.open}
        position={mainContextMenu.position}
        menu={mainContextMenuConfig}
        onClose={() => setMainContextMenu({ open: false, position: null, row: null })}
      />
    </div>
  );
};

