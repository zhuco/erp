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
  ProcurementReconciliation,
  ProcurementReconciliationCreatePayload,
  ProcurementReconciliationListQuery,
  ProcurementReconciliationStatus,
  ProcurementReconciliationUpdatePayload
} from '@/api/procurementReconciliations';
import {
  createProcurementReconciliation,
  deleteProcurementReconciliation,
  listProcurementReconciliations,
  updateProcurementReconciliation
} from '@/api/procurementReconciliations';
import { listSuppliers } from '@/api/suppliers';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  supplier_id?: string;
  status?: ProcurementReconciliationStatus;
  period_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type FormValues = {
  supplier_id?: string;
  status?: ProcurementReconciliationStatus;
  period_range?: [dayjs.Dayjs, dayjs.Dayjs];
  amount_cent: number;
  remark?: string;
};

const statusOptions: Array<{ label: string; value: ProcurementReconciliationStatus }> = [
  { label: 'OPEN', value: 'OPEN' },
  { label: 'CONFIRMED', value: 'CONFIRMED' }
];

const statusColors: Record<ProcurementReconciliationStatus, string> = {
  OPEN: 'orange',
  CONFIRMED: 'green'
};

const formatAmount = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const ProcurementReconciliationsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWrite = useCan('procurement_reconciliations.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingRecord, setEditingRecord] = useState<ProcurementReconciliation | null>(null);
  const [saving, setSaving] = useState(false);
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'procurement-reconciliations'],
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

  const query = useMemo<ProcurementReconciliationListQuery>(
    () => ({
      supplier_id: filters.supplier_id?.trim() || undefined,
      status: filters.status,
      period_from: filters.period_range?.[0]?.format('YYYY-MM-DD'),
      period_to: filters.period_range?.[1]?.format('YYYY-MM-DD'),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['procurement-reconciliations', tenantId, query],
    queryFn: () => listProcurementReconciliations(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (reconciliationId: string) =>
      deleteProcurementReconciliation(tenantId, reconciliationId),
    onSuccess: () => {
      message.success('对账单已删除');
      queryClient.invalidateQueries({ queryKey: ['procurement-reconciliations', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除对账单失败');
    }
  });

  const saveMutation = useMutation({
    mutationFn: async (payload: {
      reconciliationId?: string;
      body: ProcurementReconciliationCreatePayload | ProcurementReconciliationUpdatePayload;
    }) => {
      if (payload.reconciliationId) {
        return updateProcurementReconciliation(
          tenantId,
          payload.reconciliationId,
          payload.body
        );
      }
      return createProcurementReconciliation(
        tenantId,
        payload.body as ProcurementReconciliationCreatePayload
      );
    },
    onSuccess: () => {
      message.success('对账单已保存');
      setDrawerOpen(false);
      setEditingRecord(null);
      drawerForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['procurement-reconciliations', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存对账单失败');
    },
    onSettled: () => setSaving(false)
  });

  const handleEdit = (record: ProcurementReconciliation) => {
    setEditingRecord(record);
    setDrawerOpen(true);
    drawerForm.setFieldsValue({
      supplier_id: record.supplier_id,
      status: record.status as ProcurementReconciliationStatus,
      period_range: [dayjs(record.period_start), dayjs(record.period_end)],
      amount_cent: record.amount_cent,
      remark: record.remark ?? undefined
    });
  };

  const handleCreate = () => {
    setEditingRecord(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      const payload: ProcurementReconciliationCreatePayload | ProcurementReconciliationUpdatePayload =
        {
          supplier_id: values.supplier_id ?? '',
          period_start: values.period_range?.[0]?.format('YYYY-MM-DD') ?? '',
          period_end: values.period_range?.[1]?.format('YYYY-MM-DD') ?? '',
          amount_cent: values.amount_cent,
          status: values.status ?? undefined,
          remark: values.remark?.trim() || null
        };
      saveMutation.mutate({
        reconciliationId: editingRecord?.reconciliation_id,
        body: payload
      });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    }
  };

  const columns = useMemo<ColumnsType<ProcurementReconciliation>>(
    () => [
      { title: '对账单号', dataIndex: 'reconciliation_id' },
      {
        title: '供应商',
        dataIndex: 'supplier_id',
        render: (value: string, record) =>
          record.supplier_name ?? supplierMap.get(value) ?? value ?? '-'
      },
      { title: '期间开始', dataIndex: 'period_start' },
      { title: '期间结束', dataIndex: 'period_end' },
      {
        title: '对账金额',
        dataIndex: 'amount_cent',
        render: formatAmount
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ProcurementReconciliationStatus) => (
          <Tag color={statusColors[value] ?? 'default'}>{value}</Tag>
        )
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
              title="确认删除该对账单？"
              onConfirm={() => deleteMutation.mutate(record.reconciliation_id)}
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
          供应商对账
        </Typography.Title>
        <Button type="primary" onClick={handleCreate} disabled={!canWrite}>
          新建对账
        </Button>
      </Space>
      <Card>
        <Form
          form={filterForm}
          layout="inline"
          onFinish={(values) => setFilters(values)}
          initialValues={filters}
        >
          <Form.Item name="supplier_id" label="供应商">
            <Select options={supplierOptions} allowClear style={{ width: 200 }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear style={{ width: 160 }} />
          </Form.Item>
          <Form.Item name="period_range" label="对账期间">
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
      <UiModule03MainTableSection<ProcurementReconciliation>
        header={header}
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="供应商对账加载失败"
        emptyText="暂无对账单"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="procurement-reconciliations.table.columns"
        rowKey="reconciliation_id"
      />
      <Drawer
        title={editingRecord ? '编辑对账单' : '新建对账单'}
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
            name="supplier_id"
            label="供应商"
            rules={[{ required: true, message: '请选择供应商' }]}
          >
            <Select options={supplierOptions} allowClear />
          </Form.Item>
          <Form.Item
            name="period_range"
            label="对账期间"
            rules={[{ required: true, message: '请选择对账期间' }]}
          >
            <DatePicker.RangePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="amount_cent"
            label="对账金额(分)"
            rules={[{ required: true, message: '请输入对账金额' }]}
          >
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear />
          </Form.Item>
          <Form.Item name="remark" label="备注">
            <Input.TextArea rows={3} placeholder="备注信息" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
