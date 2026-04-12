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

import type { ArDoc, ArDocCreatePayload } from '@/api/finance';
import {
  createArDoc,
  deleteArDoc,
  listArDocs,
  updateArDoc,
  writeOffArDoc
} from '@/api/finance';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
};

type FormValues = ArDocCreatePayload;

const statusOptions = [
  { label: '未核销', value: 'OPEN' },
  { label: '已核销', value: 'WRITTEN_OFF' }
];

const renderDate = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD') : '-';

export const ArManagementPage = () => {
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
  const [editing, setEditing] = useState<ArDoc | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['ar-docs', tenantId, query],
    queryFn: () => listArDocs(tenantId, query),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: FormValues = {
        customer_id: values.customer_id?.trim() || undefined,
        ref_type: values.ref_type,
        ref_id: values.ref_id,
        amount_cent: values.amount_cent,
        currency: values.currency,
        status: values.status,
        due_date: values.due_date
      };
      if (editing) {
        return updateArDoc(tenantId, editing.ar_id, payload);
      }
      return createArDoc(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '应收已更新' : '应收已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['ar-docs', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存应收失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (arId: string) => deleteArDoc(tenantId, arId),
    onSuccess: async () => {
      message.success('应收已删除');
      await queryClient.invalidateQueries({ queryKey: ['ar-docs', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除应收失败');
    }
  });

  const writeOffMutation = useMutation({
    mutationFn: (arId: string) => writeOffArDoc(tenantId, arId),
    onSuccess: async () => {
      message.success('应收已核销');
      await queryClient.invalidateQueries({ queryKey: ['ar-docs', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '核销失败');
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
      status: 'OPEN',
      currency: 'CNY'
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: ArDoc) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      customer_id: record.customer_id ?? undefined,
      ref_type: record.ref_type,
      ref_id: record.ref_id,
      amount_cent: record.amount_cent,
      currency: record.currency,
      status: record.status,
      due_date: record.due_date ?? undefined
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<ArDoc>>(
    () => [
      {
        title: '客户',
        dataIndex: 'customer_name',
        render: (_, record) => record.customer_name ?? record.customer_id ?? '-'
      },
      { title: '引用类型', dataIndex: 'ref_type' },
      { title: '引用单号', dataIndex: 'ref_id' },
      {
        title: '金额',
        dataIndex: 'amount_cent',
        render: (value) => value.toLocaleString()
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'WRITTEN_OFF' ? <Tag color="green">已核销</Tag> : <Tag color="orange">待核销</Tag>
      },
      { title: '到期日', dataIndex: 'due_date', render: renderDate },
      {
        title: '核销时间',
        dataIndex: 'written_off_at',
        render: (value) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-')
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
              title="确认核销该应收？"
              okText="核销"
              cancelText="取消"
              onConfirm={() => writeOffMutation.mutate(record.ar_id)}
              disabled={!canWrite || record.status === 'WRITTEN_OFF'}
            >
              <Button size="small" disabled={!canWrite || record.status === 'WRITTEN_OFF'}>
                核销
              </Button>
            </Popconfirm>
            <Popconfirm
              title="确认删除该应收？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.ar_id)}
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
    [canWrite, deleteMutation, writeOffMutation]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          应收管理与核销
        </Typography.Title>
        <Button type="primary" disabled={!canWrite} onClick={openCreate}>
          新建应收
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="inline" onFinish={handleSearch}>
          <Form.Item name="keyword" label="关键词">
            <Input allowClear placeholder="客户/引用单号" style={{ width: 200 }} />
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
      <UiModule03MainTableSection<ArDoc>
        header={header}
        rowKey="ar_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="应收数据加载失败"
        emptyText="暂无应收记录"
        columnSettingsKey="finance.ar.table.columns"
      />
      <Drawer
        title={editing ? '编辑应收' : '新建应收'}
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
          <Form.Item name="customer_id" label="客户ID">
            <Input placeholder="客户ID" />
          </Form.Item>
          <Form.Item name="ref_type" label="引用类型" rules={[{ required: true, message: '请输入引用类型' }]}>
            <Input placeholder="订单/发票" />
          </Form.Item>
          <Form.Item name="ref_id" label="引用单号" rules={[{ required: true, message: '请输入引用单号' }]}>
            <Input placeholder="单号" />
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
          <Form.Item name="due_date" label="到期日">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
