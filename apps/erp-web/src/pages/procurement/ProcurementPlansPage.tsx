import {
  Button,
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
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  ProcurementPlan,
  ProcurementPlanCreatePayload,
  ProcurementPlanDetail,
  ProcurementPlanItemPayload,
  ProcurementPlanListQuery,
  ProcurementPlanStatus,
  ProcurementPlanUpdatePayload
} from '@/api/procurementPlans';
import {
  createProcurementPlan,
  deleteProcurementPlan,
  getProcurementPlanDetail,
  listProcurementPlans,
  updateProcurementPlan
} from '@/api/procurementPlans';
import { listSuppliers } from '@/api/suppliers';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  supplier_id?: string;
  status?: ProcurementPlanStatus;
  created_range?: [dayjs.Dayjs, dayjs.Dayjs];
};

type FormValues = {
  request_id?: string;
  supplier_id?: string;
  status?: ProcurementPlanStatus;
  planned_at?: dayjs.Dayjs;
  expected_at?: dayjs.Dayjs;
  remark?: string;
  items: ProcurementPlanItemPayload[];
};

const statusOptions: Array<{ label: string; value: ProcurementPlanStatus }> = [
  { label: 'DRAFT', value: 'DRAFT' },
  { label: 'APPROVED', value: 'APPROVED' },
  { label: 'RELEASED', value: 'RELEASED' }
];

const statusColors: Record<ProcurementPlanStatus, string> = {
  DRAFT: 'default',
  APPROVED: 'blue',
  RELEASED: 'green'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatAmount = (value: number) => `¥${(value / 100).toFixed(2)}`;

export const ProcurementPlansPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWrite = useCan('procurement_plans.write');
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingPlan, setEditingPlan] = useState<ProcurementPlanDetail | null>(null);
  const [saving, setSaving] = useState(false);
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();

  const supplierQuery = useQuery({
    queryKey: ['suppliers', tenantId, 'procurement-plans'],
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

  const query = useMemo<ProcurementPlanListQuery>(
    () => ({
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
    queryKey: ['procurement-plans', tenantId, query],
    queryFn: () => listProcurementPlans(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (planId: string) => deleteProcurementPlan(tenantId, planId),
    onSuccess: () => {
      message.success('采购计划已删除');
      queryClient.invalidateQueries({ queryKey: ['procurement-plans', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除采购计划失败');
    }
  });

  const saveMutation = useMutation({
    mutationFn: async (payload: {
      planId?: string;
      body: ProcurementPlanCreatePayload | ProcurementPlanUpdatePayload;
    }) => {
      if (payload.planId) {
        return updateProcurementPlan(tenantId, payload.planId, payload.body);
      }
      return createProcurementPlan(tenantId, payload.body as ProcurementPlanCreatePayload);
    },
    onSuccess: () => {
      message.success('采购计划已保存');
      setDrawerOpen(false);
      setEditingPlan(null);
      drawerForm.resetFields();
      queryClient.invalidateQueries({ queryKey: ['procurement-plans', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存采购计划失败');
    },
    onSettled: () => setSaving(false)
  });

  const handleEdit = async (planId: string) => {
    try {
      const detail = await getProcurementPlanDetail(tenantId, planId);
      setEditingPlan(detail);
      setDrawerOpen(true);
      drawerForm.setFieldsValue({
        request_id: detail.plan.request_id ?? undefined,
        supplier_id: detail.plan.supplier_id ?? undefined,
        status: detail.plan.status as ProcurementPlanStatus,
        planned_at: detail.plan.planned_at ? dayjs(detail.plan.planned_at) : undefined,
        expected_at: detail.plan.expected_at ? dayjs(detail.plan.expected_at) : undefined,
        remark: detail.plan.remark ?? undefined,
        items: detail.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          price_cent: item.price_cent
        }))
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '加载采购计划失败');
    }
  };

  const handleCreate = () => {
    setEditingPlan(null);
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
      const payload: ProcurementPlanCreatePayload | ProcurementPlanUpdatePayload = {
        request_id: values.request_id?.trim() || null,
        supplier_id: values.supplier_id ?? null,
        status: values.status ?? undefined,
        planned_at: values.planned_at ? values.planned_at.toISOString() : null,
        expected_at: values.expected_at ? values.expected_at.toISOString() : null,
        remark: values.remark?.trim() || null,
        items: values.items.map((item) => ({
          sku_id: item.sku_id,
          qty: item.qty,
          price_cent: item.price_cent
        }))
      };
      saveMutation.mutate({ planId: editingPlan?.plan.plan_id, body: payload });
    } catch (err) {
      if (err instanceof Error) {
        message.error(err.message);
      }
    }
  };

  const columns = useMemo<ColumnsType<ProcurementPlan>>(
    () => [
      { title: '计划编号', dataIndex: 'plan_id' },
      {
        title: '供应商',
        dataIndex: 'supplier_id',
        render: (value: string | null, record) =>
          record.supplier_name ?? supplierMap.get(value ?? '') ?? value ?? '-'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ProcurementPlanStatus) => (
          <Tag color={statusColors[value] ?? 'default'}>{value}</Tag>
        )
      },
      {
        title: '计划金额',
        dataIndex: 'total_cent',
        render: formatAmount
      },
      {
        title: '计划时间',
        dataIndex: 'planned_at',
        render: formatDateTime
      },
      {
        title: '预计到货',
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
            <Button size="small" onClick={() => handleEdit(record.plan_id)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该采购计划？"
              onConfirm={() => deleteMutation.mutate(record.plan_id)}
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

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        采购计划
      </Typography.Title>
    </Space>
  );

  const toolbarRight = (
    <Button type="primary" onClick={handleCreate} disabled={!canWrite}>
    新建计划
    </Button>
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
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
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<ProcurementPlan>
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        header={header}
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="采购计划加载失败"
        emptyText="暂无采购计划"
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        dataTestId="procurement-plans-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        columnSettingsKey="procurement-plans.table.columns"
        rowKey="plan_id"
      />
      <Drawer
        title={editingPlan ? '编辑采购计划' : '新建采购计划'}
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
          <Form.Item name="request_id" label="关联申请单">
            <Input placeholder="申请单ID" />
          </Form.Item>
          <Form.Item name="supplier_id" label="供应商">
            <Select options={supplierOptions} placeholder="选择供应商" allowClear />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} allowClear />
          </Form.Item>
          <Form.Item name="planned_at" label="计划时间">
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="expected_at" label="预计到货">
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
