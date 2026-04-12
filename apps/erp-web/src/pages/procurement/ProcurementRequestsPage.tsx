import {
  Button,
  Card,
  DatePicker,
  Drawer,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  ProcurementRequestCreatePayload,
  ProcurementRequestDetail,
  ProcurementRequestItemPayload,
  ProcurementRequestListQuery,
  ProcurementRequestStatus,
  ProcurementRequest,
  ProcurementRequestUpdatePayload
} from '@/api/procurementRequests';
import {
  createProcurementRequest,
  deleteProcurementRequest,
  getProcurementRequestDetail,
  listProcurementRequests,
  updateProcurementRequest
} from '@/api/procurementRequests';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  department?: string;
  requester?: string;
  status?: ProcurementRequestStatus;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type FormValues = {
  department?: string;
  requester?: string;
  status?: ProcurementRequestStatus;
  expected_at?: dayjs.Dayjs;
  remark?: string;
  items: ProcurementRequestItemPayload[];
};

const statusOptions: Array<{ label: string; value: ProcurementRequestStatus }> = [
  { label: 'DRAFT', value: 'DRAFT' },
  { label: 'APPROVED', value: 'APPROVED' },
  { label: 'REJECTED', value: 'REJECTED' }
];

const statusColors: Record<ProcurementRequestStatus, string> = {
  DRAFT: 'default',
  APPROVED: 'green',
  REJECTED: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatAmount = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const ProcurementRequestsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWrite = useCan('procurement_requests.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingRequest, setEditingRequest] = useState<ProcurementRequestDetail | null>(null);
  const [saving, setSaving] = useState(false);
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();

  const query = useMemo<ProcurementRequestListQuery>(
    () => ({
      department: filters.department?.trim() || undefined,
      requester: filters.requester?.trim() || undefined,
      status: filters.status,
      created_from: filters.created_range?.[0]?.toISOString(),
      created_to: filters.created_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['procurement-requests', tenantId, query],
    queryFn: () => listProcurementRequests(tenantId, query),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (requestId: string) => deleteProcurementRequest(tenantId, requestId),
    onSuccess: () => {
      message.success('采购申请已删除');
      queryClient.invalidateQueries({ queryKey: ['procurement-requests', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除采购申请失败');
    }
  });

  const saveMutation = useMutation({
    mutationFn: async (payload: {
      requestId?: string;
      body: ProcurementRequestCreatePayload | ProcurementRequestUpdatePayload;
    }) => {
      if (payload.requestId) {
        return updateProcurementRequest(tenantId, payload.requestId, payload.body);
      }
      return createProcurementRequest(tenantId, payload.body as ProcurementRequestCreatePayload);
    },
    onSuccess: () => {
      message.success('采购申请已保存');
      setDrawerOpen(false);
      setEditingRequest(null);
      drawerForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['procurement-requests', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存采购申请失败');
    },
    onSettled: () => setSaving(false)
  });

  const handleEdit = async (requestId: string) => {
    try {
      const detail = await getProcurementRequestDetail(tenantId, requestId);
      setEditingRequest(detail);
      setDrawerOpen(true);
      drawerForm.setFieldsValue({
        department: detail.request.department ?? undefined,
        requester: detail.request.requester ?? undefined,
        status: detail.request.status as ProcurementRequestStatus,
        expected_at: detail.request.expected_at
          ? dayjs(detail.request.expected_at)
          : undefined,
        remark: detail.request.remark ?? undefined,
        items: detail.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          price_cent: item.price_cent
        }))
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载采购申请失败');
    }
  };

  const handleCreate = () => {
    setEditingRequest(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      items: [{ sku_id: '', qty: 1, price_cent: 0 }]
    });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      const payload: ProcurementRequestCreatePayload | ProcurementRequestUpdatePayload = {
        department: values.department?.trim() || null,
        requester: values.requester?.trim() || null,
        status: values.status ?? undefined,
        expected_at: values.expected_at ? values.expected_at.toISOString() : null,
        remark: values.remark?.trim() || null,
        items: values.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          price_cent: item.price_cent
        }))
      };
      saveMutation.mutate({ requestId: editingRequest?.request.request_id, body: payload });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    }
  };

  const columns = useMemo<ColumnsType<ProcurementRequest>>(
    () => [
      { title: '申请单号', dataIndex: 'request_id' },
      { title: '部门', dataIndex: 'department' },
      { title: '申请人', dataIndex: 'requester' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ProcurementRequestStatus) => (
          <Tag color={statusColors[value] ?? 'default'}>{value}</Tag>
        )
      },
      {
        title: '预算金额',
        dataIndex: 'budget_cent',
        render: formatAmount
      },
      {
        title: '期望到货',
        dataIndex: 'expected_at',
        render: formatDateTime
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: formatDateTime
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" onClick={() => handleEdit(record.request_id)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该采购申请？"
              onConfirm={() => deleteMutation.mutate(record.request_id)}
              okButtonProps={{ disabled: !canWrite }}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          采购申请
        </Typography.Title>
        <Button type="primary" onClick={handleCreate} disabled={!canWrite}>
          新建申请
        </Button>
      </Space>
      <Card>
        <Form
          form={filterForm}
          layout="inline"
          onFinish={(values) => setFilters(values)}
          initialValues={filters}
        >
          <Form.Item name="department" label="部门">
            <Input placeholder="部门" allowClear />
          </Form.Item>
          <Form.Item name="requester" label="申请人">
            <Input placeholder="申请人" allowClear />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear style={{ width: 160 }} />
          </Form.Item>
          <Form.Item name="created_range" label="创建时间">
            <DatePicker.RangePicker />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button
                onClick={() => {
                  filterForm.resetFields();
                  setFilters({});
                }}
              >
                重置
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<ProcurementRequest>
        header={header}
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="采购申请加载失败"
        emptyText="暂无采购申请"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="procurement-requests.table.columns"
        rowKey="request_id"
      />
      <Drawer
        title={editingRequest ? '编辑采购申请' : '新建采购申请'}
        open={drawerOpen}
        width={720}
        onClose={() => setDrawerOpen(false)}
        extra={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave} disabled={!canWrite}>
              保存
            </Button>
          </Space>
        }
      >
        <Form
          form={drawerForm}
          layout="vertical"
          initialValues={{ items: [{ sku_id: '', qty: 1, price_cent: 0 }] }}
        >
          <Form.Item name="department" label="部门">
            <Input placeholder="部门" />
          </Form.Item>
          <Form.Item name="requester" label="申请人">
            <Input placeholder="申请人" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear />
          </Form.Item>
          <Form.Item name="expected_at" label="期望到货">
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={3} placeholder="备注信息" />
          </Form.Item>
          <Form.List name="items">
            {(fields, { add, remove }) => (
              <Space direction="vertical" style={{ width: '100%' }}>
                {fields.map((field, index) => (
                  <Space key={field.key} align="baseline" style={{ display: 'flex' }}>
                    <Form.Item
                      {...field}
                      name={[field.name, 'sku_id']}
                      rules={[{ required: true, message: 'SKU必填' }]}
                    >
                      <Input placeholder="SKU ID" />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'qty']}
                      rules={[{ required: true, message: '数量必填' }]}
                    >
                      <InputNumber min={1} />
                    </Form.Item>
                    <Form.Item
                      {...field}
                      name={[field.name, 'price_cent']}
                      rules={[{ required: true, message: '单价必填' }]}
                    >
                      <InputNumber min={0} />
                    </Form.Item>
                    {fields.length > 1 ? (
                      <Button type="link" onClick={() => remove(field.name)}>
                        删除
                      </Button>
                    ) : null}
                    {index === fields.length - 1 ? (
                      <Button type="link" onClick={() => add({ sku_id: '', qty: 1, price_cent: 0 })}>
                        添加
                      </Button>
                    ) : null}
                  </Space>
                ))}
              </Space>
            )}
          </Form.List>
        </Form>
      </Drawer>
    </>
  );
};
