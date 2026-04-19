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
  ProcurementSourcing,
  ProcurementSourcingCreatePayload,
  ProcurementSourcingListQuery,
  ProcurementSourcingStatus,
  ProcurementSourcingUpdatePayload
} from '@/api/procurementSourcing';
import {
  createProcurementSourcing,
  deleteProcurementSourcing,
  listProcurementSourcing,
  updateProcurementSourcing
} from '@/api/procurementSourcing';
import { listSuppliers } from '@/api/suppliers';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  supplier_id?: string;
  status?: ProcurementSourcingStatus;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type FormValues = {
  title: string;
  category?: string;
  supplier_id?: string;
  status?: ProcurementSourcingStatus;
  target_price_cent: number;
  remark?: string;
};

const statusOptions: Array<{ label: string; value: ProcurementSourcingStatus }> = [
  { label: 'OPEN', value: 'OPEN' },
  { label: 'REVIEWED', value: 'REVIEWED' }
];

const statusColors: Record<ProcurementSourcingStatus, string> = {
  OPEN: 'blue',
  REVIEWED: 'green'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatAmount = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const ProcurementSourcingPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWrite = useCan('procurement_sourcing.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingSourcing, setEditingSourcing] = useState<ProcurementSourcing | null>(null);
  const [saving, setSaving] = useState(false);
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'procurement-sourcing'],
    queryFn: () => listSuppliers(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const supplierOptions = useMemo(
    () =>
      supplierQuery.data?.items.map((supplier) => ({
        label: supplier.name,
        value: supplier.supplier_id
      })) ?? [],
    [supplierQuery.data?.items]
  );

  const supplierMap = useMemo(() => {
    const entries = supplierQuery.data?.items ?? [];
    return new Map(entries.map((supplier) => [supplier.supplier_id, supplier.name]));
  }, [supplierQuery.data?.items]);

  const query = useMemo<ProcurementSourcingListQuery>(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      supplier_id: filters.supplier_id?.trim() || undefined,
      status: filters.status,
      created_from: filters.created_range?.[0]?.toISOString(),
      created_to: filters.created_range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['procurement-sourcing', tenantId, query],
    queryFn: () => listProcurementSourcing(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (sourcingId: string) => deleteProcurementSourcing(tenantId, sourcingId),
    onSuccess: () => {
      message.success('寻源需求已删除');
      queryClient.invalidateQueries({ queryKey: ['procurement-sourcing', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除寻源需求失败');
    }
  });

  const saveMutation = useMutation({
    mutationFn: async (payload: {
      sourcingId?: string;
      body: ProcurementSourcingCreatePayload | ProcurementSourcingUpdatePayload;
    }) => {
      if (payload.sourcingId) {
        return updateProcurementSourcing(tenantId, payload.sourcingId, payload.body);
      }
      return createProcurementSourcing(
        tenantId,
        payload.body as ProcurementSourcingCreatePayload
      );
    },
    onSuccess: () => {
      message.success('寻源需求已保存');
      setDrawerOpen(false);
      setEditingSourcing(null);
      drawerForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['procurement-sourcing', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存寻源需求失败');
    },
    onSettled: () => setSaving(false)
  });

  const handleEdit = (record: ProcurementSourcing) => {
    setEditingSourcing(record);
    setDrawerOpen(true);
    drawerForm.setFieldsValue({
      title: record.title,
      category: record.category ?? undefined,
      supplier_id: record.supplier_id ?? undefined,
      status: record.status as ProcurementSourcingStatus,
      target_price_cent: record.target_price_cent,
      remark: record.remark ?? undefined
    });
  };

  const handleCreate = () => {
    setEditingSourcing(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      const payload: ProcurementSourcingCreatePayload | ProcurementSourcingUpdatePayload = {
        title: values.title?.trim(),
        category: values.category?.trim() || null,
        supplier_id: values.supplier_id ?? null,
        status: values.status ?? undefined,
        target_price_cent: values.target_price_cent,
        remark: values.remark?.trim() || null
      };
      saveMutation.mutate({ sourcingId: editingSourcing?.sourcing_id, body: payload });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    }
  };

  const columns = useMemo<ColumnsType<ProcurementSourcing>>(
    () => [
      { title: '寻源主题', dataIndex: 'title' },
      { title: '品类', dataIndex: 'category' },
      {
        title: '供应商',
        dataIndex: 'supplier_id',
        render: (value: string | null, record) =>
          record.supplier_name ?? supplierMap.get(value ?? '') ?? value ?? '-'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ProcurementSourcingStatus) => (
          <Tag color={statusColors[value] ?? 'default'}>{value}</Tag>
        )
      },
      {
        title: '目标价格',
        dataIndex: 'target_price_cent',
        render: formatAmount
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
            <Button size="small" onClick={() => handleEdit(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该寻源需求？"
              onConfirm={() => deleteMutation.mutate(record.sourcing_id)}
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
    [canWrite, deleteMutation, supplierMap]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          寻源与选品
        </Typography.Title>
        <Button type="primary" onClick={handleCreate} disabled={!canWrite}>
          新建需求
        </Button>
      </Space>
      <Card>
        <Form
          form={filterForm}
          layout="inline"
          onFinish={(values) => setFilters(values)}
          initialValues={filters}
        >
          <Form.Item name="keyword" label="关键词">
            <Input placeholder="主题/品类" allowClear />
          </Form.Item>
          <Form.Item name="supplier_id" label="供应商">
            <Select options={supplierOptions} allowClear style={{ width: 200 }} />
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
      <UiModule03MainTableSection<ProcurementSourcing>
        header={header}
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="寻源需求加载失败"
        emptyText="暂无寻源需求"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="procurement-sourcing.table.columns"
        rowKey="sourcing_id"
      />
      <Drawer
        title={editingSourcing ? '编辑寻源需求' : '新建寻源需求'}
        open={drawerOpen}
        width={600}
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
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="title"
            label="主题"
            rules={[{ required: true, message: '请输入主题' }]}
          >
            <Input placeholder="寻源主题" />
          </Form.Item>
          <Form.Item name="category" label="品类">
            <Input placeholder="品类" />
          </Form.Item>
          <Form.Item name="supplier_id" label="供应商">
            <Select options={supplierOptions} allowClear />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear />
          </Form.Item>
          <Form.Item
            name="target_price_cent"
            label="目标价格(分)"
            rules={[{ required: true, message: '请输入目标价格' }]}
          >
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={3} placeholder="备注信息" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
