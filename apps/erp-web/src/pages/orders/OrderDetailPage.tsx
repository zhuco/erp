import {
  Alert,
  Button,
  Card,
  Checkbox,
  Descriptions,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Steps,
  Table,
  Tag,
  Tooltip,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useCallback, useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { OrderDetail } from '@/api/orders';
import {
  addOrderNote,
  addOrderTags,
  getOrderDetail,
  handleOrderException,
  reviewOrder
} from '@/api/orders';
import { listOperationLogs, type OperationLog } from '@/api/system';
import type { OmsShipmentSummary } from '@/api/omsShipments';
import {
  createOmsIdempotencyKey,
  listOmsOrderShipments,
  releaseOmsOrder,
  resendOmsOrder
} from '@/api/omsShipments';
import { OrderTagBadges } from '@/components/OrderTagBadges';
import { OperationButton } from '@/components/OperationButton';
import { PlatformTag } from '@/components/PlatformTag';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { ShipmentStatusTag } from '@/components/ShipmentStatusTag';
import { UiModule07SettingsLayout } from '@/components/ui-modules';
import { createPrintJobAndPrint } from '@/features/printing/printJobFlow';
import { ShipOrderDrawer } from '@/pages/orders/ShipOrderDrawer';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { resolveOperationActionLabel } from '@/utils/logLabels';

type OrderItem = OrderDetail['items'][number];
type AuditLog = OrderDetail['audit_logs'][number];
type ReviewHit = OrderDetail['review_hits'][number];
type OrderNote = OrderDetail['notes'][number];
type OrderTag = OrderDetail['tags'][number];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : `¥${(value / 100).toFixed(2)}`;

const orderStatusSteps = [
  { value: 'NEW', title: '新建' },
  { value: 'REVIEW_PENDING', title: '待复核' },
  { value: 'PENDING_SHIPMENT', title: '待发货' },
  { value: 'ALLOCATED', title: '已分配' },
  { value: 'PICKING', title: '拣货' },
  { value: 'PACKED', title: '已打包' },
  { value: 'SHIPPED', title: '已发货' },
  { value: 'DONE', title: '已完成' },
  { value: 'CANCELLED', title: '已取消' }
];

const orderStatusMeta: Record<string, { label: string; color: string }> = {
  NEW: { label: '新建', color: 'blue' },
  REVIEW_PENDING: { label: '待复核', color: 'orange' },
  PENDING_SHIPMENT: { label: '待发货', color: 'cyan' },
  ALLOCATED: { label: '已分配', color: 'geekblue' },
  PICKING: { label: '拣货中', color: 'purple' },
  PACKED: { label: '已打包', color: 'gold' },
  SHIPPED: { label: '已发货', color: 'green' },
  DONE: { label: '已完成', color: 'default' },
  CANCELLED: { label: '已取消', color: 'default' }
};

const reviewStatusMeta: Record<string, { label: string; color: string }> = {
  PENDING: { label: '待复核', color: 'gold' },
  PASSED: { label: '已通过', color: 'green' },
  REJECTED: { label: '已拒绝', color: 'red' }
};

const exceptionFlagLabels: Record<string, string> = {
  MAPPING_MISSING: '映射缺失',
  INVENTORY_SHORTAGE: '库存不足',
  ADDRESS_INVALID: '地址异常',
  DUPLICATE_ORDER: '重复订单',
  BLACKLISTED_CUSTOMER: '黑名单客户',
  REVIEW_REJECTED: '审核拒绝',
  REVIEW_FLAGGED: '审核拦截',
  SHIPPING_FAILED: '发货失败',
  ORDER_FROZEN: '冻结订单',
  ORDER_MERGED: '合并订单',
  ORDER_SPLIT: '拆分订单'
};

const reviewActionLabels: Record<string, string> = {
  PASS: '通过',
  REJECT: '拒绝',
  FLAG: '拦截'
};

const reviewActionColors: Record<string, string> = {
  PASS: 'green',
  REJECT: 'red',
  FLAG: 'orange'
};

export const OrderDetailPage = () => {
  const { order_id } = useParams();
  const navigate = useNavigate();
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const canWriteOrders = useCan('orders.write');
  const canWriteInventory = useCan('inventory.write');
  const canWriteShipping = useCan('shipping.write');
  const canReadOperationLogs = useCan('operation_logs.read');
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [printing, setPrinting] = useState(false);
  const [reviewSubmitting, setReviewSubmitting] = useState(false);
  const [reviewAction, setReviewAction] = useState<'REJECT' | 'FLAG' | null>(null);
  const [reviewReason, setReviewReason] = useState('');
  const [tagInput, setTagInput] = useState('');
  const [noteInput, setNoteInput] = useState('');
  const [tagSubmitting, setTagSubmitting] = useState(false);
  const [noteSubmitting, setNoteSubmitting] = useState(false);
  const [exceptionModalOpen, setExceptionModalOpen] = useState(false);
  const [exceptionSubmitting, setExceptionSubmitting] = useState(false);
  const [exceptionAction, setExceptionAction] = useState<'RESOLVE' | 'REJECT' | 'REVOKE'>(
    'RESOLVE'
  );
  const [exceptionReason, setExceptionReason] = useState('');
  const [exceptionSelections, setExceptionSelections] = useState<string[]>([]);
  const [resendOpen, setResendOpen] = useState(false);
  const [resendSubmitting, setResendSubmitting] = useState(false);
  const [resendCarrierCode, setResendCarrierCode] = useState('');
  const [resendOriginShipmentId, setResendOriginShipmentId] = useState('');
  const [resendIdempotencyKey, setResendIdempotencyKey] = useState(createOmsIdempotencyKey());
  const [releaseSubmitting, setReleaseSubmitting] = useState(false);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['order-detail', tenantId, order_id],
    queryFn: () => getOrderDetail(tenantId, order_id ?? ''),
    enabled: Boolean(order_id)
  });
  const detailLoading = (isLoading || isFetching) && !data;

  const {
    data: omsShipments,
    isFetching: omsShipmentsFetching,
    error: omsShipmentsError
  } = useQuery({
    queryKey: ['order-oms-shipments', tenantId, order_id],
    queryFn: () => listOmsOrderShipments(tenantId, order_id ?? ''),
    enabled: Boolean(order_id)
  });

  const operationLogsQuery = useQuery({
    queryKey: ['order-operation-logs', tenantId, order_id],
    queryFn: () =>
      listOperationLogs(tenantId, {
        ref_type: 'ORDER',
        ref_id: order_id,
        limit: 20
      }),
    enabled: Boolean(order_id) && canReadOperationLogs
  });

  const order = data?.order;
  const canPrintLabel = order?.oms_status === 'SHIPPED';
  const canRelease =
    order && !['SHIPPED', 'DONE', 'CANCELLED'].includes(order.oms_status);
  const releaseDisabledReason = !canWriteInventory
    ? '暂无库存权限'
    : !canRelease
      ? '当前状态不可释放'
      : undefined;
  const resendDisabledReason =
    !canWriteShipping || !canWriteInventory ? '暂无补发权限' : undefined;
  const reviewDisabled =
    !order ||
    ['ALLOCATED', 'PICKING', 'PACKED', 'SHIPPED', 'DONE', 'CANCELLED'].includes(
      order.oms_status
    );
  const reviewDisabledReason = !canWriteOrders
    ? '暂无审核权限'
    : reviewDisabled
      ? '当前状态不可审核'
      : undefined;
  const exceptionDisabledReason = !canWriteOrders ? '暂无异常处理权限' : undefined;

  const exceptionFlagOptions = useMemo(
    () =>
      Object.entries(exceptionFlagLabels).map(([value, label]) => ({
        label,
        value
      })),
    []
  );

  const handlePrintLabel = async () => {
    if (!order) {
      return;
    }
    setPrinting(true);
    try {
      const { job } = await createPrintJobAndPrint(tenantId, {
        order_id: order.id,
        platform: order.platform,
        shop_id: order.shop_id,
        tracking_no: order.tracking_no ?? null,
        carrier_code: order.logistics_company ?? null
      });
      if (job.status === 'DONE') {
        message.success('打印任务已完成');
      } else if (job.status === 'FAILED') {
        message.error(job.last_error || '打印失败');
      } else {
        message.success('打印任务已提交');
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '创建打印任务失败');
    } finally {
      setPrinting(false);
    }
  };

  const openResendModal = () => {
    if (!order) {
      return;
    }
    setResendCarrierCode('');
    setResendOriginShipmentId('');
    setResendIdempotencyKey(createOmsIdempotencyKey());
    setResendOpen(true);
  };

  const handleResend = async () => {
    if (!order) {
      return;
    }
    setResendSubmitting(true);
    try {
      await resendOmsOrder(tenantId, order.id, {
        carrier_code: resendCarrierCode.trim() || undefined,
        origin_shipment_id: resendOriginShipmentId.trim() || undefined,
        idempotency_key: resendIdempotencyKey
      });
      message.success('补发包裹已创建');
      setResendOpen(false);
      setResendIdempotencyKey(createOmsIdempotencyKey());
      await refetch();
      await queryClient.invalidateQueries({
        queryKey: ['order-oms-shipments', tenantId, order_id]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '补发失败');
    } finally {
      setResendSubmitting(false);
    }
  };

  const confirmRelease = () => {
    if (!order) {
      return;
    }
    Modal.confirm({
      title: '释放库存占用',
      content: '将释放该订单库存占用并标记订单取消。',
      okText: '确认',
      cancelText: '取消',
      onOk: async () => {
        setReleaseSubmitting(true);
        try {
          await releaseOmsOrder(tenantId, order.id, createOmsIdempotencyKey());
          message.success('库存占用已释放');
          await refetch();
          await queryClient.invalidateQueries({
            queryKey: ['order-oms-shipments', tenantId, order_id]
          });
        } catch (err) {
          message.error(err instanceof Error ? err.message : '释放库存失败');
        } finally {
          setReleaseSubmitting(false);
        }
      }
    });
  };

  const submitReview = async (action: 'PASS' | 'REJECT' | 'FLAG', reason?: string) => {
    if (!order) {
      return;
    }
    setReviewSubmitting(true);
    try {
      await reviewOrder(tenantId, order.id, {
        action,
        reason: reason?.trim() || undefined,
        version: order.version
      });
      message.success('审核已更新');
      await refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '审核失败');
    } finally {
      setReviewSubmitting(false);
      setReviewAction(null);
      setReviewReason('');
    }
  };

  const handleReviewPass = () => {
    if (!order) {
      return;
    }
    submitReview('PASS');
  };

  const handleReviewReject = () => {
    setReviewAction('REJECT');
    setReviewReason('');
  };

  const handleReviewFlag = () => {
    setReviewAction('FLAG');
    setReviewReason('');
  };

  const handleOpenExceptionModal = () => {
    if (!order) {
      return;
    }
    setExceptionAction('RESOLVE');
    setExceptionReason('');
    setExceptionSelections(order.exception_flags ?? []);
    setExceptionModalOpen(true);
  };

  const handleExceptionSubmit = async () => {
    if (!order) {
      return;
    }
    setExceptionSubmitting(true);
    try {
      await handleOrderException(tenantId, order.id, {
        action: exceptionAction,
        reason: exceptionReason.trim() || undefined,
        flags:
          exceptionAction === 'REJECT' || exceptionSelections.length === 0
            ? undefined
            : exceptionSelections
      });
      message.success('异常处理已更新');
      setExceptionModalOpen(false);
      await refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '异常处理失败');
    } finally {
      setExceptionSubmitting(false);
    }
  };

  const handleAddTags = async () => {
    if (!order) {
      return;
    }
    const tags = tagInput
      .split(',')
      .map((tag) => tag.trim())
      .filter((tag) => tag.length > 0);
    if (!tags.length) {
      message.warning('请先输入标签');
      return;
    }
    setTagSubmitting(true);
    try {
      await addOrderTags(tenantId, order.id, tags);
      message.success('标签已添加');
      setTagInput('');
      await refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '添加标签失败');
    } finally {
      setTagSubmitting(false);
    }
  };

  const handleAddNote = async () => {
    if (!order) {
      return;
    }
    const note = noteInput.trim();
    if (!note) {
      message.warning('请输入备注内容');
      return;
    }
    setNoteSubmitting(true);
    try {
      await addOrderNote(tenantId, order.id, note);
      message.success('备注已添加');
      setNoteInput('');
      await refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '添加备注失败');
    } finally {
      setNoteSubmitting(false);
    }
  };

  const orderItemColumns = useMemo<ColumnsType<OrderItem>>(
    () => [
      {
        title: '商品',
        dataIndex: 'title',
        render: (_, record) => record.title ?? record.platform_sku_id ?? '-'
      },
      {
        title: '平台 SKU',
        dataIndex: 'platform_sku_id',
        render: (_, record) => record.platform_sku_id ?? '-'
      },
      {
        title: '外部 SKU',
        dataIndex: 'outer_sku_id',
        render: (_, record) => record.outer_sku_id ?? '-'
      },
      {
        title: '数量',
        dataIndex: 'quantity',
        align: 'right'
      },
      {
        title: '单价',
        dataIndex: 'price_cent',
        align: 'right',
        render: (_, record) => formatCurrency(record.price_cent)
      },
      {
        title: '映射状态',
        dataIndex: 'mapping_missing',
        render: (_, record) => {
          if (!record.mapping_missing) {
            return '正常';
          }
          return (
            <Space size={4}>
              <Tag color="red">缺失</Tag>
            </Space>
          );
        }
      }
    ],
    []
  );

  const reviewHitColumns = useMemo<ColumnsType<ReviewHit>>(
    () => [
      {
        title: '规则名称',
        dataIndex: 'rule_name'
      },
      {
        title: '动作',
        dataIndex: 'action',
        render: (_, record) => {
          const action = record.action?.toUpperCase?.() ?? record.action;
          return (
            <Tag color={reviewActionColors[action] ?? 'default'}>
              {reviewActionLabels[action] ?? record.action}
            </Tag>
          );
        }
      },
      {
        title: '命中说明',
        dataIndex: 'message',
        render: (_, record) => record.message ?? '-'
      },
      {
        title: '命中时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      }
    ],
    []
  );

  const tagColumns = useMemo<ColumnsType<OrderTag>>(
    () => [
      {
        title: '标签',
        dataIndex: 'tag',
        render: (_, record) => <OrderTagBadges tags={[record.tag]} />
      },
      {
        title: '创建人',
        dataIndex: 'created_by',
        render: (_, record) => (record.created_by ? '人工' : '系统')
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      }
    ],
    []
  );

  const noteColumns = useMemo<ColumnsType<OrderNote>>(
    () => [
      { title: '备注内容', dataIndex: 'note' },
      {
        title: '创建人',
        dataIndex: 'created_by',
        render: (_, record) => record.created_by ?? '-'
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      }
    ],
    []
  );

  const auditColumns = useMemo<ColumnsType<AuditLog>>(
    () => [
      {
        title: '动作',
        dataIndex: 'action',
        render: (value) => resolveOperationActionLabel(value)
      },
      {
        title: '操作者',
        dataIndex: 'user_id',
        render: (_, record) => record.user_id ?? '-'
      },
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      },
      {
        title: '变更',
        dataIndex: 'detail',
        render: (_, record) => {
          const before = record.before_json ? JSON.stringify(record.before_json) : '-';
          const after = record.after_json ? JSON.stringify(record.after_json) : '-';
          return (
            <Tooltip title={`before: ${before}\nafter: ${after}`}>
              <Typography.Text ellipsis>
                {before === '-' && after === '-' ? '-' : `${before} -> ${after}`}
              </Typography.Text>
            </Tooltip>
          );
        }
      }
    ],
    []
  );

  const operationColumns = useMemo<ColumnsType<OperationLog>>(
    () => [
      {
        title: '动作',
        dataIndex: 'action',
        render: (value) => resolveOperationActionLabel(value)
      },
      {
        title: '操作者',
        dataIndex: 'actor_user_id',
        render: (_, record) =>
          record.actor_type === 'SYSTEM' ? '系统' : record.actor_user_id ?? '-'
      },
      {
        title: '说明',
        dataIndex: 'metadata',
        render: (_, record) => {
          const metadata = record.metadata as { reason?: string } | undefined;
          return metadata?.reason ?? JSON.stringify(record.metadata ?? {});
        }
      },
      {
        title: '时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      }
    ],
    []
  );

  const shipmentColumns = useMemo<ColumnsType<OmsShipmentSummary>>(
    () => [
      { title: '包裹单号', dataIndex: 'shipment_id' },
      {
        title: '类型',
        dataIndex: 'shipment_type',
        render: (_, record) => record.shipment_type ?? '-'
      },
      {
        title: '原包裹',
        dataIndex: 'origin_shipment_id',
        render: (_, record) => record.origin_shipment_id ?? '-'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => <ShipmentStatusTag status={record.status} />
      },
      { title: '仓库', dataIndex: 'warehouse_id' },
      { title: '承运商', dataIndex: 'carrier_code', render: (_, record) => record.carrier_code ?? '-' },
      { title: '运单号', dataIndex: 'tracking_no', render: (_, record) => record.tracking_no ?? '-' },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      }
    ],
    []
  );

  if (!order_id) {
    return (
      <ErrorState
        title="缺少订单 ID"
        description="无法加载订单详情"
        hint="请返回订单列表重新选择订单。"
        action={<Button onClick={() => navigate('/orders')}>返回订单列表</Button>}
      />
    );
  }

  if (error) {
    return (
      <ErrorState title="订单详情加载失败" error={error} onRetry={() => refetch()} />
    );
  }

  const exceptionFlags = order?.exception_flags ?? [];
  const statusMeta = order ? orderStatusMeta[order.oms_status] : undefined;
  const reviewMeta = order ? reviewStatusMeta[order.review_status] : undefined;
  const currentStep = order
    ? Math.max(
        0,
        orderStatusSteps.findIndex((step) => step.value === order.oms_status)
      )
    : 0;
  const stepStatus = order?.oms_status === 'CANCELLED' ? 'error' : 'process';

  return (
    <UiModule07SettingsLayout>
      <Row align="middle" justify="space-between" gutter={[16, 16]}>
        <Space align="center" size={12} wrap>
          <Button onClick={() => navigate('/orders')}>返回列表</Button>
          <Typography.Title level={3} style={{ margin: 0 }}>
            订单详情
          </Typography.Title>
        </Space>
        <Space align="center" size={8} wrap>
        {order ? (
          <OperationButton
            type="primary"
            operation="orders.ship"
            platform={order.platform}
            onClick={() => setDrawerOpen(true)}
            data-testid="order-detail-ship"
          >
            发货
          </OperationButton>
        ) : null}
        {order ? (
          <Tooltip title={canPrintLabel ? undefined : '请先完成发货'}>
            <span>
              <Button
                type="primary"
                ghost
                disabled={!canPrintLabel}
                loading={printing}
                onClick={handlePrintLabel}
                data-testid="order-detail-print-label"
              >
                打印面单
              </Button>
            </span>
          </Tooltip>
        ) : null}
        {order ? (
          <Tooltip title={resendDisabledReason}>
            <span>
              <Button
                onClick={openResendModal}
                disabled={!canWriteShipping || !canWriteInventory}
              >
                补发
              </Button>
            </span>
          </Tooltip>
        ) : null}
        {order ? (
          <Tooltip title={releaseDisabledReason}>
            <span>
              <Button
                danger
                loading={releaseSubmitting}
                onClick={confirmRelease}
                disabled={!canRelease || !canWriteInventory}
              >
                释放占用
              </Button>
            </span>
          </Tooltip>
        ) : null}
        {order ? (
          <Tooltip title={reviewDisabledReason}>
            <span>
              <Button
                onClick={handleReviewPass}
                disabled={reviewDisabled || !canWriteOrders}
                loading={reviewSubmitting}
              >
                审核通过
              </Button>
            </span>
          </Tooltip>
        ) : null}
        {order ? (
          <Tooltip title={reviewDisabledReason}>
            <span>
              <Button
                danger
                onClick={handleReviewReject}
                disabled={reviewDisabled || !canWriteOrders}
                loading={reviewSubmitting}
              >
                审核拒绝
              </Button>
            </span>
          </Tooltip>
        ) : null}
        {order ? (
          <Tooltip title={reviewDisabledReason}>
            <span>
              <Button
                onClick={handleReviewFlag}
                disabled={reviewDisabled || !canWriteOrders}
                loading={reviewSubmitting}
              >
                审核拦截
              </Button>
            </span>
          </Tooltip>
        ) : null}
        {order ? (
          <Tooltip title={exceptionDisabledReason}>
            <span>
              <Button onClick={handleOpenExceptionModal} disabled={!canWriteOrders}>
                异常处理
              </Button>
            </span>
          </Tooltip>
        ) : null}
        </Space>
      </Row>
      <LoadingState loading={detailLoading} onRetry={() => refetch()} />
      {order ? (
        <>
          <Card>
            <Space direction="vertical" size={12} style={{ width: '100%' }}>
              <Steps
                size="small"
                current={currentStep}
                status={stepStatus}
                items={orderStatusSteps.map((step) => ({ title: step.title }))}
              />
              <Descriptions column={3} bordered size="small">
                <Descriptions.Item label="订单号">{order.id}</Descriptions.Item>
                <Descriptions.Item label="平台订单号">
                  {order.platform_order_id ?? '-'}
                </Descriptions.Item>
                <Descriptions.Item label="平台">
                  <PlatformTag platform={order.platform} />
                </Descriptions.Item>
                <Descriptions.Item label="店铺">{order.shop_id}</Descriptions.Item>
                <Descriptions.Item label="平台状态">
                  <Tag>{order.platform_status}</Tag>
                </Descriptions.Item>
                <Descriptions.Item label="订单状态">
                  <Tag color={statusMeta?.color ?? 'default'}>
                    {statusMeta?.label ?? order.oms_status}
                  </Tag>
                </Descriptions.Item>
                <Descriptions.Item label="审核状态">
                  <Tag color={reviewMeta?.color ?? 'default'}>
                    {reviewMeta?.label ?? order.review_status}
                  </Tag>
                </Descriptions.Item>
                <Descriptions.Item label="异常标记">
                  {exceptionFlags.length ? (
                    <Space size={[4, 4]} wrap>
                      {exceptionFlags.map((flag) => (
                        <Tag key={flag} color="volcano">
                          {exceptionFlagLabels[flag] ?? flag}
                        </Tag>
                      ))}
                    </Space>
                  ) : (
                    <Tag>无</Tag>
                  )}
                </Descriptions.Item>
                <Descriptions.Item label="下单时间">
                  {formatDateTime(order.order_created_at)}
                </Descriptions.Item>
                <Descriptions.Item label="更新时间">
                  {formatDateTime(order.updated_at ?? order.order_created_at)}
                </Descriptions.Item>
              </Descriptions>
            </Space>
          </Card>
          <Card>
            <Descriptions column={3} bordered size="small">
              <Descriptions.Item label="收件人">
                {order.receiver_name ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="客户">
                {order.customer_id ? (
                  <Space>
                    <Typography.Link
                      onClick={() => navigate(`/customers/${order.customer_id}`)}
                    >
                      {order.customer_name ?? order.receiver_name ?? order.customer_id}
                    </Typography.Link>
                    {order.customer_status ? (
                      <Tag color={order.customer_status === 'INACTIVE' ? 'red' : 'green'}>
                        {order.customer_status === 'INACTIVE' ? '停用' : '启用'}
                      </Tag>
                    ) : null}
                  </Space>
                ) : (
                  order.customer_name ?? '-'
                )}
              </Descriptions.Item>
              <Descriptions.Item label="联系电话">
                {order.receiver_phone ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="收货地址">
                {order.receiver_address ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="商品金额">
                {formatCurrency(order.goods_amount_cent)}
              </Descriptions.Item>
              <Descriptions.Item label="运费">
                {formatCurrency(order.postage_amount_cent)}
              </Descriptions.Item>
              <Descriptions.Item label="优惠">
                {formatCurrency(order.discount_amount_cent)}
              </Descriptions.Item>
              <Descriptions.Item label="应付金额">
                {formatCurrency(order.total_amount_cent)}
              </Descriptions.Item>
              <Descriptions.Item label="物流公司">
                {order.logistics_company ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="运单号">
                {order.tracking_no ?? '-'}
              </Descriptions.Item>
            </Descriptions>
          </Card>
        </>
      ) : null}
      <Card title="订单明细" data-testid="order-items-card">
        <Table<OrderItem>
          rowKey={(record) => record.id}
          columns={orderItemColumns}
          dataSource={data?.items ?? []}
          pagination={false}
        />
      </Card>
      <Card title="审单命中">
        <Table<ReviewHit>
          rowKey={(record) => `${record.rule_id}-${record.created_at}`}
          columns={reviewHitColumns}
          dataSource={data?.review_hits ?? []}
          pagination={false}
          locale={{ emptyText: '暂无命中记录' }}
        />
      </Card>
      <Card title="标签">
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Table<OrderTag>
            rowKey={(record) => `${record.tag}-${record.created_at}`}
            columns={tagColumns}
            dataSource={data?.tags ?? []}
            pagination={false}
            locale={{ emptyText: '暂无标签' }}
          />
          <Space>
            <Input
              value={tagInput}
              onChange={(event) => setTagInput(event.target.value)}
              placeholder="输入标签，使用逗号分隔"
              style={{ width: 360 }}
            />
            <Button loading={tagSubmitting} onClick={handleAddTags} disabled={!canWriteOrders}>
              添加标签
            </Button>
          </Space>
        </Space>
      </Card>
      <Card title="备注">
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Table<OrderNote>
            rowKey={(record) => record.note_id}
            columns={noteColumns}
            dataSource={data?.notes ?? []}
            pagination={false}
            locale={{ emptyText: '暂无备注' }}
          />
          <Space direction="vertical" style={{ width: '100%' }}>
            <Input.TextArea
              value={noteInput}
              onChange={(event) => setNoteInput(event.target.value)}
              placeholder="输入备注"
              rows={3}
            />
            <Button
              type="primary"
              loading={noteSubmitting}
              onClick={handleAddNote}
              disabled={!canWriteOrders}
            >
              添加备注
            </Button>
          </Space>
        </Space>
      </Card>
      {canReadOperationLogs ? (
        <Card title="操作日志">
          {operationLogsQuery.error ? (
            <Alert
              type="error"
              showIcon
              message="操作日志加载失败"
              description={
                operationLogsQuery.error instanceof Error
                  ? operationLogsQuery.error.message
                  : null
              }
            />
          ) : (
            <Table<OperationLog>
              rowKey={(record) => record.log_id}
              columns={operationColumns}
              dataSource={operationLogsQuery.data?.data ?? []}
              pagination={false}
              loading={operationLogsQuery.isFetching}
              locale={{ emptyText: '暂无操作日志' }}
            />
          )}
        </Card>
      ) : null}
      <Card title="审计日志">
        <Table<AuditLog>
          rowKey={(record) => record.audit_id}
          columns={auditColumns}
          dataSource={data?.audit_logs ?? []}
          pagination={false}
          locale={{ emptyText: '暂无日志' }}
        />
      </Card>
      <Card title="OMS包裹">
        {omsShipmentsError ? (
          <Alert
            type="error"
            showIcon
            message="包裹信息加载失败"
            description={omsShipmentsError instanceof Error ? omsShipmentsError.message : null}
          />
        ) : (
          <Table<OmsShipmentSummary>
            rowKey={(record) => record.shipment_id}
            columns={shipmentColumns}
            dataSource={omsShipments?.items ?? []}
            loading={omsShipmentsFetching}
            pagination={false}
            locale={{ emptyText: '暂无包裹记录' }}
          />
        )}
      </Card>
      <ShipOrderDrawer
        open={drawerOpen}
        order={order ?? null}
        onClose={() => setDrawerOpen(false)}
      />
      <Modal
        title="补发包裹"
        open={resendOpen}
        okText="提交"
        cancelText="取消"
        confirmLoading={resendSubmitting}
        onOk={handleResend}
        onCancel={() => setResendOpen(false)}
        destroyOnClose
      >
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Input
            value={resendOriginShipmentId}
            onChange={(event) => setResendOriginShipmentId(event.target.value)}
            placeholder="原包裹ID（可选）"
          />
          <Input
            value={resendCarrierCode}
            onChange={(event) => setResendCarrierCode(event.target.value)}
            placeholder="承运商编码（可选）"
          />
        </Space>
      </Modal>
      <Modal
        title={reviewAction === 'REJECT' ? '审核拒绝' : '审核拦截'}
        open={reviewAction !== null}
        okText="提交"
        cancelText="取消"
        confirmLoading={reviewSubmitting}
        onOk={() => {
          if (reviewAction) {
            submitReview(reviewAction, reviewReason);
          }
        }}
        onCancel={() => setReviewAction(null)}
      >
        <Input.TextArea
          value={reviewReason}
          onChange={(event) => setReviewReason(event.target.value)}
          placeholder="请输入原因"
          rows={4}
        />
      </Modal>
      <Modal
        title="异常处理"
        open={exceptionModalOpen}
        okText="提交"
        cancelText="取消"
        confirmLoading={exceptionSubmitting}
        onOk={handleExceptionSubmit}
        onCancel={() => setExceptionModalOpen(false)}
      >
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Select
            value={exceptionAction}
            onChange={(value) => {
              setExceptionAction(value);
              if (value === 'RESOLVE') {
                setExceptionSelections(order?.exception_flags ?? []);
              } else if (value === 'REVOKE') {
                setExceptionSelections([]);
              } else {
                setExceptionSelections([]);
              }
            }}
            options={[
              { value: 'RESOLVE', label: '解除异常' },
              { value: 'REJECT', label: '驳回处理' },
              { value: 'REVOKE', label: '恢复异常' }
            ]}
          />
          {exceptionAction === 'REJECT' ? null : (
            <Checkbox.Group
              options={exceptionFlagOptions}
              value={exceptionSelections}
              onChange={(values) => setExceptionSelections(values as string[])}
            />
          )}
          <Input.TextArea
            value={exceptionReason}
            onChange={(event) => setExceptionReason(event.target.value)}
            placeholder="请输入原因（可选）"
            rows={3}
          />
        </Space>
      </Modal>
    </UiModule07SettingsLayout>
  );
};
