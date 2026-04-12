import {
  Button,
  Card,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';

import type {
  InboundDocCreatePayload,
  InboundDocSummary,
  InboundDocType,
  InboundDocStatus
} from '@/api/inbounds';
import { auditInboundDoc, createInboundDoc, listInbounds } from '@/api/inbounds';
import { listSuppliers } from '@/api/suppliers';
import { listSettings, type Setting } from '@/api/system';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  doc_no?: string;
  doc_type?: InboundDocType;
  status?: InboundDocStatus;
};

type CreateFormValues = {
  warehouse_id: string;
  doc_no: string;
  doc_type: InboundDocType;
  ref_type?: string;
  ref_id?: string;
  items: Array<{
    sku_id: string;
    planned_qty: number;
  }>;
};

const docTypeOptions: Array<{ label: string; value: InboundDocType }> = [
  { label: '采购入库', value: 'PURCHASE' },
  { label: '退货入库', value: 'RETURN' },
  { label: '其他入库', value: 'OTHER' }
];

const statusOptions: Array<{ label: string; value: InboundDocStatus }> = [
  { label: '草稿', value: 'DRAFT' },
  { label: '已审核', value: 'AUDITED' },
  { label: '收货中', value: 'RECEIVING' },
  { label: '已上架', value: 'PUTAWAY_DONE' },
  { label: '已取消', value: 'CANCELLED' }
];

const statusColors: Record<InboundDocStatus, string> = {
  DRAFT: 'default',
  AUDITED: 'gold',
  RECEIVING: 'blue',
  PUTAWAY_DONE: 'green',
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

export const InboundListPage = () => {
  const tenantId = useTenantId();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const canWrite = useCan('wms.inbounds.write');
  const tenantReady = isTenantSelected(tenantId);
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const [form] = Form.useForm<FilterValues>();
  const [createForm] = Form.useForm<CreateFormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [createOpen, setCreateOpen] = useState(false);

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'inbounds'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'inbounds'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const auditSettingQuery = useQuery({
    queryKey: ['settings', tenantId, 'inbound-audit'],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: 'inv.inbound_require_audit'
      }),
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

  const supplierMap = useMemo(() => {
    const entries = supplierQuery.data?.items ?? [];
    return new Map(entries.map((supplier) => [supplier.supplier_id, supplier.name]));
  }, [supplierQuery.data?.items]);

  const auditRequired = useMemo(
    () => parseSettingBoolean(auditSettingQuery.data?.[0]),
    [auditSettingQuery.data]
  );

  const query = useMemo(
    () => ({
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      doc_no: filters.doc_no?.trim() || undefined,
      doc_type: filters.doc_type,
      status: filters.status,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['inbounds', tenantId, query],
    queryFn: () => listInbounds(tenantId, query),
    enabled: tenantReady && inventoryFeature.enabled,
    retry: false
  });

  const createMutation = useMutation({
    mutationFn: (payload: InboundDocCreatePayload) => createInboundDoc(tenantId, payload),
    onSuccess: () => {
      message.success('入库单已创建');
      setCreateOpen(false);
      createForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['inbounds', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '创建入库单失败');
    }
  });

  const auditMutation = useMutation({
    mutationFn: (docId: string) => auditInboundDoc(tenantId, docId),
    onSuccess: () => {
      message.success('入库单已审核');
      queryClient.invalidateQueries({ queryKey: ['inbounds', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '审核入库单失败');
    }
  });

  const columns = useMemo<ColumnsType<InboundDocSummary>>(
    () => [
      {
        title: '入库单号',
        dataIndex: 'doc_no'
      },
      {
        title: '类型',
        dataIndex: 'doc_type',
        render: (value: InboundDocType) => docTypeOptions.find((item) => item.value === value)?.label
      },
      {
        title: '供应商',
        dataIndex: 'supplier_name',
        render: (_, record) =>
          record.supplier_name ||
          (record.supplier_id ? supplierMap.get(record.supplier_id) ?? record.supplier_id : '-')
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: InboundDocStatus) => (
          <Tag color={statusColors[value] || 'default'}>{value}</Tag>
        )
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (value) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              data-testid="inbound-detail-link"
              onClick={() => navigate(`/wms/inbounds/${record.doc_id}`)}
            >
              详情
            </Button>
            {auditRequired ? (
              <Button
                size="small"
                disabled={!canWrite || record.status !== 'DRAFT'}
                loading={auditMutation.isPending}
                onClick={() => auditMutation.mutate(record.doc_id)}
              >
                审核
              </Button>
            ) : null}
            <Button
              size="small"
              data-testid="inbound-receive-action"
              disabled={!canWrite}
              onClick={() => navigate(`/wms/inbounds/${record.doc_id}/receive`)}
            >
              收货
            </Button>
            <Button
              size="small"
              data-testid="inbound-inspect-action"
              disabled={!canWrite}
              onClick={() => navigate(`/wms/inbounds/${record.doc_id}/inspect`)}
            >
              质检
            </Button>
            <Button
              size="small"
              data-testid="inbound-putaway-action"
              disabled={!canWrite}
              onClick={() => navigate(`/wms/inbounds/${record.doc_id}/putaway`)}
            >
              上架
            </Button>
          </Space>
        )
      }
    ],
    [auditMutation.isPending, auditRequired, canWrite, navigate, supplierMap, warehouseMap]
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

  const handleCreate = async () => {
    const values = await createForm.validateFields();
    const payload: InboundDocCreatePayload = {
      warehouse_id: values.warehouse_id,
      doc_type: values.doc_type,
      doc_no: values.doc_no.trim(),
      ref_type: values.ref_type?.trim() || undefined,
      ref_id: values.ref_id?.trim() || undefined,
      items: values.items.map((item) => ({
        sku_id: item.sku_id.trim(),
        planned_qty: item.planned_qty
      }))
    };
    createMutation.mutate(payload);
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} data-testid="inbound-list-title" style={{ margin: 0 }}>
        收货入库
      </Typography.Title>
      <Button onClick={() => navigate('/wms/inbounds/receive-entry')}>快速收货</Button>
      <Button
        type="primary"
        disabled={!canWrite || !inventoryFeature.enabled}
        onClick={() => setCreateOpen(true)}
      >
        新建入库单
      </Button>
    </Space>
  );

  const header = (
    <Space direction="vertical" size={12}>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Space wrap>
            <Form.Item name="warehouse_id" label="仓库">
              <Select
                allowClear
                options={warehouseOptions}
                placeholder="选择仓库"
                loading={warehouseQuery.isFetching}
              />
            </Form.Item>
            <Form.Item name="doc_no" label="入库单号">
              <Input allowClear placeholder="输入入库单号" data-testid="inbound-doc-no-input" />
            </Form.Item>
            <Form.Item name="doc_type" label="类型">
              <Select allowClear options={docTypeOptions} placeholder="选择类型" />
            </Form.Item>
            <Form.Item name="status" label="状态">
              <Select allowClear options={statusOptions} placeholder="选择状态" />
            </Form.Item>
            <Form.Item label=" ">
              <Space>
                <Button type="primary" htmlType="submit" data-testid="inbound-search-submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Form.Item>
          </Space>
        </Form>
      </Card>
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
    </Space>
  );

  return (
    <>
      {inventoryFeature.enabled ? (
        <UiModule03MainTableSection<InboundDocSummary>
          toolbarLeft={toolbarLeft}
          header={header}
          rowKey={(record) => record.doc_id}
          columns={columns}
          data={listQuery.data?.items ?? []}
          loading={listQuery.isFetching}
          error={listQuery.error}
          onRetry={() => listQuery.refetch()}
          errorTitle="入库单加载失败"
          emptyText="暂无入库单"
          total={listQuery.data?.total ?? 0}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="inbounds.table.columns"
        />
      ) : null}
      <Modal
        title="新建入库单"
        open={createOpen}
        onCancel={() => setCreateOpen(false)}
        onOk={handleCreate}
        confirmLoading={createMutation.isPending}
        okText="创建"
        destroyOnClose
      >
        <Form
          form={createForm}
          layout="vertical"
          initialValues={{ doc_type: 'PURCHASE', items: [{ sku_id: '', planned_qty: 1 }] }}
        >
          <Form.Item
            name="warehouse_id"
            label="仓库"
            rules={[{ required: true, message: '请选择仓库' }]}
          >
            <Select options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
          <Form.Item
            name="doc_no"
            label="入库单号"
            rules={[{ required: true, message: '请输入入库单号' }]}
          >
            <Input placeholder="INB-0001" />
          </Form.Item>
          <Form.Item
            name="doc_type"
            label="入库类型"
            rules={[{ required: true, message: '请选择入库类型' }]}
          >
            <Select options={docTypeOptions} />
          </Form.Item>
          <Form.Item name="ref_type" label="来源类型">
            <Input placeholder="PURCHASE/RETURN/OTHER" />
          </Form.Item>
          <Form.Item name="ref_id" label="来源单号">
            <Input placeholder="关联单号" />
          </Form.Item>
          <Form.List name="items">
            {(fields, { add, remove }) => (
              <Space direction="vertical" style={{ width: '100%' }}>
                {fields.map((field, index) => (
                  <Space key={field.key} align="baseline" style={{ display: 'flex' }}>
                    <Form.Item
                      {...field}
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true, message: '请输入 SKU ID' }]}
                    >
                      <Input placeholder="SKU ID" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'planned_qty']}
                      rules={[{ required: true, message: '请输入数量' }]}
                    >
                      <InputNumber min={0} />
                    </Form.Item>
                    {fields.length > 1 ? (
                      <Button type="link" onClick={() => remove(field.name)}>
                        移除
                      </Button>
                    ) : null}
                    {index === fields.length - 1 ? (
                      <Button type="link" onClick={() => add({ sku_id: '', planned_qty: 1 })}>
                        添加
                      </Button>
                    ) : null}
                  </Space>
                ))}
              </Space>
            )}
          </Form.List>
        </Form>
      </Modal>
    </>
  );
};
