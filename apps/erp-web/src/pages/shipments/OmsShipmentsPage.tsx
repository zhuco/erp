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
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Table,
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

import type { OmsShipmentItem, OmsShipmentListQuery, OmsShipmentSummary } from '@/api/omsShipments';
import {
  createOmsIdempotencyKey,
  getOmsShipment,
  listOmsShipments,
  requestOmsShipmentCancel,
  shipOmsShipment
} from '@/api/omsShipments';
import { listWarehouses } from '@/api/warehouses';
import { PlatformTag } from '@/components/PlatformTag';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { ShipmentStatusTag } from '@/components/ShipmentStatusTag';
import { useOperationAccess } from '@/features/operations';
import { useCan } from '@/features/rbac';
import type { PlatformCode } from '@/features/platform';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  order_id?: string;
  warehouse_id?: string;
  status?: string;
};

type ShipFormValues = {
  carrier_code?: string;
  tracking_no?: string;
  weight_gram?: number;
  shipped_at?: Dayjs;
};

const statusOptions = [
  { label: 'CREATED', value: 'CREATED' },
  { label: 'PARTIALLY_SHIPPED', value: 'PARTIALLY_SHIPPED' },
  { label: 'SHIPPED', value: 'SHIPPED' },
  { label: 'CANCELLED', value: 'CANCELLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

type ShipmentActionsProps = {
  shipment: OmsShipmentSummary;
  onView: (shipment: OmsShipmentSummary) => void;
  onShip: (shipment: OmsShipmentSummary) => void;
  onCancel: (shipment: OmsShipmentSummary) => void;
};

const ShipmentActions = ({ shipment, onView, onShip, onCancel }: ShipmentActionsProps) => {
  const decision = useOperationAccess('orders.ship', shipment.platform as PlatformCode);
  const canCancel = ['SHIPPED', 'PARTIALLY_SHIPPED'].includes(shipment.status);
  const canCancelRequest = useCan('shipping.write');
  const canShip = shipment.status !== 'SHIPPED';

  return (
    <Space>
      <Button size="small" onClick={() => onView(shipment)} data-testid="oms-shipment-view">
        查看
      </Button>
      {canShip ? (
        <Tooltip title={decision.allowed ? undefined : decision.reason}>
          <span>
            <Button
              size="small"
              disabled={!decision.allowed}
              onClick={() => onShip(shipment)}
              data-testid="oms-shipment-ship"
            >
              发货
            </Button>
          </span>
        </Tooltip>
      ) : null}
      {canCancel ? (
        <Tooltip title={canCancelRequest ? undefined : '暂无撤销权限'}>
          <span>
            <Button
              size="small"
              disabled={!canCancelRequest}
              onClick={() => onCancel(shipment)}
              data-testid="oms-shipment-cancel-request"
            >
              撤销申请
            </Button>
          </span>
        </Tooltip>
      ) : null}
    </Space>
  );
};

export const OmsShipmentsPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [shipForm] = Form.useForm<ShipFormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeShipment, setActiveShipment] = useState<OmsShipmentSummary | null>(null);
  const [shipSubmitting, setShipSubmitting] = useState(false);
  const [shipError, setShipError] = useState<string | null>(null);
  const [idempotencyKey, setIdempotencyKey] = useState(createOmsIdempotencyKey());
  const [cancelOpen, setCancelOpen] = useState(false);
  const [cancelShipment, setCancelShipment] = useState<OmsShipmentSummary | null>(null);
  const [cancelReason, setCancelReason] = useState('');
  const [cancelSubmitting, setCancelSubmitting] = useState(false);
  const [cancelIdempotencyKey, setCancelIdempotencyKey] = useState(createOmsIdempotencyKey());

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'oms-shipments'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId)
  });
  const warehouseOptions = useMemo(
    () =>
      (warehouseQuery.data?.items ?? []).map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })),
    [warehouseQuery.data?.items]
  );

  const query = useMemo<OmsShipmentListQuery>(() => {
    return {
      order_id: filters.order_id?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['oms-shipments', tenantId, query],
    queryFn: () => listOmsShipments(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const detailQuery = useQuery({
    queryKey: ['oms-shipment-detail', tenantId, activeShipment?.shipment_id],
    queryFn: () =>
      getOmsShipment(tenantId, activeShipment?.shipment_id ?? ''),
    enabled: detailOpen && Boolean(activeShipment?.shipment_id)
  });

  const shipmentDetail = detailQuery.data ?? activeShipment;
  const shipDecision = useOperationAccess(
    'orders.ship',
    shipmentDetail?.platform ?? 'taobao'
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

  const toolbarLeft = (
    <Typography.Title level={3} style={{ margin: 0 }}>
      OMS包裹
    </Typography.Title>
  );

  const header = (
    <Card>
      <Form form={form} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="order_id" label="订单ID">
              <Input allowClear placeholder="输入订单ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="warehouse_id" label="仓库">
              <Select
                allowClear
                options={warehouseOptions}
                placeholder="选择仓库"
                loading={warehouseQuery.isFetching}
              />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="status" label="状态">
              <Select allowClear options={statusOptions} placeholder="选择状态" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
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

  const openDrawer = (shipment: OmsShipmentSummary) => {
    setActiveShipment(shipment);
    setDetailOpen(true);
    setShipError(null);
    setIdempotencyKey(createOmsIdempotencyKey());
    shipForm.setFieldsValue({
      carrier_code: shipment.carrier_code ?? undefined,
      tracking_no: shipment.tracking_no ?? undefined,
      weight_gram: shipment.weight_gram ?? undefined,
      shipped_at: shipment.shipped_at ? dayjs(shipment.shipped_at) : undefined
    });
  };

  const handleShip = async (values: ShipFormValues) => {
    if (!shipmentDetail) {
      return;
    }
    setShipSubmitting(true);
    setShipError(null);
    try {
      const shipped_at = values.shipped_at?.toISOString();
      const updated = await shipOmsShipment(tenantId, shipmentDetail.shipment_id, {
        carrier_code: values.carrier_code,
        tracking_no: values.tracking_no,
        shipped_at,
        weight_gram: values.weight_gram,
        idempotency_key: idempotencyKey
      });
      setActiveShipment(updated);
      setIdempotencyKey(createOmsIdempotencyKey());
      await queryClient.invalidateQueries({ queryKey: ['oms-shipments', tenantId] });
      await queryClient.invalidateQueries({
        queryKey: ['oms-shipment-detail', tenantId, shipmentDetail.shipment_id]
      });
      message.success('发货成功');
    } catch (err) {
      const messageText = err instanceof Error ? err.message : '发货失败';
      setShipError(messageText);
      message.error(messageText);
    } finally {
      setShipSubmitting(false);
    }
  };

  const openCancelModal = (shipment: OmsShipmentSummary) => {
    setCancelShipment(shipment);
    setCancelReason('');
    setCancelIdempotencyKey(createOmsIdempotencyKey());
    setCancelOpen(true);
  };

  const handleCancelRequest = async () => {
    if (!cancelShipment) {
      return;
    }
    setCancelSubmitting(true);
    try {
      await requestOmsShipmentCancel(tenantId, cancelShipment.shipment_id, {
        reason: cancelReason.trim() || undefined,
        idempotency_key: cancelIdempotencyKey
      });
      message.success('撤销申请已提交');
      setCancelOpen(false);
      setCancelShipment(null);
      await queryClient.invalidateQueries({ queryKey: ['oms-shipments', tenantId] });
      if (detailOpen && shipmentDetail?.shipment_id === cancelShipment.shipment_id) {
        await queryClient.invalidateQueries({
          queryKey: ['oms-shipment-detail', tenantId, cancelShipment.shipment_id]
        });
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '撤销申请失败');
    } finally {
      setCancelSubmitting(false);
    }
  };

  const renderShipmentType = (value?: string | null) => {
    if (!value) {
      return '-';
    }
    if (value === 'RESEND') {
      return '补发';
    }
    if (value === 'OMS') {
      return '正常';
    }
    return value;
  };

  const columns = useMemo<ColumnsType<OmsShipmentSummary>>(
    () => [
      {
        title: '包裹号',
        dataIndex: 'shipment_no',
        render: (_, record) => record.shipment_no ?? record.shipment_id
      },
      {
        title: '平台订单',
        dataIndex: 'platform_order_id',
        render: (_, record) => record.platform_order_id ?? record.order_id
      },
      {
        title: '类型',
        dataIndex: 'shipment_type',
        render: (_, record) => renderShipmentType(record.shipment_type)
      },
      {
        title: '平台',
        dataIndex: 'platform',
        render: (_, record) => <PlatformTag platform={record.platform as PlatformCode} />
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => <ShipmentStatusTag status={record.status} />
      },
      {
        title: '承运商',
        dataIndex: 'carrier_code',
        render: (_, record) => record.carrier_code ?? '-'
      },
      {
        title: '运单号',
        dataIndex: 'tracking_no',
        render: (_, record) => record.tracking_no ?? '-'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <ShipmentActions
            shipment={record}
            onView={(shipment) => openDrawer(shipment)}
            onShip={(shipment) => openDrawer(shipment)}
            onCancel={(shipment) => openCancelModal(shipment)}
          />
        )
      }
    ],
    [openDrawer, openCancelModal, renderShipmentType]
  );

  const itemColumns = useMemo<ColumnsType<OmsShipmentItem>>(
    () => [
      {
        title: 'SKU',
        dataIndex: 'sku_code',
        render: (_, record) => record.sku_code ?? record.sku_id
      },
      {
        title: '数量',
        dataIndex: 'qty'
      }
    ],
    []
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <UiModule03MainTableSection<OmsShipmentSummary>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="shipment_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="包裹列表加载失败"
        emptyText="暂无包裹记录"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="oms-shipments.table.columns"
      />
      <Drawer
        title="包裹详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={560}
        destroyOnHidden
      >
        {shipmentDetail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Descriptions column={1} bordered size="small">
              <Descriptions.Item label="包裹号">
                {shipmentDetail.shipment_no ?? shipmentDetail.shipment_id}
              </Descriptions.Item>
              <Descriptions.Item label="订单">
                {shipmentDetail.platform_order_id ?? shipmentDetail.order_id}
              </Descriptions.Item>
              <Descriptions.Item label="类型">
                {renderShipmentType(shipmentDetail.shipment_type)}
              </Descriptions.Item>
              <Descriptions.Item label="原包裹">
                {shipmentDetail.origin_shipment_id ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="平台">
                <PlatformTag platform={shipmentDetail.platform as PlatformCode} />
              </Descriptions.Item>
              <Descriptions.Item label="仓库">
                {shipmentDetail.warehouse_id}
              </Descriptions.Item>
              <Descriptions.Item label="状态">
                <ShipmentStatusTag status={shipmentDetail.status} />
              </Descriptions.Item>
              <Descriptions.Item label="承运商">
                {shipmentDetail.carrier_code ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="运单号">
                {shipmentDetail.tracking_no ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="发货时间">
                {formatDateTime(shipmentDetail.shipped_at)}
              </Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {formatDateTime(shipmentDetail.updated_at ?? shipmentDetail.created_at)}
              </Descriptions.Item>
            </Descriptions>
            {shipmentDetail.items ? (
              <Card size="small" title="包裹明细">
                <Table<OmsShipmentItem>
                  rowKey={(record) => `${record.order_item_id}-${record.sku_id}`}
                  columns={itemColumns}
                  dataSource={shipmentDetail.items}
                  pagination={false}
                  size="small"
                />
              </Card>
            ) : null}
            {shipmentDetail.status !== 'SHIPPED' ? (
              <Card size="small" title="发货确认">
                <Form form={shipForm} layout="vertical" onFinish={handleShip}>
                  <Form.Item
                    name="carrier_code"
                    label="承运商"
                    rules={[{ required: true, message: '请输入承运商' }]}
                  >
                    <Input allowClear placeholder="例如：SF/YTO" />
                  </Form.Item>
                  <Form.Item name="tracking_no" label="运单号">
                    <Input allowClear placeholder="可选" />
                  </Form.Item>
                  <Form.Item name="weight_gram" label="重量(g)">
                    <InputNumber min={0} style={{ width: '100%' }} />
                  </Form.Item>
                  <Form.Item name="shipped_at" label="发货时间">
                    <DatePicker
                      allowClear
                      showTime
                      style={{ width: '100%' }}
                      placeholder="选择发货时间"
                    />
                  </Form.Item>
                  <Tooltip title={shipDecision.allowed ? undefined : shipDecision.reason}>
                    <Button
                      type="primary"
                      htmlType="submit"
                      disabled={!shipDecision.allowed}
                      loading={shipSubmitting}
                      block
                    >
                      确认发货
                    </Button>
                  </Tooltip>
                </Form>
                {shipError ? (
                  <Alert
                    style={{ marginTop: 12 }}
                    type="error"
                    showIcon
                    message="发货失败"
                    description={shipError}
                  />
                ) : null}
              </Card>
            ) : null}
          </Space>
        ) : null}
      </Drawer>
      <Modal
        title="提交撤销申请"
        open={cancelOpen}
        onOk={handleCancelRequest}
        onCancel={() => setCancelOpen(false)}
        confirmLoading={cancelSubmitting}
        okText="提交"
        cancelText="取消"
        destroyOnHidden
      >
        <Input.TextArea
          value={cancelReason}
          onChange={(event) => setCancelReason(event.target.value)}
          placeholder="请输入撤销原因（可选）"
          rows={4}
        />
      </Modal>
    </Space>
  );
};
