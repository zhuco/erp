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

import type { ConsignmentSettlement, ConsignmentSettlementCreatePayload } from '@/api/finance';
import {
  createConsignmentSettlement,
  deleteConsignmentSettlement,
  listConsignmentSettlements,
  markConsignmentSettlement,
  updateConsignmentSettlement
} from '@/api/finance';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
};

type FormValues = ConsignmentSettlementCreatePayload;

const statusOptions = [
  { label: '草稿', value: 'DRAFT' },
  { label: '已结算', value: 'SETTLED' }
];

const formatDate = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD') : '-');

export const ConsignmentPage = () => {
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
  const [editing, setEditing] = useState<ConsignmentSettlement | null>(null);

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
    queryKey: ['consignments', tenantId, query],
    queryFn: () => listConsignmentSettlements(tenantId, query),
    enabled: tenantReady,
    placeholderData: (previousData) => previousData
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      const payload: FormValues = {
        partner_name: values.partner_name.trim(),
        agreement_no: values.agreement_no?.trim() || undefined,
        period_start: values.period_start,
        period_end: values.period_end,
        sales_amount_cent: values.sales_amount_cent,
        commission_rate: values.commission_rate,
        commission_cent: values.commission_cent,
        status: values.status,
        notes: values.notes?.trim()
      };
      if (editing) {
        return updateConsignmentSettlement(tenantId, editing.settlement_id, payload);
      }
      return createConsignmentSettlement(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '代销结算已更新' : '代销结算已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['consignments', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存代销结算失败');
    }
  });

  const deleteMutation = useMutation({
    mutationFn: (settlementId: string) => deleteConsignmentSettlement(tenantId, settlementId),
    onSuccess: async () => {
      message.success('代销结算已删除');
      await queryClient.invalidateQueries({ queryKey: ['consignments', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除代销结算失败');
    }
  });

  const settleMutation = useMutation({
    mutationFn: (settlementId: string) => markConsignmentSettlement(tenantId, settlementId),
    onSuccess: async () => {
      message.success('代销结算已确认');
      await queryClient.invalidateQueries({ queryKey: ['consignments', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '确认代销结算失败');
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
    drawerForm.setFieldsValue({ status: 'DRAFT' });
    setDrawerOpen(true);
  };

  const openEdit = (record: ConsignmentSettlement) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      partner_name: record.partner_name,
      agreement_no: record.agreement_no ?? undefined,
      period_start: record.period_start ?? undefined,
      period_end: record.period_end ?? undefined,
      sales_amount_cent: record.sales_amount_cent,
      commission_rate: record.commission_rate,
      commission_cent: record.commission_cent,
      status: record.status,
      notes: record.notes ?? undefined
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<ConsignmentSettlement>>(
    () => [
      { title: '合作方', dataIndex: 'partner_name' },
      { title: '协议号', dataIndex: 'agreement_no', render: (value) => value ?? '-' },
      {
        title: '期间',
        render: (_, record) => `${formatDate(record.period_start)} ~ ${formatDate(record.period_end)}`
      },
      {
        title: '销售额',
        dataIndex: 'sales_amount_cent',
        render: (value) => value.toLocaleString()
      },
      { title: '扣点', dataIndex: 'commission_rate' },
      {
        title: '结算金额',
        dataIndex: 'commission_cent',
        render: (value) => value.toLocaleString()
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) => (value === 'SETTLED' ? <Tag color="green">已结算</Tag> : <Tag>草稿</Tag>)
      },
      { title: '结算时间', dataIndex: 'settled_at', render: formatDate },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => openEdit(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认结算？"
              okText="确认"
              cancelText="取消"
              onConfirm={() => settleMutation.mutate(record.settlement_id)}
              disabled={!canWrite || record.status === 'SETTLED'}
            >
              <Button size="small" disabled={!canWrite || record.status === 'SETTLED'}>
                确认结算
              </Button>
            </Popconfirm>
            <Popconfirm
              title="确认删除代销结算？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.settlement_id)}
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
    [canWrite, deleteMutation, settleMutation]
  );

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        代销结算管理
      </Typography.Title>
      <Typography.Text type="secondary">代销结算单的创建、审核与结算状态跟踪</Typography.Text>
    </Space>
  );

  const toolbarRight = (
    <Space>
      <Button loading={listQuery.isFetching} onClick={() => void listQuery.refetch()}>
        刷新
      </Button>
      <Button onClick={() => message.info('代销结算导出能力待接入')}>导出</Button>
      <Button type="primary" disabled={!canWrite} onClick={openCreate}>
        新建结算单
      </Button>
    </Space>
  );

  const filterActionBar = (
    <Form form={form} layout="inline" onFinish={handleSearch}>
      <Form.Item name="keyword" label="关键词">
        <Input allowClear placeholder="合作方/协议号" style={{ width: 200 }} />
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
  );

  return (
    <>
      <UiModule03MainTableSection<ConsignmentSettlement>
        dataTestId="finance-consignment-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        actionBar={filterActionBar}
        rowKey="settlement_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="代销结算加载失败"
        emptyText="暂无结算记录"
        columnSettingsKey="finance.consignment.table.columns"
        showColumnSettings
      />
      <Drawer
        title={editing ? '编辑结算单' : '新建结算单'}
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
          <Form.Item name="partner_name" label="合作方" rules={[{ required: true, message: '请输入合作方' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="agreement_no" label="协议号">
            <Input />
          </Form.Item>
          <Form.Item name="period_start" label="开始日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="period_end" label="结束日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="sales_amount_cent" label="销售额(分)" rules={[{ required: true, message: '请输入销售额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="commission_rate" label="扣点(%)" rules={[{ required: true, message: '请输入扣点' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="commission_cent" label="结算金额(分)" rules={[{ required: true, message: '请输入结算金额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="notes" label="备注">
            <Input.TextArea rows={3} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
