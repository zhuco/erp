import {
  Alert,
  Button,
  Card,
  Descriptions,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Tooltip,
  Typography,
  message
} from 'antd';
import { CopyOutlined, ReloadOutlined } from '@ant-design/icons';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type { ShipmentSummary } from '@/api/shipments';
import { createIdempotencyKey, retryShipment, shipOrder } from '@/api/shipments';
import { listWarehouses } from '@/api/warehouses';
import { ShipmentStatusTag } from '@/components/ShipmentStatusTag';
import { PlatformTag } from '@/components/PlatformTag';
import { UiModule07SettingsLayout } from '@/components/ui-modules';
import { useOperationAccess } from '@/features/operations';
import { useOrderAuditSettingsStore } from '@/features/orderAuditSettings';
import { platformValues } from '@/features/platform';
import { createPrintJobAndPrint } from '@/features/printing/printJobFlow';
import { useFeatureFlag } from '@/features/featureFlags';
import { useTenantId, useWarehouseId } from '@/features/tenant';
import type { OrderSummary } from '@/api/orders';
import { getOrderDetail } from '@/api/orders';
import { listInventory, reserveInventory } from '@/api/inventory';

type ShipOrderDrawerProps = {
  open: boolean;
  order: OrderSummary | null;
  onClose: () => void;
};

type ShipFormValues = {
  warehouse_id: string;
  carrier_code: string;
  tracking_no?: string;
  weight_gram?: number;
};

const orderStatusLabels: Record<string, string> = {
  NEW: '新建',
  REVIEW_PENDING: '待复核',
  PENDING_SHIPMENT: '待发货',
  ALLOCATED: '已分配',
  PICKING: '拣货中',
  PACKED: '已打包',
  SHIPPED: '已发货',
  DONE: '已完成',
  CANCELLED: '已取消'
};

export const ShipOrderDrawer = ({ open, order, onClose }: ShipOrderDrawerProps) => {
  const tenantId = useTenantId();
  const defaultWarehouseId = useWarehouseId();
  const [form] = Form.useForm<ShipFormValues>();
  const [idempotencyKey, setIdempotencyKey] = useState(createIdempotencyKey());
  const [shipment, setShipment] = useState<ShipmentSummary | null>(null);
  const [error, setError] = useState<Error | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [printing, setPrinting] = useState(false);
  const auditEnabled = useOrderAuditSettingsStore((state) => state.enabled);
  const auditMode = useOrderAuditSettingsStore((state) => state.mode);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'ship'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId)
  });
  const warehouses = warehouseQuery.data?.items ?? [];
  const defaultWarehouse = useMemo(
    () => warehouses.find((warehouse) => warehouse.is_default) ?? warehouses[0] ?? null,
    [warehouses]
  );
  const warehouseOptions = useMemo(() => {
    if (!defaultWarehouse) {
      return [];
    }
    return [{ label: defaultWarehouse.name, value: defaultWarehouse.warehouse_id }];
  }, [defaultWarehouse]);
  const warehouseSelectDisabled = warehouseOptions.length <= 1;
  const inventoryDedupRef = useRef(new Set<string>());

  const fallbackPlatform = platformValues[0] ?? 'taobao';
  const operationDecision = useOperationAccess('orders.ship', order?.platform ?? fallbackPlatform);

  const resolvedWarehouseId = useMemo(() => {
    if (defaultWarehouseId) {
      return defaultWarehouseId;
    }
    return defaultWarehouse?.warehouse_id ?? '';
  }, [defaultWarehouseId, defaultWarehouse]);

  useEffect(() => {
    if (!open || !order) {
      return;
    }
    setIdempotencyKey(createIdempotencyKey());
    inventoryDedupRef.current.clear();
    setShipment(null);
    setError(null);
    form.setFieldsValue({
      warehouse_id: resolvedWarehouseId,
      carrier_code: '',
      tracking_no: '',
      weight_gram: undefined
    });
  }, [open, order, resolvedWarehouseId, form]);

  const reserveInventoryForOrder = async (
    warehouseId: string,
    idempotency: string
  ): Promise<void> => {
    if (!order || !inventoryFeature.enabled) {
      return;
    }
    if (inventoryDedupRef.current.has(idempotency)) {
      return;
    }
    inventoryDedupRef.current.add(idempotency);
    try {
      const detail = await getOrderDetail(tenantId, order.id);
      const items = detail.items ?? [];
      for (const item of items) {
        const quantity = item.quantity ?? 0;
        if (!quantity) {
          continue;
        }
        const skuCode = item.outer_sku_id ?? item.platform_sku_id ?? undefined;
        const barcode = undefined;
        if (!skuCode && !barcode) {
          continue;
        }
        const inventory = await listInventory(tenantId, {
          warehouse_id: warehouseId,
          sku_code: skuCode,
          barcode,
          offset: 0,
          limit: 1
        });
        const record = inventory.items[0];
        if (!record?.sku_id) {
          continue;
        }
        await reserveInventory(tenantId, {
          warehouse_id: warehouseId,
          sku_id: record.sku_id,
          qty: quantity,
          ref_type: 'SHIP_ORDER',
          ref_id: idempotency
        });
      }
    } catch (err) {
      inventoryDedupRef.current.delete(idempotency);
      throw err;
    }
  };

  const handleCopyKey = async () => {
    try {
      await navigator.clipboard.writeText(idempotencyKey);
      message.success('已复制 Idempotency-Key');
    } catch {
      message.error('复制失败，请手动复制');
    }
  };

  const submitShipment = async (values: ShipFormValues) => {
    if (!order) {
      return;
    }
    setSubmitting(true);
    try {
      await reserveInventoryForOrder(values.warehouse_id, idempotencyKey);
      const nextShipment = await shipOrder(tenantId, order.id, {
        warehouse_id: values.warehouse_id,
        carrier_code: values.carrier_code,
        tracking_no: values.tracking_no,
        weight_gram: values.weight_gram,
        idempotency_key: idempotencyKey
      });
      setShipment(nextShipment);
      setError(null);
    } catch (err) {
      setError(err as Error);
    } finally {
      setSubmitting(false);
    }
  };

  const handleSubmit = async (values: ShipFormValues) => {
    if (!order) {
      return;
    }
    if (auditEnabled) {
      const modeLabel = auditMode === 'auto' ? '自动审核' : '手动审核';
      Modal.confirm({
        title: '确认审核并发货',
        content: `当前审核模式：${modeLabel}，确认后将进入发货流程。`,
        okText: '确认审核',
        cancelText: '取消',
        onOk: () => submitShipment(values)
      });
      return;
    }
    await submitShipment(values);
  };

  const handleRetry = async () => {
    if (!shipment) {
      return;
    }
    setSubmitting(true);
    try {
      const nextShipment = await retryShipment(tenantId, shipment);
      setShipment(nextShipment);
      setError(null);
    } catch (err) {
      setError(err as Error);
    } finally {
      setSubmitting(false);
    }
  };

  const handlePrintLabel = async () => {
    if (!order || !shipment) {
      return;
    }
    setPrinting(true);
    try {
      const { job } = await createPrintJobAndPrint(tenantId, {
        order_id: order.id,
        platform: order.platform,
        shop_id: order.shop_id,
        shipment_id: shipment.shipment_id,
        tracking_no: shipment.tracking_no ?? null,
        carrier_code: shipment.carrier_code ?? null
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

  const retryButton = shipment?.status === 'FAILED' ? (
    <Tooltip title={operationDecision.allowed ? undefined : operationDecision.reason}>
      <Button
        type="primary"
        icon={<ReloadOutlined />}
        disabled={!operationDecision.allowed}
        loading={submitting}
        onClick={handleRetry}
        data-testid="ship-retry-button"
      >
        重试
      </Button>
    </Tooltip>
  ) : null;

  const printButton =
    shipment && shipment.status !== 'FAILED' ? (
      <Button
        type="primary"
        ghost
        loading={printing}
        onClick={handlePrintLabel}
        data-testid="ship-print-label"
      >
        打印面单
      </Button>
    ) : null;

  const statusActions =
    retryButton || printButton ? (
      <Space>
        {printButton}
        {retryButton}
      </Space>
    ) : null;

  return (
    <Drawer
      title="发货"
      open={open}
      onClose={onClose}
      width={520}
      destroyOnClose
      data-testid="ship-drawer"
    >
      {order ? (
        <UiModule07SettingsLayout>
          <Card size="small">
            <Descriptions column={2} size="small">
              <Descriptions.Item label="订单号">
                {order.platform_order_id ?? order.id}
              </Descriptions.Item>
              <Descriptions.Item label="平台">
                <PlatformTag platform={order.platform} />
              </Descriptions.Item>
              <Descriptions.Item label="店铺">{order.shop_id}</Descriptions.Item>
              <Descriptions.Item label="平台状态">{order.platform_status ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="订单状态">
                {orderStatusLabels[order.oms_status] ?? order.oms_status ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="平台订单号">
                {order.platform_order_id ?? '-'}
              </Descriptions.Item>
            </Descriptions>
          </Card>
          <Card size="small" title="发货信息">
            <Form form={form} layout="vertical" onFinish={handleSubmit}>
              <Form.Item
                name="warehouse_id"
                label="仓库"
                rules={[{ required: true, message: '请选择仓库' }]}
              >
                <Select
                  options={warehouseOptions}
                  loading={warehouseQuery.isFetching}
                  disabled={warehouseSelectDisabled}
                  placeholder="选择仓库"
                  data-testid="ship-warehouse-select"
                />
              </Form.Item>
              <Form.Item
                name="carrier_code"
                label="承运商"
                rules={[{ required: true, message: '请输入承运商代码' }]}
              >
                <Input
                  allowClear
                  placeholder="例如：SF/YTO"
                  data-testid="ship-carrier-input"
                />
              </Form.Item>
              <Form.Item name="tracking_no" label="运单号">
                <Input allowClear placeholder="可选" data-testid="ship-tracking-input" />
              </Form.Item>
              <Form.Item name="weight_gram" label="重量(克)">
                <InputNumber min={0} style={{ width: '100%' }} placeholder="可选" />
              </Form.Item>
              <Form.Item label="Idempotency-Key">
                <Input
                  value={idempotencyKey}
                  readOnly
                  addonAfter={
                    <Button
                      type="text"
                      icon={<CopyOutlined />}
                      aria-label="Copy idempotency key"
                      onClick={handleCopyKey}
                      data-testid="ship-idempotency-copy"
                    />
                  }
                  data-testid="ship-idempotency-input"
                />
              </Form.Item>
              <Button
                type="primary"
                htmlType="submit"
                disabled={!operationDecision.allowed}
                loading={submitting}
                data-testid="ship-submit"
                block
              >
                提交发货
              </Button>
            </Form>
          </Card>
          {error ? (
            <Alert type="error" showIcon message="发货失败" description={error.message} />
          ) : null}
          {shipment ? (
            <Card
              size="small"
              title="发货状态"
              extra={statusActions}
              data-testid="ship-status-card"
            >
              <Descriptions column={1} size="small">
                <Descriptions.Item label="状态">
                  <ShipmentStatusTag status={shipment.status} />
                </Descriptions.Item>
                <Descriptions.Item label="发货单号">
                  {shipment.shipment_id}
                </Descriptions.Item>
                <Descriptions.Item label="仓库">{shipment.warehouse_id}</Descriptions.Item>
                <Descriptions.Item label="承运商">{shipment.carrier_code}</Descriptions.Item>
                <Descriptions.Item label="运单号">
                  {shipment.tracking_no ?? '-'}
                </Descriptions.Item>
                <Descriptions.Item label="重量(克)">
                  {shipment.weight_gram ?? '-'}
                </Descriptions.Item>
                <Descriptions.Item label="更新时间">
                  {shipment.updated_at ?? shipment.requested_at ?? '-'}
                </Descriptions.Item>
              </Descriptions>
              {shipment.status === 'FAILED' ? (
                <Alert
                  type="error"
                  showIcon
                  message="发货失败原因"
                  description={
                    shipment.last_error ? (
                      <Typography.Text>{shipment.last_error}</Typography.Text>
                    ) : (
                      '未知错误'
                    )
                  }
                  style={{ marginTop: 12 }}
                />
              ) : null}
            </Card>
          ) : null}
        </UiModule07SettingsLayout>
      ) : null}
    </Drawer>
  );
};
