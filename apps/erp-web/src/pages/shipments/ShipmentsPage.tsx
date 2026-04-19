import {
  Alert,
  Button,
  Card,
  Col,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  List,
  Row,
  Select,
  Space,
  Tag,
  Tooltip,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  ShipmentListQuery,
  ShipmentPackage,
  ShipmentPushAttempt,
  ShipmentPushStatus,
  ShipmentSummary
} from '@/api/shipments';
import {
  confirmShipment,
  listShipmentPackages,
  listShipmentPushAttempts,
  listShipments,
  retryShipment
} from '@/api/shipments';
import type { LogisticsPriceEstimateResponse } from '@/api/logisticsPrices';
import { estimateLogisticsPrice } from '@/api/logisticsPrices';
import { ShipmentStatusTag } from '@/components/ShipmentStatusTag';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { PlatformTag } from '@/components/PlatformTag';
import { useFeatureFlag } from '@/features/featureFlags';
import { useOperationAccess } from '@/features/operations';
import type { PlatformCode } from '@/features/platform';
import { platformOptions } from '@/features/platform';
import { resolveShopId, useShopIds, useTenantId } from '@/features/tenant';

type FilterValues = {
  platform?: PlatformCode;
  shop_id?: string;
  status?: string;
  updated_range?: [Dayjs, Dayjs];
  platform_order_id?: string;
};

const statusOptions = [
  { label: 'REQUESTED', value: 'REQUESTED' },
  { label: 'SUCCESS', value: 'SUCCESS' },
  { label: 'FAILED', value: 'FAILED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatPrice = (value?: number | null) =>
  value == null ? '-' : `¥${(value / 100).toFixed(2)}`;

const pushStatusMeta: Record<ShipmentPushStatus, { color: string; label: string }> = {
  SUCCESS: { color: 'green', label: '成功' },
  FAILED: { color: 'red', label: '失败' },
  RETRYING: { color: 'orange', label: '重试中' },
  DEAD: { color: 'volcano', label: '已终止' }
};

const renderPushStatus = (status?: ShipmentPushStatus | null) => {
  if (!status) {
    return <Tag>-</Tag>;
  }
  const meta = pushStatusMeta[status];
  if (!meta) {
    return <Tag>{status}</Tag>;
  }
  return <Tag color={meta.color}>{meta.label}</Tag>;
};

type ShipmentActionsProps = {
  shipment: ShipmentSummary;
  onView: (shipment: ShipmentSummary) => void;
  onRetry: (shipment: ShipmentSummary) => void;
  onConfirm: (shipment: ShipmentSummary) => void;
  retryingId?: string | null;
  confirmingId?: string | null;
};

const ShipmentActions = ({
  shipment,
  onView,
  onRetry,
  onConfirm,
  retryingId,
  confirmingId
}: ShipmentActionsProps) => {
  const decision = useOperationAccess('orders.ship', shipment.platform);
  const canConfirm =
    shipment.status === 'SUCCESS' && shipment.push_status !== 'SUCCESS';
  return (
    <Space>
      <Button size="small" onClick={() => onView(shipment)} data-testid="shipment-view-button">
        查看
      </Button>
      {canConfirm ? (
        <Tooltip title={decision.allowed ? undefined : decision.reason}>
          <span>
            <Button
              size="small"
              disabled={!decision.allowed}
              loading={confirmingId === shipment.shipment_id}
              onClick={() => {
                if (decision.allowed) {
                  onConfirm(shipment);
                }
              }}
              data-testid="shipment-confirm-button"
            >
              回写
            </Button>
          </span>
        </Tooltip>
      ) : null}
      {shipment.status === 'FAILED' ? (
        <Tooltip title={decision.allowed ? undefined : decision.reason}>
          <span>
            <Button
              size="small"
              disabled={!decision.allowed}
              loading={retryingId === shipment.shipment_id}
              onClick={() => {
                if (decision.allowed) {
                  onRetry(shipment);
                }
              }}
              data-testid="shipment-retry-button"
            >
              重试
            </Button>
          </span>
        </Tooltip>
      ) : null}
    </Space>
  );
};

export const ShipmentsPage = () => {
  const tenantId = useTenantId();
  const shopIds = useShopIds();
  const queryClient = useQueryClient();
  const freightDecision = useFeatureFlag('directory.logistics_price');
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeShipment, setActiveShipment] = useState<ShipmentSummary | null>(null);
  const [retryingId, setRetryingId] = useState<string | null>(null);
  const [confirmingId, setConfirmingId] = useState<string | null>(null);

  const query = useMemo<ShipmentListQuery>(() => {
    const range = filters.updated_range;
    const resolvedShopId = resolveShopId(shopIds, filters.shop_id);
    return {
      platform: filters.platform,
      shop_id: resolvedShopId,
      status: filters.status?.trim() || undefined,
      platform_order_id: filters.platform_order_id?.trim() || undefined,
      updated_from: range?.[0]?.toISOString(),
      updated_to: range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize, shopIds]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['shipments', tenantId, query],
    queryFn: () => listShipments(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const packagesQuery = useQuery({
    queryKey: ['shipment-packages', tenantId, activeShipment?.shipment_id],
    queryFn: () =>
      listShipmentPackages(tenantId, activeShipment?.shipment_id ?? ''),
    enabled: detailOpen && !!activeShipment?.shipment_id,
    retry: false
  });
  const packages: ShipmentPackage[] = packagesQuery.data?.items ?? [];

  const pushAttemptKey =
    activeShipment?.platform_order_id ?? activeShipment?.order_id ?? '';
  const pushAttemptQuery = useQuery({
    queryKey: ['shipment-push-attempts', tenantId, pushAttemptKey],
    queryFn: () =>
      listShipmentPushAttempts(tenantId, {
        platform_order_id: pushAttemptKey,
        limit: 20
      }),
    enabled: detailOpen && Boolean(pushAttemptKey),
    retry: false
  });
  const pushAttempts: ShipmentPushAttempt[] = pushAttemptQuery.data?.data ?? [];

  const freightQuery = useQuery<LogisticsPriceEstimateResponse>({
    queryKey: ['logistics-price-estimate', tenantId, activeShipment?.shipment_id],
    queryFn: () =>
      estimateLogisticsPrice(tenantId, { shipment_id: activeShipment?.shipment_id }),
    enabled: freightDecision.enabled && detailOpen && !!activeShipment?.shipment_id,
    retry: false
  });
  const freightData = freightQuery.data;
  const freightPrice = freightData?.price ?? null;

  const handleRetry = async (shipment: ShipmentSummary) => {
    setRetryingId(shipment.shipment_id);
    try {
      const updated = await retryShipment(tenantId, shipment);
      setActiveShipment(updated);
      await queryClient.invalidateQueries({ queryKey: ['shipments', tenantId] });
      message.success('已提交重试');
    } catch (err) {
      message.error(err instanceof Error ? err.message : '重试失败');
    } finally {
      setRetryingId(null);
    }
  };

  const handleConfirm = async (shipment: ShipmentSummary) => {
    setConfirmingId(shipment.shipment_id);
    try {
      await confirmShipment(tenantId, shipment.shipment_id);
      message.success('回写任务已提交');
      await queryClient.invalidateQueries({ queryKey: ['shipments', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '回写失败');
    } finally {
      setConfirmingId(null);
    }
  };

  const columns = useMemo<ColumnsType<ShipmentSummary>>(
    () => [
      {
        title: '平台订单号',
        dataIndex: 'platform_order_id',
        render: (_, record) => record.platform_order_id ?? record.order_id ?? '-'
      },
      {
        title: '平台',
        dataIndex: 'platform',
        render: (_, record) => <PlatformTag platform={record.platform} />
      },
      {
        title: '店铺',
        dataIndex: 'shop_id'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => <ShipmentStatusTag status={record.status} />
      },
      {
        title: '回传状态',
        dataIndex: 'push_status',
        render: (_, record) => renderPushStatus(record.push_status)
      },
      {
        title: '承运商',
        dataIndex: 'carrier_code'
      },
      {
        title: '运单号',
        dataIndex: 'tracking_no',
        render: (_, record) => record.tracking_no ?? '-'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.requested_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <ShipmentActions
            shipment={record}
            onView={(shipment) => {
              setActiveShipment(shipment);
              setDetailOpen(true);
            }}
            onRetry={(shipment) => handleRetry(shipment)}
            onConfirm={(shipment) => handleConfirm(shipment)}
            retryingId={retryingId}
            confirmingId={confirmingId}
          />
        )
      }
    ],
    [confirmingId, handleConfirm, handleRetry, retryingId]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const detailDecision = useOperationAccess(
    'orders.ship',
    activeShipment?.platform ?? 'taobao'
  );

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      发货/包裹
    </Typography.Title>
  );

  const header = (
    <Card>
      <Form form={form} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="platform" label="平台">
              <Select allowClear options={platformOptions} placeholder="选择平台" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="shop_id" label="店铺">
              <Input allowClear placeholder="输入店铺 ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="status" label="状态">
              <Select allowClear options={statusOptions} placeholder="选择状态" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="platform_order_id" label="平台订单号">
              <Input allowClear placeholder="输入平台订单号" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12}>
            <Form.Item name="updated_range" label="更新时间">
              <DatePicker.RangePicker
                allowClear
                showTime
                style={{ width: '100%' }}
                placeholder={['开始时间', '结束时间']}
              />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button onClick={handleReset}>重置</Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
  );

  return (
    <>
      <UiModule03MainTableSection<ShipmentSummary>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="shipment_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="发货列表加载失败"
        emptyText="暂无发货记录"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="shipments.table.columns"
      />
      <Drawer
        title="发货详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={520}
        destroyOnHidden
      >
        {activeShipment ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Descriptions column={1} bordered size="small">
              <Descriptions.Item label="发货单号">
                {activeShipment.shipment_id}
              </Descriptions.Item>
              <Descriptions.Item label="订单号">
                {activeShipment.platform_order_id ?? activeShipment.order_id ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="平台">
                <PlatformTag platform={activeShipment.platform} />
              </Descriptions.Item>
              <Descriptions.Item label="店铺">{activeShipment.shop_id}</Descriptions.Item>
              <Descriptions.Item label="状态">
                <ShipmentStatusTag status={activeShipment.status} />
              </Descriptions.Item>
              <Descriptions.Item label="回传状态">
                {renderPushStatus(activeShipment.push_status)}
              </Descriptions.Item>
              <Descriptions.Item label="仓库">{activeShipment.warehouse_id}</Descriptions.Item>
              <Descriptions.Item label="承运商">{activeShipment.carrier_code}</Descriptions.Item>
              <Descriptions.Item label="运单号">
                {activeShipment.tracking_no ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {formatDateTime(activeShipment.updated_at ?? activeShipment.requested_at)}
              </Descriptions.Item>
              <Descriptions.Item label="回传时间">
                {formatDateTime(activeShipment.push_attempted_at)}
              </Descriptions.Item>
            </Descriptions>
            <Card size="small" title="Packages">
              {packagesQuery.isLoading ? (
                <Typography.Text>Loading packages...</Typography.Text>
              ) : packagesQuery.error ? (
                <Alert
                  type="warning"
                  showIcon
                  message="Package load failed"
                  description={
                    packagesQuery.error instanceof Error
                      ? packagesQuery.error.message
                      : 'Package load failed'
                  }
                />
              ) : packages.length > 0 ? (
                <Descriptions column={1} bordered size="small">
                  {packages.map((pkg, index) => (
                    <Descriptions.Item
                      key={pkg.package_id}
                      label={`Package ${index + 1}`}
                    >
                      {`${pkg.package_no} / ${pkg.tracking_no ?? '-'} / ${
                        pkg.carrier_code ?? '-'
                      } / ${pkg.weight_gram ?? '-'}`}
                    </Descriptions.Item>
                  ))}
                </Descriptions>
              ) : (
                <Typography.Text type="secondary">No packages</Typography.Text>
              )}
            </Card>
            <Card size="small" title="回写记录">
              {pushAttemptQuery.isLoading ? (
                <Typography.Text>回写记录加载中...</Typography.Text>
              ) : pushAttemptQuery.error ? (
                <Alert
                  type="warning"
                  showIcon
                  message="回写记录加载失败"
                  description={
                    pushAttemptQuery.error instanceof Error
                      ? pushAttemptQuery.error.message
                      : '回写记录加载失败'
                  }
                />
              ) : pushAttempts.length > 0 ? (
                <List
                  size="small"
                  dataSource={pushAttempts}
                  renderItem={(attempt) => (
                    <List.Item>
                      <Space direction="vertical" size={2}>
                        <Typography.Text>
                          {attempt.status} / {attempt.trigger} / 尝试 {attempt.attempt}
                        </Typography.Text>
                        <Typography.Text type="secondary">
                          {attempt.created_at
                            ? formatDateTime(attempt.created_at)
                            : '-'}
                          {attempt.error_message
                            ? ` | ${attempt.error_message}`
                            : ''}
                        </Typography.Text>
                      </Space>
                    </List.Item>
                  )}
                />
              ) : (
                <Typography.Text type="secondary">暂无回写记录</Typography.Text>
              )}
            </Card>
            {activeShipment.status === 'FAILED' ? (
              <Alert
                type="error"
                showIcon
                message="失败原因"
                description={activeShipment.last_error ?? '未知错误'}
              />
            ) : null}
            {activeShipment.push_status === 'FAILED' || activeShipment.push_status === 'DEAD' ? (
              <Alert
                type="warning"
                showIcon
                message="回传失败原因"
                description={activeShipment.push_error_message ?? '暂无回传错误信息'}
              />
            ) : null}
            {freightDecision.enabled ? (
              <Card size="small" title="运费测算">
                {freightQuery.isLoading ? (
                  <Typography.Text>运费测算中...</Typography.Text>
                ) : freightQuery.error ? (
                  <Alert
                    type="warning"
                    showIcon
                    message="运费测算失败"
                    description={
                      freightQuery.error instanceof Error
                        ? freightQuery.error.message
                        : '运费测算失败'
                    }
                  />
                ) : freightData?.matched && freightPrice ? (
                  <Descriptions column={1} bordered size="small">
                    <Descriptions.Item label="预估运费">
                      {formatPrice(freightPrice.price_cent)}
                    </Descriptions.Item>
                    <Descriptions.Item label="计费重量">
                      {`${freightData.weight_kg.toFixed(2)} kg`}
                    </Descriptions.Item>
                    <Descriptions.Item label="重量区间">
                      {`${freightPrice.weight_min_kg} - ${
                        freightPrice.weight_max_kg > 0
                          ? freightPrice.weight_max_kg
                          : '不限'
                      }`}
                    </Descriptions.Item>
                    <Descriptions.Item label="地区编码">
                      {freightData.region_code}
                    </Descriptions.Item>
                    <Descriptions.Item label="物流产品">
                      {freightPrice.product_id ?? '-'}
                    </Descriptions.Item>
                  </Descriptions>
                ) : (
                  <Typography.Text type="secondary">未匹配运费规则</Typography.Text>
                )}
              </Card>
            ) : null}
            {activeShipment.status === 'SUCCESS' &&
            activeShipment.push_status !== 'SUCCESS' ? (
              <Tooltip title={detailDecision.allowed ? undefined : detailDecision.reason}>
                <Button
                  type="primary"
                  disabled={!detailDecision.allowed}
                  loading={confirmingId === activeShipment.shipment_id}
                  onClick={() => handleConfirm(activeShipment)}
                >
                  回写
                </Button>
              </Tooltip>
            ) : null}
            {activeShipment.status === 'FAILED' ? (
              <Tooltip title={detailDecision.allowed ? undefined : detailDecision.reason}>
                <Button
                  type="primary"
                  disabled={!detailDecision.allowed}
                  loading={retryingId === activeShipment.shipment_id}
                  onClick={() => handleRetry(activeShipment)}
                >
                  重试
                </Button>
              </Tooltip>
            ) : null}
          </Space>
        ) : null}
      </Drawer>
    </>
  );
};
