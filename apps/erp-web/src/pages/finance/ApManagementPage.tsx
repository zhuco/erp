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
  Tabs,
  Tag,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { ApDoc, ApPaymentPlan, ApPaymentPlanCreatePayload } from '@/api/finance';
import {
  createApPaymentPlan,
  deleteApPaymentPlan,
  listApDocs,
  listApPaymentPlans,
  markApPaymentPlanPaid,
  updateApPaymentPlan
} from '@/api/finance';
import { UiModule03MainTableSection, UiModule07SettingsLayout } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type DocFilterValues = {
  status?: string;
  supplier_id?: string;
};

type PlanFilterValues = {
  status?: string;
  supplier_id?: string;
};

type PlanFormValues = ApPaymentPlanCreatePayload;

const planStatusOptions = [
  { label: '待计划', value: 'PLANNED' },
  { label: '已批准', value: 'APPROVED' },
  { label: '已支付', value: 'PAID' }
];

const docStatusOptions = [
  { label: '待支付', value: 'OPEN' },
  { label: '已支付', value: 'PAID' },
  { label: '已关单', value: 'CLOSED' }
];

const settlementTag = (status?: string | null) => {
  if (status === 'SETTLED') {
    return <Tag color="green">已结清</Tag>;
  }
  if (status === 'SETTLING') {
    return <Tag color="orange">结算中</Tag>;
  }
  return <Tag>未结清</Tag>;
};

const formatDate = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD') : '-');

export const ApManagementPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('finance.write');
  const queryClient = useQueryClient();
  const [docFilters, setDocFilters] = useState<DocFilterValues>({});
  const [docPagination, setDocPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [planFilters, setPlanFilters] = useState<PlanFilterValues>({});
  const [planPagination, setPlanPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [docForm] = Form.useForm<DocFilterValues>();
  const [planForm] = Form.useForm<PlanFilterValues>();
  const [drawerForm] = Form.useForm<PlanFormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<ApPaymentPlan | null>(null);

  const docQuery = useMemo(
    () => ({
      status: docFilters.status || undefined,
      supplier_id: docFilters.supplier_id?.trim() || undefined,
      offset: ((docPagination.current ?? 1) - 1) * (docPagination.pageSize ?? 20),
      limit: docPagination.pageSize ?? 20
    }),
    [docFilters, docPagination.current, docPagination.pageSize]
  );

  const planQuery = useMemo(
    () => ({
      status: planFilters.status || undefined,
      supplier_id: planFilters.supplier_id?.trim() || undefined,
      offset: ((planPagination.current ?? 1) - 1) * (planPagination.pageSize ?? 20),
      limit: planPagination.pageSize ?? 20
    }),
    [planFilters, planPagination.current, planPagination.pageSize]
  );

  const docsQuery = useQuery({
    queryKey: ['ap-docs', tenantId, docQuery],
    queryFn: () => listApDocs(tenantId, docQuery),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const plansQuery = useQuery({
    queryKey: ['ap-plans', tenantId, planQuery],
    queryFn: () => listApPaymentPlans(tenantId, planQuery),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: PlanFormValues = {
        ap_id: values.ap_id,
        supplier_id: values.supplier_id?.trim() || undefined,
        planned_date: values.planned_date,
        amount_cent: values.amount_cent,
        status: values.status,
        notes: values.notes?.trim()
      };
      if (editing) {
        return updateApPaymentPlan(tenantId, editing.plan_id, payload);
      }
      return createApPaymentPlan(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '付款计划已更新' : '付款计划已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['ap-plans', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存付款计划失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (planId: string) => deleteApPaymentPlan(tenantId, planId),
    onSuccess: async () => {
      message.success('付款计划已删除');
      await queryClient.invalidateQueries({ queryKey: ['ap-plans', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除付款计划失败');
    }
  });

  const markPaidMutation = useMutation({
    mutationFn: (planId: string) => markApPaymentPlanPaid(tenantId, planId),
    onSuccess: async () => {
      message.success('付款计划已标记完成');
      await queryClient.invalidateQueries({ queryKey: ['ap-plans', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '标记付款计划失败');
    }
  });

  const handleDocSearch = (values: DocFilterValues) => {
    setDocFilters(values);
    setDocPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handlePlanSearch = (values: PlanFilterValues) => {
    setPlanFilters(values);
    setPlanPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleDocReset = () => {
    docForm.resetFields();
    setDocFilters({});
    setDocPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handlePlanReset = () => {
    planForm.resetFields();
    setPlanFilters({});
    setPlanPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreate = () => {
    setEditing(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      status: 'PLANNED'
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: ApPaymentPlan) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      ap_id: record.ap_id,
      supplier_id: record.supplier_id ?? undefined,
      planned_date: record.planned_date ?? undefined,
      amount_cent: record.amount_cent,
      status: record.status,
      notes: record.notes ?? undefined
    });
    setDrawerOpen(true);
  };

  const docColumns = useMemo<ColumnsType<ApDoc>>(
    () => [
      { title: '供应商', dataIndex: 'supplier_name' },
      { title: '引用类型', dataIndex: 'ref_type' },
      { title: '引用单号', dataIndex: 'ref_id' },
      { title: '金额', dataIndex: 'amount_cent', render: (value) => value.toLocaleString() },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'PAID' ? (
            <Tag color="green">已支付</Tag>
          ) : value === 'CLOSED' ? (
            <Tag color="default">已关单</Tag>
          ) : (
            <Tag>待支付</Tag>
          )
      },
      {
        title: '结算状态',
        dataIndex: 'settlement_status',
        render: (_, record) => settlementTag(record.settlement_status)
      },
      { title: '付款时间', dataIndex: 'paid_at', render: formatDate }
    ],
    []
  );

  const planColumns = useMemo<ColumnsType<ApPaymentPlan>>(
    () => [
      { title: '应付单据', dataIndex: 'ap_id' },
      { title: '供应商', dataIndex: 'supplier_name' },
      { title: '计划日期', dataIndex: 'planned_date', render: formatDate },
      { title: '金额', dataIndex: 'amount_cent', render: (value) => value.toLocaleString() },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'PAID' ? (
            <Tag color="green">已支付</Tag>
          ) : value === 'APPROVED' ? (
            <Tag color="blue">已批准</Tag>
          ) : (
            <Tag color="orange">待计划</Tag>
          )
      },
      { title: '付款时间', dataIndex: 'paid_at', render: formatDate },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => openEdit(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认标记为已支付？"
              okText="确认"
              cancelText="取消"
              onConfirm={() => markPaidMutation.mutate(record.plan_id)}
              disabled={!canWrite || record.status === 'PAID'}
            >
              <Button size="small" disabled={!canWrite || record.status === 'PAID'}>
                标记支付
              </Button>
            </Popconfirm>
            <Popconfirm
              title="确认删除付款计划？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.plan_id)}
              disabled={!canWrite}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation, markPaidMutation]
  );

  const docsHeader = (
    <Card>
      <Form form={docForm} layout="inline" onFinish={handleDocSearch}>
        <Form.Item name="supplier_id" label="供应商ID">
          <Input allowClear placeholder="供应商ID" style={{ width: 200 }} />
        </Form.Item>
        <Form.Item name="status" label="状态">
          <Select allowClear options={docStatusOptions} placeholder="全部" style={{ width: 160 }} />
        </Form.Item>
        <Form.Item>
          <Space>
            <Button type="primary" htmlType="submit">
              查询
            </Button>
            <Button onClick={handleDocReset}>重置</Button>
          </Space>
        </Form.Item>
      </Form>
    </Card>
  );

  const plansHeader = (
    <Space align="center" style={{ justifyContent: 'space-between', width: '100%' }}>
      <Card style={{ flex: 1 }}>
        <Form form={planForm} layout="inline" onFinish={handlePlanSearch}>
          <Form.Item name="supplier_id" label="供应商ID">
            <Input allowClear placeholder="供应商ID" style={{ width: 200 }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={planStatusOptions} placeholder="全部" style={{ width: 160 }} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button onClick={handlePlanReset}>重置</Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>
      <Button type="primary" disabled={!canWrite} onClick={openCreate}>
        新建计划
      </Button>
    </Space>
  );

  return (
    <UiModule07SettingsLayout title="应付管理与付款计划">
      <Tabs
        items={[
          {
            key: 'docs',
            label: '应付单据',
            children: (
              <UiModule03MainTableSection<ApDoc>
                header={docsHeader}
                rowKey="ap_id"
                columns={docColumns}
                data={docsQuery.data?.items ?? []}
                loading={docsQuery.isFetching}
                total={docsQuery.data?.total ?? 0}
                pagination={docPagination}
                onPaginationChange={setDocPagination}
                error={docsQuery.error}
                onRetry={() => docsQuery.refetch()}
                columnSettingsKey="finance.ap.docs.table.columns"
              />
            )
          },
          {
            key: 'plans',
            label: '付款计划',
            children: (
              <UiModule03MainTableSection<ApPaymentPlan>
                header={plansHeader}
                rowKey="plan_id"
                columns={planColumns}
                data={plansQuery.data?.items ?? []}
                loading={plansQuery.isFetching}
                total={plansQuery.data?.total ?? 0}
                pagination={planPagination}
                onPaginationChange={setPlanPagination}
                error={plansQuery.error}
                onRetry={() => plansQuery.refetch()}
                columnSettingsKey="finance.ap.plans.table.columns"
              />
            )
          }
        ]}
      />
      <Drawer
        title={editing ? '编辑付款计划' : '新建付款计划'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saveMutation.isPending} onClick={() => saveMutation.mutate()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item name="ap_id" label="应付单据ID" rules={[{ required: true, message: '请输入应付单据ID' }]}>
            <Input placeholder="ap_id" />
          </Form.Item>
          <Form.Item name="supplier_id" label="供应商ID">
            <Input placeholder="supplier_id" />
          </Form.Item>
          <Form.Item name="planned_date" label="计划日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="amount_cent" label="金额(分)" rules={[{ required: true, message: '请输入金额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={planStatusOptions} />
          </Form.Item>
          <Form.Item name="notes" label="备注">
            <Input.TextArea rows={3} />
          </Form.Item>
        </Form>
      </Drawer>
    </UiModule07SettingsLayout>
  );
};
