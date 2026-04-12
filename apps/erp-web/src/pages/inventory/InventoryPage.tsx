import {
  Alert,
  Button,
  Card,
  Checkbox,
  Col,
  Descriptions,
  Form,
  Input,
  InputNumber,
  Modal,
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
import { useEffect, useMemo, useRef, useState } from 'react';
import type { Key } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type { InventoryActionKind, InventoryActionPayload, InventorySnapshot } from '@/api/inventory';
import {
  adjustInventory,
  listInventory,
  releaseInventory,
  reserveInventory
} from '@/api/inventory';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection, UiModule03TableFooter } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  sku_code?: string;
  barcode?: string;
  tag?: string;
  on_hand_negative?: boolean;
  available_below_safety?: boolean;
};

type ActionFormValues = {
  qty: number;
  adjust_direction?: 'INCREASE' | 'DECREASE';
  ref_type?: string;
  ref_id?: string;
};

type InventoryActionState = {
  kind: InventoryActionKind;
  record: InventorySnapshot;
};

const actionLabels: Record<InventoryActionKind, string> = {
  ADJUST: '调整',
  RESERVE: '预占',
  RELEASE: '释放'
};
const actionKinds: InventoryActionKind[] = ['ADJUST', 'RESERVE', 'RELEASE'];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatCurrency = (value?: number | null) =>
  value == null ? '-' : `¥${(value / 100).toFixed(2)}`;

export const InventoryPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const costFeature = useFeatureFlag('cost.enabled');
  const [form] = Form.useForm<FilterValues>();
  const [actionForm] = Form.useForm<ActionFormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [actionState, setActionState] = useState<InventoryActionState | null>(null);
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const tableContainerRef = useRef<HTMLDivElement | null>(null);
  const resolveInventoryRowKey = (record: InventorySnapshot) =>
    record.id ?? `${record.sku_id}-${record.warehouse_id ?? 'NA'}`;

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'options'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
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

  const defaultWarehouseId = useMemo(
    () => warehouseQuery.data?.items.find((warehouse) => warehouse.is_default)?.warehouse_id,
    [warehouseQuery.data?.items]
  );

  const query = useMemo(
    () => ({
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      sku_code: filters.sku_code?.trim() || undefined,
      barcode: filters.barcode?.trim() || undefined,
      tag: filters.tag?.trim() || undefined,
      on_hand_negative: filters.on_hand_negative ? true : undefined,
      available_below_safety: filters.available_below_safety ? true : undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const inventoryQuery = useQuery({
    queryKey: ['inventory', tenantId, 'warehouse', query],
    queryFn: () => listInventory(tenantId, query),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const inventoryItems = inventoryQuery.data?.items ?? [];
  const total = inventoryQuery.data?.total ?? 0;
  const inventoryLoading = inventoryQuery.isFetching && inventoryItems.length === 0;
  const selectedRecord = useMemo(() => {
    if (!selectedRowKeys.length) {
      return inventoryItems[0] ?? null;
    }
    const keySet = new Set(selectedRowKeys);
    return (
      inventoryItems.find((record) => keySet.has(resolveInventoryRowKey(record))) ??
      inventoryItems[0] ??
      null
    );
  }, [inventoryItems, selectedRowKeys]);

  useEffect(() => {
    if (!inventoryItems.length) {
      if (selectedRowKeys.length) {
        setSelectedRowKeys([]);
      }
      return;
    }
    const keySet = new Set(selectedRowKeys);
    const hasSelection = inventoryItems.some((record) => keySet.has(resolveInventoryRowKey(record)));
    if (!hasSelection) {
      setSelectedRowKeys([resolveInventoryRowKey(inventoryItems[0])]);
    }
  }, [inventoryItems, selectedRowKeys]);

  const actionMutation = useMutation({
    mutationFn: async ({
      kind,
      payload
    }: {
      kind: InventoryActionKind;
      payload: InventoryActionPayload;
    }) => {
      if (kind === 'ADJUST') {
        return adjustInventory(tenantId, payload);
      }
      if (kind === 'RESERVE') {
        return reserveInventory(tenantId, payload);
      }
      return releaseInventory(tenantId, payload);
    },
    onSuccess: () => {
      message.success('库存操作已提交');
      setActionState(null);
      actionForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['inventory', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '库存操作失败');
    }
  });

  const inventoryActionsAllowed = inventoryFeature.enabled && canWrite;
  const costDisabledReason = !inventoryFeature.enabled
    ? '库存管理已关闭'
    : !costFeature.enabled
      ? '成本管理已关闭'
      : undefined;
  const selectedActionDisabledReason = !inventoryFeature.enabled
    ? '库存管理已关闭'
    : !canWrite
      ? '暂无库存操作权限'
      : !selectedRecord
        ? '暂无可操作库存记录'
        : undefined;
  const openActionModal = (kind: InventoryActionKind, record: InventorySnapshot) => {
    setActionState({ kind, record });
    actionForm.setFieldsValue({
      qty: 1,
      adjust_direction: 'INCREASE'
    });
  };

  const columns = useMemo<ColumnsType<InventorySnapshot>>(() => {
    const baseColumns: ColumnsType<InventorySnapshot> = [
      {
        title: 'SKU 编码',
        dataIndex: 'sku_code'
      },
      {
        title: 'SKU ID',
        dataIndex: 'sku_id'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value?: string | null) =>
          value ? warehouseMap.get(value) ?? value : filters.warehouse_id ?? '-'
      }
    ];

    if (costFeature.enabled) {
      baseColumns.push({
        title: '成本',
        dataIndex: 'cost_cent',
        align: 'right',
        render: (value) => formatCurrency(value)
      });
    }

    baseColumns.push(
      {
        title: '库存余额',
        key: 'on_hand',
        render: (_, record) =>
          record.on_hand ?? record.available + record.reserved + (record.frozen ?? 0)
      },
      {
        title: '预占',
        dataIndex: 'reserved'
      },
      {
        title: '冻结',
        dataIndex: 'frozen',
        render: (value) => value ?? 0
      },
      {
        title: '在途',
        dataIndex: 'in_transit',
        render: (value) => value ?? 0
      },
      {
        title: '可用',
        dataIndex: 'available'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value) => formatDateTime(value)
      }
    );

    return baseColumns;
  }, [
    costFeature.enabled,
    filters.warehouse_id,
    warehouseMap
  ]);

  const tableFooterSyncKey = useMemo(
    () => `${columns.length}-${inventoryItems.length}`,
    [columns.length, inventoryItems.length]
  );

  const tableFooter = (
    <UiModule03TableFooter
      tableContainerRef={tableContainerRef}
      syncKey={tableFooterSyncKey}
      summary={`共 ${total} 条`}
      total={total}
      pagination={pagination}
      onPaginationChange={setPagination}
    />
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

  const handleSubmitAction = async () => {
    if (!actionState) {
      return;
    }
    if (!inventoryActionsAllowed) {
      message.error('库存管理已关闭');
      return;
    }
    let values: ActionFormValues;
    try {
      values = await actionForm.validateFields();
    } catch {
      return;
    }
    const resolvedWarehouseId =
      actionState.record.warehouse_id ?? filters.warehouse_id ?? defaultWarehouseId;
    if (!resolvedWarehouseId) {
      message.error('请先选择仓库');
      return;
    }
    const normalizedQty = Math.abs(values.qty);
    const qty =
      actionState.kind === 'ADJUST' && values.adjust_direction === 'DECREASE'
        ? -normalizedQty
        : normalizedQty;
    actionMutation.mutate({
      kind: actionState.kind,
      payload: {
        warehouse_id: resolvedWarehouseId,
        sku_id: actionState.record.sku_id,
        qty,
        ref_type: values.ref_type?.trim() || undefined,
        ref_id: values.ref_id?.trim() || undefined
      }
    });
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Space align="center">
          <Typography.Title level={3} data-testid="inventory-title">
            分仓库存
          </Typography.Title>
        </Space>
        <Space>
          <Tooltip title={!inventoryFeature.enabled ? '库存管理已关闭' : undefined}>
            <span>
              <Button
                onClick={() => navigate('/inventory/ledgers')}
                disabled={!inventoryFeature.enabled}
              >
                查看流水
              </Button>
            </span>
          </Tooltip>
          <Tooltip title={costDisabledReason}>
            <span>
              <Button
                onClick={() => navigate('/inventory/costs')}
                disabled={!inventoryFeature.enabled || !costFeature.enabled}
              >
                库存成本
              </Button>
            </span>
          </Tooltip>
          <Tooltip title={!inventoryFeature.enabled ? '库存管理已关闭' : undefined}>
            <span>
              <Button
                onClick={() => navigate('/inventory/sync-jobs')}
                disabled={!inventoryFeature.enabled}
              >
                同步任务
              </Button>
            </span>
          </Tooltip>
          {actionKinds.map((kind) => (
            <Tooltip key={kind} title={selectedActionDisabledReason}>
              <span>
                <Button
                  onClick={() => {
                    if (!selectedRecord || selectedActionDisabledReason) {
                      return;
                    }
                    openActionModal(kind, selectedRecord);
                  }}
                  disabled={Boolean(selectedActionDisabledReason)}
                  data-testid={
                    kind === 'ADJUST'
                      ? 'inventory-selected-adjust-button'
                      : kind === 'RESERVE'
                        ? 'inventory-selected-reserve-button'
                        : 'inventory-selected-release-button'
                  }
                >
                  {actionLabels[kind]}
                </Button>
              </span>
            </Tooltip>
          ))}
        </Space>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="warehouse_id" label="仓库">
                <Select
                  allowClear
                  options={warehouseOptions}
                  placeholder="选择仓库"
                  loading={warehouseQuery.isFetching}
                  data-testid="inventory-warehouse-select"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="sku_code" label="SKU 编码">
                <Input
                  allowClear
                  placeholder="输入 SKU 编码"
                  data-testid="inventory-sku-code-input"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="barcode" label="条码">
                <Input
                  allowClear
                  placeholder="输入条码"
                  data-testid="inventory-barcode-input"
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="tag" label="标签">
                <Input allowClear placeholder="输入标签" data-testid="inventory-tag-input" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="on_hand_negative" valuePropName="checked" label=" ">
                <Checkbox>库存 &lt; 0</Checkbox>
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="available_below_safety" valuePropName="checked" label=" ">
                <Checkbox>可用 &lt; 安全库存</Checkbox>
              </Form.Item>
            </Col>
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit" data-testid="inventory-search-submit">
                  查询
                </Button>
                <Button onClick={handleReset} data-testid="inventory-search-reset">
                  重置
                </Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      {!inventoryFeature.enabled ? (
        <Alert
          type="warning"
          showIcon
          message="库存管理已关闭"
          description="库存查询与操作已暂停，如需使用请先在设置中开启。"
        />
      ) : null}
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
      {inventoryFeature.enabled ? (
        <UiModule03MainTableSection<InventorySnapshot>
          tableContainerRef={tableContainerRef}
          dataTestId="inventory-table"
          rowKey={resolveInventoryRowKey}
          rowSelection={{
            type: 'radio',
            selectedRowKeys,
            onChange: (keys) => setSelectedRowKeys(keys)
          }}
          columns={columns}
          data={inventoryItems}
          loading={inventoryLoading}
          error={inventoryQuery.error}
          onRetry={() => inventoryQuery.refetch()}
          errorTitle="库存加载失败"
          emptyText="暂无库存记录"
          total={inventoryQuery.data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="inventory.warehouse.table.columns"
          virtual
          enablePanels={false}
          tableFooter={tableFooter}
          showStatusBar={false}
        />
      ) : null}
      <Modal
        title={actionState ? `${actionLabels[actionState.kind]}库存` : '库存操作'}
        open={Boolean(actionState)}
        onCancel={() => {
          setActionState(null);
          actionForm.resetFields();
        }}
        onOk={handleSubmitAction}
        confirmLoading={actionMutation.isPending}
        okText="提交"
        okButtonProps={{ 'data-testid': 'inventory-action-submit' }}
        destroyOnClose
        data-testid="inventory-action-modal"
      >
        {actionState ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions size="small" column={1}>
              <Descriptions.Item label="SKU 编码">
                {actionState.record.sku_code}
              </Descriptions.Item>
              <Descriptions.Item label="SKU ID">{actionState.record.sku_id}</Descriptions.Item>
              <Descriptions.Item label="仓库">
                {warehouseMap.get(actionState.record.warehouse_id ?? '') ??
                  actionState.record.warehouse_id ??
                  '-'}
                {actionState.record.warehouse_id ? null : (
                  <Tag color="orange" style={{ marginLeft: 8 }}>
                    请选择仓库
                  </Tag>
                )}
              </Descriptions.Item>
            </Descriptions>
            <Form form={actionForm} layout="vertical">
              {actionState.kind === 'ADJUST' ? (
                <Form.Item
                  name="adjust_direction"
                  label="调整方向"
                  rules={[{ required: true, message: '请选择调整方向' }]}
                >
                  <Select
                    options={[
                      { label: '增加', value: 'INCREASE' },
                      { label: '减少', value: 'DECREASE' }
                    ]}
                    data-testid="inventory-action-adjust-direction"
                  />
                </Form.Item>
              ) : null}
              <Form.Item
                name="qty"
                label="数量"
                rules={[{ required: true, message: '请输入数量' }]}
              >
                <InputNumber min={1} style={{ width: '100%' }} data-testid="inventory-action-qty" />
              </Form.Item>
              <Form.Item name="ref_type" label="关联类型">
                <Input placeholder="如 ORDER/RETURN" data-testid="inventory-action-ref-type" />
              </Form.Item>
              <Form.Item name="ref_id" label="关联单号">
                <Input placeholder="业务单号/请求 ID" data-testid="inventory-action-ref-id" />
              </Form.Item>
            </Form>
          </Space>
        ) : null}
      </Modal>
    </Space>
  );
};
