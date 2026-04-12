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

import type { FinanceExpense, FinanceExpenseCreatePayload } from '@/api/finance';
import {
  createFinanceExpense,
  deleteFinanceExpense,
  listFinanceExpenses,
  updateFinanceExpense
} from '@/api/finance';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
  category?: string;
};

type FormValues = Omit<FinanceExpenseCreatePayload, 'allocation_rule'> & {
  allocation_rule?: string;
};

const categoryOptions = [
  { label: '销售费用', value: 'SALES' },
  { label: '管理费用', value: 'ADMIN' },
  { label: '履约费用', value: 'FULFILLMENT' }
];

const statusOptions = [
  { label: '草稿', value: 'DRAFT' },
  { label: '已过账', value: 'POSTED' }
];

const formatRange = (start?: string | null, end?: string | null) => {
  if (!start && !end) {
    return '-';
  }
  return `${start ? dayjs(start).format('YYYY-MM-DD') : '不限'} ~ ${
    end ? dayjs(end).format('YYYY-MM-DD') : '不限'
  }`;
};

export const ExpenseAllocationPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('finance.write');
  const queryClient = useQueryClient();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editing, setEditing] = useState<FinanceExpense | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status || undefined,
      category: filters.category || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['finance-expenses', tenantId, query],
    queryFn: () => listFinanceExpenses(tenantId, query),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      let allocationRule: Record<string, unknown> | undefined;
      if (values.allocation_rule) {
        try {
          allocationRule = JSON.parse(values.allocation_rule);
        } catch {
          message.error('分摊规则需为 JSON');
          throw new Error('invalid allocation rule');
        }
      }
      const payload: FinanceExpenseCreatePayload = {
        name: values.name.trim(),
        category: values.category,
        amount_cent: values.amount_cent,
        currency: values.currency,
        allocation_rule: allocationRule,
        period_start: values.period_start,
        period_end: values.period_end,
        status: values.status
      };
      if (editing) {
        return updateFinanceExpense(tenantId, editing.expense_id, payload);
      }
      return createFinanceExpense(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '费用已更新' : '费用已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['finance-expenses', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存费用失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (expenseId: string) => deleteFinanceExpense(tenantId, expenseId),
    onSuccess: async () => {
      message.success('费用已删除');
      await queryClient.invalidateQueries({ queryKey: ['finance-expenses', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除费用失败');
    }
  });

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreate = () => {
    setEditing(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      status: 'DRAFT',
      currency: 'CNY'
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: FinanceExpense) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      name: record.name,
      category: record.category,
      amount_cent: record.amount_cent,
      currency: record.currency,
      allocation_rule: record.allocation_rule,
      period_start: record.period_start ?? undefined,
      period_end: record.period_end ?? undefined,
      status: record.status
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<FinanceExpense>>(
    () => [
      { title: '费用名称', dataIndex: 'name' },
      {
        title: '类别',
        dataIndex: 'category',
        render: (value) => categoryOptions.find((option) => option.value === value)?.label ?? value
      },
      { title: '金额', dataIndex: 'amount_cent', render: (value) => value.toLocaleString() },
      {
        title: '期间',
        render: (_, record) => formatRange(record.period_start, record.period_end)
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'POSTED' ? <Tag color="green">已过账</Tag> : <Tag color="orange">草稿</Tag>
      },
      {
        title: '更新',
        dataIndex: 'updated_at',
        render: (value) => dayjs(value).format('YYYY-MM-DD HH:mm')
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => openEdit(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除费用？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.expense_id)}
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
    [canWrite, deleteMutation]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          费用归集与分摊
        </Typography.Title>
        <Button type="primary" disabled={!canWrite} onClick={openCreate}>
          新建费用
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="inline" onFinish={handleSearch}>
          <Form.Item name="keyword" label="关键词">
            <Input allowClear placeholder="费用名称" style={{ width: 200 }} />
          </Form.Item>
          <Form.Item name="category" label="类别">
            <Select allowClear options={categoryOptions} placeholder="全部" style={{ width: 160 }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="全部" style={{ width: 140 }} />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button onClick={handleReset}>重置</Button>
            </Space>
          </Form.Item>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<FinanceExpense>
        header={header}
        rowKey="expense_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="费用归集加载失败"
        emptyText="暂无费用记录"
        columnSettingsKey="finance.expenses.table.columns"
      />
      <Drawer
        title={editing ? '编辑费用' : '新建费用'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
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
          <Form.Item name="name" label="费用名称" rules={[{ required: true, message: '请输入费用名称' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="category" label="类别" rules={[{ required: true, message: '请选择类别' }]}>
            <Select options={categoryOptions} />
          </Form.Item>
          <Form.Item name="amount_cent" label="金额(分)" rules={[{ required: true, message: '请输入金额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="currency" label="币种">
            <Input placeholder="CNY" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="period_start" label="开始日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="period_end" label="结束日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="allocation_rule" label="分摊规则(JSON)">
            <Input.TextArea rows={3} placeholder='{"rule":"by_channel"}' />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
