import {
  Button,
  Card,
  DatePicker,
  Drawer,
  Form,
  Input,
  InputNumber,
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

import type { InventoryCostingRun, InventoryCostingRunCreatePayload } from '@/api/finance';
import {
  createInventoryCostingRun,
  listInventoryCostingRuns,
  updateInventoryCostingRun
} from '@/api/finance';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  status?: string;
  method?: string;
};

type FormValues = Omit<InventoryCostingRunCreatePayload, 'result_json'> & {
  result_json?: string;
};

const statusOptions = [
  { label: '草稿', value: 'DRAFT' },
  { label: '运行中', value: 'RUNNING' },
  { label: '完成', value: 'DONE' }
];

const methodOptions = [
  { label: '移动平均', value: 'AVG' },
  { label: 'FIFO', value: 'FIFO' }
];

const formatRange = (start?: string | null, end?: string | null) => {
  if (!start && !end) {
    return '-';
  }
  return `${start ? dayjs(start).format('YYYY-MM-DD') : '不限'} ~ ${
    end ? dayjs(end).format('YYYY-MM-DD') : '不限'
  }`;
};

export const CostingPage = () => {
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
  const [editing, setEditing] = useState<InventoryCostingRun | null>(null);

  const query = useMemo(
    () => ({
      status: filters.status || undefined,
      method: filters.method || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const listQuery = useQuery({
    queryKey: ['costing-runs', tenantId, query],
    queryFn: () => listInventoryCostingRuns(tenantId, query),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const saveMutation = useMutation({
    mutationFn: async () => {
      const values = await drawerForm.validateFields();
      let resultJson: Record<string, unknown> | undefined;
      if (values.result_json) {
        try {
          resultJson = JSON.parse(values.result_json);
        } catch {
          message.error('结果 JSON 格式不正确');
          throw new Error('invalid json');
        }
      }
      const payload: InventoryCostingRunCreatePayload = {
        method: values.method,
        period_start: values.period_start,
        period_end: values.period_end,
        status: values.status,
        variance_cent: values.variance_cent,
        result_json: resultJson
      };
      if (editing) {
        return updateInventoryCostingRun(tenantId, editing.costing_id, payload);
      }
      return createInventoryCostingRun(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editing ? '成本核算已更新' : '成本核算已创建');
      setDrawerOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['costing-runs', tenantId] });
    },
    onError: (err) => {
      if (err instanceof Error && err.message === 'invalid json') {
        return;
      }
      message.error(err instanceof Error ? err.message : '保存成本核算失败');
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
      method: 'AVG',
      variance_cent: 0
    });
    setDrawerOpen(true);
  };

  const openEdit = (record: InventoryCostingRun) => {
    setEditing(record);
    drawerForm.setFieldsValue({
      method: record.method,
      period_start: record.period_start ?? undefined,
      period_end: record.period_end ?? undefined,
      status: record.status,
      variance_cent: record.variance_cent,
      result_json: record.result_json ? JSON.stringify(record.result_json, null, 2) : undefined
    });
    setDrawerOpen(true);
  };

  const columns = useMemo<ColumnsType<InventoryCostingRun>>(
    () => [
      { title: '核算方法', dataIndex: 'method' },
      { title: '期间', render: (_, record) => formatRange(record.period_start, record.period_end) },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'DONE' ? (
            <Tag color="green">完成</Tag>
          ) : value === 'RUNNING' ? (
            <Tag color="blue">运行中</Tag>
          ) : (
            <Tag color="orange">草稿</Tag>
          )
      },
      {
        title: '差异(分)',
        dataIndex: 'variance_cent',
        render: (value) => value.toLocaleString()
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
          <Button size="small" disabled={!canWrite} onClick={() => openEdit(record)}>
            编辑
          </Button>
        )
      }
    ],
    [canWrite]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          成本核算与库存成本
        </Typography.Title>
        <Button type="primary" disabled={!canWrite} onClick={openCreate}>
          新建核算
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="inline" onFinish={handleSearch}>
          <Form.Item name="method" label="方法">
            <Select allowClear options={methodOptions} placeholder="全部" style={{ width: 160 }} />
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
      <UiModule03MainTableSection<InventoryCostingRun>
        header={header}
        rowKey="costing_id"
        columns={columns}
        data={listQuery.data?.items ?? []}
        loading={listQuery.isFetching}
        total={listQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        error={listQuery.error}
        onRetry={() => listQuery.refetch()}
        errorTitle="成本核算加载失败"
        emptyText="暂无核算记录"
        columnSettingsKey="finance.costing.table.columns"
      />
      <Drawer
        title={editing ? '编辑核算' : '新建核算'}
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
          <Form.Item name="method" label="核算方法" rules={[{ required: true, message: '请选择方法' }]}>
            <Select options={methodOptions} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="variance_cent" label="差异(分)">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="period_start" label="开始日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="period_end" label="结束日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="result_json" label="结果 JSON">
            <Input.TextArea rows={3} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
