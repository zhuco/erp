import {
  Button,
  Descriptions,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate, useParams } from 'react-router-dom';

import type {
  PurchaseOrderItem,
  PurchaseOrderReceivePayload,
  PurchaseOrderStatus,
  PurchaseReceiptSummary
} from '@/api/purchaseOrders';
import {
  confirmPurchaseOrder,
  getPurchaseOrder,
  receivePurchaseOrder,
  sendPurchaseOrder
} from '@/api/purchaseOrders';
import { listSettings, type Setting } from '@/api/system';
import { listWarehouses } from '@/api/warehouses';
import { SettingsPageTemplate, SettingsPageSection } from '@/components/page-templates';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const statusColors: Record<PurchaseOrderStatus, string> = {
  DRAFT: 'default',
  SENT: 'blue',
  CONFIRMED: 'purple',
  RECEIVING: 'gold',
  DONE: 'green',
  CANCELLED: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const parseSettingBoolean = (setting?: Setting | null) => {
  if (!setting) {
    return false;
  }
  const raw = setting.value_json as unknown;
  if (typeof raw === 'boolean') {
    return raw;
  }
  if (raw && typeof raw === 'object' && 'value' in raw) {
    return Boolean((raw as { value?: unknown }).value);
  }
  return false;
};

type ReceiveFormValues = {
  warehouse_id: string;
  items: Array<{ sku_id: string; qty: number }>;
};

export const PurchaseOrderDetailPage = () => {
  const { po_id: poId } = useParams<{ po_id: string }>();
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const navigate = useNavigate();
  const canWrite = useCan('purchase_orders.write');
  const [receiveOpen, setReceiveOpen] = useState(false);
  const [receiveForm] = Form.useForm<ReceiveFormValues>();

  const detailQuery = useQuery({
    queryKey: ['purchase-order', tenantId, poId],
    queryFn: () => getPurchaseOrder(tenantId, poId ?? ''),
    enabled: tenantReady && !!poId
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'purchase-receive'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const approvalSettingQuery = useQuery({
    queryKey: ['settings', tenantId, 'procurement-approval'],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: 'procurement.approval_required'
      }),
    enabled: tenantReady,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const approvalRequired = useMemo(
    () => parseSettingBoolean(approvalSettingQuery.data?.[0]),
    [approvalSettingQuery.data]
  );

  const receiveMutation = useMutation({
    mutationFn: (payload: PurchaseOrderReceivePayload) =>
      receivePurchaseOrder(tenantId, poId ?? '', payload),
    onSuccess: (response) => {
      message.success('Purchase receipt recorded');
      setReceiveOpen(false);
      receiveForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['purchase-order', tenantId, poId] });
      queryClient.invalidateQueries({ queryKey: ['purchase-orders', tenantId] });
      if (response.inbound_doc_id) {
        navigate(`/wms/inbounds/${response.inbound_doc_id}/receive`);
      }
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to receive purchase order');
    }
  });

  const sendMutation = useMutation({
    mutationFn: () => sendPurchaseOrder(tenantId, poId ?? ''),
    onSuccess: () => {
      message.success('Purchase order sent');
      queryClient.invalidateQueries({ queryKey: ['purchase-order', tenantId, poId] });
      queryClient.invalidateQueries({ queryKey: ['purchase-orders', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to send purchase order');
    }
  });

  const confirmMutation = useMutation({
    mutationFn: () => confirmPurchaseOrder(tenantId, poId ?? ''),
    onSuccess: () => {
      message.success('Purchase order confirmed');
      queryClient.invalidateQueries({ queryKey: ['purchase-order', tenantId, poId] });
      queryClient.invalidateQueries({ queryKey: ['purchase-orders', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to confirm purchase order');
    }
  });

  const handleReceiveOpen = () => {
    const detail = detailQuery.data;
    if (!detail) {
      return;
    }
    const items = detail.items
      .map((item) => ({
        sku_id: item.sku_id,
        qty: Math.max(item.qty - item.received_qty, 0)
      }))
      .filter((item) => item.qty > 0);
    const defaultWarehouseId = detail.order.warehouse_id ?? detail.receipts[0]?.warehouse_id;
    receiveForm.setFieldsValue({
      warehouse_id: defaultWarehouseId ?? undefined,
      items: items.length ? items : [{ sku_id: '', qty: 1 }]
    });
    setReceiveOpen(true);
  };

  const handleReceive = async () => {
    const values = await receiveForm.validateFields();
    receiveMutation.mutate({
      warehouse_id: values.warehouse_id,
      items: values.items.map((item) => ({
        sku_id: item.sku_id,
        qty: item.qty
      }))
    });
  };

  const itemColumns = useMemo<ColumnsType<PurchaseOrderItem>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_id' },
      { title: 'Qty', dataIndex: 'qty' },
      { title: 'Price', dataIndex: 'price_cent', render: (value) => `¥${(value / 100).toFixed(2)}` },
      { title: 'Received', dataIndex: 'received_qty' }
    ],
    []
  );

  const receiptColumns = useMemo<ColumnsType<PurchaseReceiptSummary>>(
    () => [
      { title: 'Receipt', dataIndex: 'receipt_id' },
      {
        title: 'Warehouse',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      { title: 'Status', dataIndex: 'status' },
      { title: 'Confirmed At', dataIndex: 'confirmed_at', render: formatDateTime },
      { title: 'Created At', dataIndex: 'created_at', render: formatDateTime }
    ],
    [warehouseMap]
  );

  const detail = detailQuery.data;
  const canReceive =
    !!detail &&
    detail.order.status !== 'DONE' &&
    detail.order.status !== 'CANCELLED' &&
    detail.order.status !== 'DRAFT' &&
    (!approvalRequired || detail.order.status !== 'SENT');
  const canConfirm = approvalRequired && detail?.order.status === 'SENT';
  const headerActions = (
    <Space>
      <Button onClick={() => navigate(-1)}>Back</Button>
      <Button
        type="primary"
        disabled={!canWrite || detail?.order.status !== 'DRAFT'}
        loading={sendMutation.isPending}
        onClick={() => sendMutation.mutate()}
      >
        Send
      </Button>
      {approvalRequired ? (
        <Button
          type="primary"
          disabled={!canWrite || !canConfirm}
          loading={confirmMutation.isPending}
          onClick={() => confirmMutation.mutate()}
        >
          Confirm
        </Button>
      ) : null}
      <Button type="primary" disabled={!canWrite || !canReceive} onClick={handleReceiveOpen}>
        Receive
      </Button>
    </Space>
  );

  return (
    <SettingsPageTemplate title="Purchase Order Detail">
      <SettingsPageSection title="Order Overview" actions={headerActions}>
        {detail ? (
          <Descriptions column={2} bordered size="small">
            <Descriptions.Item label="PO ID">{detail.order.po_id}</Descriptions.Item>
            <Descriptions.Item label="Supplier">
              {detail.order.supplier_name ?? detail.order.supplier_id}
            </Descriptions.Item>
            <Descriptions.Item label="Warehouse">
              {detail.order.warehouse_id
                ? warehouseMap.get(detail.order.warehouse_id) ?? detail.order.warehouse_id
                : '-'}
            </Descriptions.Item>
            <Descriptions.Item label="Expected At">
              {formatDateTime(detail.order.expected_at)}
            </Descriptions.Item>
            <Descriptions.Item label="Remark">{detail.order.remark || '-'}</Descriptions.Item>
            <Descriptions.Item label="Status">
              <Tag color={statusColors[detail.order.status]}>{detail.order.status}</Tag>
            </Descriptions.Item>
            <Descriptions.Item label="Total">
              ¥{(detail.order.total_cent / 100).toFixed(2)}
            </Descriptions.Item>
            <Descriptions.Item label="Created At">
              {formatDateTime(detail.order.created_at)}
            </Descriptions.Item>
            <Descriptions.Item label="Updated At">
              {formatDateTime(detail.order.updated_at)}
            </Descriptions.Item>
          </Descriptions>
        ) : (
          <Typography.Text type="secondary">
            {detailQuery.isFetching ? 'Loading...' : 'No data'}
          </Typography.Text>
        )}
      </SettingsPageSection>
      <SettingsPageSection title="Items">
        <Table
          rowKey="sku_id"
          columns={itemColumns}
          dataSource={detail?.items ?? []}
          pagination={false}
          size="small"
        />
      </SettingsPageSection>
      <SettingsPageSection title="Receipts">
        <Table
          rowKey="receipt_id"
          columns={receiptColumns}
          dataSource={detail?.receipts ?? []}
          pagination={false}
          size="small"
        />
      </SettingsPageSection>
      <Modal
        title="Receive Purchase Order"
        open={receiveOpen}
        onCancel={() => setReceiveOpen(false)}
        onOk={handleReceive}
        confirmLoading={receiveMutation.isPending}
        okText="Confirm"
        destroyOnHidden
      >
        <Form form={receiveForm} layout="vertical">
          <Form.Item
            name="warehouse_id"
            label="Warehouse"
            rules={[{ required: true, message: 'Select warehouse' }]}
          >
            <Select
              options={warehouseOptions}
              placeholder="Select warehouse"
              loading={warehouseQuery.isFetching}
            />
          </Form.Item>
          <Form.List name="items">
            {(fields, { add, remove }) => (
              <Space direction="vertical" style={{ width: '100%' }}>
                {fields.map((field, index) => (
                  <Space key={field.key} align="baseline" style={{ display: 'flex' }}>
                    <Form.Item
                      {...field}
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true, message: 'SKU required' }]}
                    >
                      <Input placeholder="SKU ID" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'qty']}
                      rules={[{ required: true, message: 'Qty required' }]}
                    >
                      <InputNumber min={1} />
                    </Form.Item>
                    {fields.length > 1 ? (
                      <Button type="link" onClick={() => remove(field.name)}>
                        Remove
                      </Button>
                    ) : null}
                    {index === fields.length - 1 ? (
                      <Button type="link" onClick={() => add({ sku_id: '', qty: 1 })}>
                        Add
                      </Button>
                    ) : null}
                  </Space>
                ))}
              </Space>
            )}
          </Form.List>
        </Form>
      </Modal>
    </SettingsPageTemplate>
  );
};
