import {
  Button,
  Card,
  DatePicker,
  Form,
  Input,
  Progress,
  Select,
  Space,
  Tag,
  Typography,
  message,
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { components } from '@/api/generated/schema';
import { createExportJob, listExportJobs } from '@/api/exports';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { apiBaseUrl } from '@/utils/env';

type ExportJob = components['schemas']['ExportJob'];

type ExportFormValues = {
  type: string;
  range?: [Dayjs, Dayjs];
  platform?: string;
  shop_id?: string;
  warehouse_id?: string;
};

const exportTypeOptions = [
  { label: 'Order export', value: 'ORDER_EXPORT' },
  { label: 'Reconcile export', value: 'RECONCILE_EXPORT' },
  { label: 'Inventory export', value: 'INVENTORY_EXPORT' },
];

const statusColor = (status: string) => {
  switch (status) {
    case 'DONE':
      return 'green';
    case 'FAILED':
      return 'red';
    case 'RUNNING':
      return 'blue';
    default:
      return 'default';
  }
};

export const ExportJobsPage = () => {
  const queryClient = useQueryClient();
  const [form] = Form.useForm<ExportFormValues>();
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });

  const listQuery = useMemo(
    () => ({
      limit: pagination.pageSize ?? 20,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
    }),
    [pagination],
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['exports', listQuery],
    queryFn: () => listExportJobs(listQuery),
    placeholderData: (previousData) => previousData,
  });

  const createMutation = useMutation({
    mutationFn: createExportJob,
    onSuccess: () => {
      message.success('Export job created');
      queryClient.invalidateQueries({ queryKey: ['exports'] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : 'Failed to create export job');
    },
  });

  const handleCreate = async (values: ExportFormValues) => {
    const params: Record<string, unknown> = {};
    if (values.range?.[0] && values.range?.[1]) {
      params.start_at = values.range[0].toISOString();
      params.end_at = values.range[1].toISOString();
    }
    if (values.platform) {
      params.platform = values.platform.trim();
    }
    if (values.shop_id) {
      params.shop_id = values.shop_id.trim();
    }
    if (values.warehouse_id) {
      params.warehouse_id = values.warehouse_id.trim();
    }
    createMutation.mutate({ type: values.type, params });
  };

  const handleRetry = (job: ExportJob) => {
    createMutation.mutate({ type: job.type, params: job.params });
  };

  const columns: ColumnsType<ExportJob> = [
    { title: 'Job ID', dataIndex: 'job_id' },
    {
      title: 'Type',
      dataIndex: 'type',
      render: (value: string) => <Tag>{value}</Tag>,
    },
    {
      title: 'Status',
      dataIndex: 'status',
      render: (value: string) => <Tag color={statusColor(value)}>{value}</Tag>,
    },
    {
      title: 'Progress',
      dataIndex: 'progress',
      render: (value: number) => <Progress percent={value} size="small" />,
    },
    {
      title: 'Created At',
      dataIndex: 'created_at',
      render: (value: string) => dayjs(value).format('YYYY-MM-DD HH:mm'),
    },
    {
      title: 'Last Error',
      dataIndex: 'last_error',
      render: (value: string) => (value ? value : '-'),
    },
    {
      title: 'Actions',
      key: 'actions',
      render: (_, record) => (
        <Space>
          {record.output_url && record.status === 'DONE' ? (
            <Button
              size="small"
              onClick={() => {
                const href = record.output_url.startsWith('http')
                  ? record.output_url
                  : new URL(record.output_url, apiBaseUrl).toString();
                window.open(href, '_blank');
              }}
            >
              Download
            </Button>
          ) : null}
          {record.status === 'FAILED' ? (
            <Button size="small" onClick={() => handleRetry(record)}>
              Retry
            </Button>
          ) : null}
        </Space>
      ),
    },
  ];

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3}>Export Jobs</Typography.Title>
      <Card>
        <Form
          form={form}
          layout="inline"
          onFinish={handleCreate}
          initialValues={{
            type: 'ORDER_EXPORT',
            range: [dayjs().subtract(7, 'day'), dayjs()],
          }}
        >
          <Form.Item name="type" label="Type">
            <Select style={{ width: 180 }} options={exportTypeOptions} />
          </Form.Item>
          <Form.Item name="range" label="Date Range">
            <DatePicker.RangePicker allowClear />
          </Form.Item>
          <Form.Item name="platform" label="Platform">
            <Input placeholder="taobao/jd/pdd/douyin" />
          </Form.Item>
          <Form.Item name="shop_id" label="Shop ID">
            <Input placeholder="optional" />
          </Form.Item>
          <Form.Item name="warehouse_id" label="Warehouse ID">
            <Input placeholder="optional for inventory export" />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit" loading={createMutation.isLoading}>
              Create Export
            </Button>
          </Form.Item>
        </Form>
      </Card>
    </Space>
  );

  return (
    <UiModule03MainTableSection<ExportJob>
      header={header}
      rowKey="job_id"
      columns={columns}
      data={data?.items ?? []}
      loading={isLoading || isFetching}
      error={error}
      onRetry={() => refetch()}
      total={data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      emptyText="No export jobs"
      columnSettingsKey="exports.jobs.columns"
    />
  );
};
