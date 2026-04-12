import {
  Button,
  Card,
  Col,
  DatePicker,
  Form,
  Input,
  Row,
  Select,
  Space,
  Tag,
  Typography
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type { SyncJobRun, SyncJobRunListQuery } from '@/api/syncJobRuns';
import { listSyncJobRuns } from '@/api/syncJobRuns';
import { ServerTable } from '@/components/ServerTable';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  shop_id?: string;
  job_type?: string;
  status?: string;
  started_range?: [Dayjs, Dayjs];
};

const jobTypeOptions = [
  { label: '订单同步', value: 'order_sync' },
  { label: '商品同步', value: 'product_sync' },
  { label: '售后同步', value: 'refund_sync' },
  { label: '物流同步', value: 'shipping_sync' }
];

const statusOptions = [
  { label: 'SUCCESS', value: 'SUCCESS' },
  { label: 'FAILED', value: 'FAILED' }
];

const statusColors: Record<string, string> = {
  SUCCESS: 'green',
  FAILED: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatDuration = (value?: number | null) =>
  value == null ? '-' : `${(value / 1000).toFixed(2)}s`;

export const SyncJobRunsPage = () => {
  const tenantId = useTenantId();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });

  const query = useMemo<SyncJobRunListQuery>(() => {
    const range = filters.started_range;
    return {
      shop_id: filters.shop_id?.trim() || undefined,
      job_type: filters.job_type,
      status: filters.status,
      started_from: range?.[0]?.toISOString(),
      started_to: range?.[1]?.toISOString(),
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sync-job-runs', tenantId, query],
    queryFn: () => listSyncJobRuns(tenantId, query),
    keepPreviousData: true
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

  const columns = useMemo<ColumnsType<SyncJobRun>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={statusColors[record.status] ?? 'default'}>{record.status}</Tag>
        )
      },
      {
        title: '店铺ID',
        dataIndex: 'shop_id'
      },
      {
        title: '任务类型',
        dataIndex: 'job_type',
        render: (_, record) =>
          jobTypeOptions.find((option) => option.value === record.job_type)?.label ??
          record.job_type
      },
      {
        title: '同步数量',
        dataIndex: 'item_count'
      },
      {
        title: '耗时',
        dataIndex: 'duration_ms',
        render: (_, record) => formatDuration(record.duration_ms)
      },
      {
        title: '错误信息',
        dataIndex: 'error',
        ellipsis: true,
        render: (_, record) => record.error || '-'
      },
      {
        title: '开始时间',
        dataIndex: 'started_at',
        render: (_, record) => formatDateTime(record.started_at)
      },
      {
        title: '结束时间',
        dataIndex: 'finished_at',
        render: (_, record) => formatDateTime(record.finished_at)
      }
    ],
    []
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3}>同步任务日志</Typography.Title>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="shop_id" label="店铺ID">
                <Input allowClear placeholder="输入店铺ID" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="job_type" label="任务类型">
                <Select allowClear options={jobTypeOptions} placeholder="选择任务类型" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="选择状态" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="started_range" label="开始时间">
                <DatePicker.RangePicker
                  allowClear
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['开始时间', '结束时间']}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      <ServerTable<SyncJobRun>
        rowKey="run_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="同步运行日志加载失败"
        emptyText="暂无运行记录"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="sales.sync.job.runs.columns"
      />
    </Space>
  );
};
