import {
  Button,
  Col,
  DatePicker,
  Form,
  Input,
  Row,
  Select,
  Space,
  Tag,
  Typography,
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type { SyncJobRun, SyncJobRunListQuery } from '@/api/syncJobRuns';
import { listSyncJobRuns } from '@/api/syncJobRuns';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  shop_id?: string;
  job_type?: string;
  status?: string;
  started_range?: [Dayjs, Dayjs];
};

const jobTypeOptions = [
  { label: 'Order sync', value: 'order_sync' },
  { label: 'Product sync', value: 'product_sync' },
  { label: 'Refund sync', value: 'refund_sync' },
  { label: 'Shipping sync', value: 'shipping_sync' },
];

const statusOptions = [
  { label: 'SUCCESS', value: 'SUCCESS' },
  { label: 'FAILED', value: 'FAILED' },
];

const statusColors: Record<string, string> = {
  SUCCESS: 'green',
  FAILED: 'red',
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const formatDuration = (value?: number | null) => (value == null ? '-' : `${(value / 1000).toFixed(2)}s`);

export const SyncJobRunsPage = () => {
  const tenantId = useTenantId();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
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
      limit: pagination.pageSize ?? 20,
    };
  }, [filters, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sync-job-runs', tenantId, query],
    queryFn: () => listSyncJobRuns(tenantId, query),
    placeholderData: (previousData) => previousData,
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
        title: 'Status',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={statusColors[record.status] ?? 'default'}>{record.status}</Tag>
        ),
      },
      {
        title: 'Shop ID',
        dataIndex: 'shop_id',
      },
      {
        title: 'Job Type',
        dataIndex: 'job_type',
        render: (_, record) =>
          jobTypeOptions.find((option) => option.value === record.job_type)?.label ?? record.job_type,
      },
      {
        title: 'Item Count',
        dataIndex: 'item_count',
      },
      {
        title: 'Duration',
        dataIndex: 'duration_ms',
        render: (_, record) => formatDuration(record.duration_ms),
      },
      {
        title: 'Error',
        dataIndex: 'error',
        ellipsis: true,
        render: (_, record) => record.error || '-',
      },
      {
        title: 'Started At',
        dataIndex: 'started_at',
        render: (_, record) => formatDateTime(record.started_at),
      },
      {
        title: 'Finished At',
        dataIndex: 'finished_at',
        render: (_, record) => formatDateTime(record.finished_at),
      },
    ],
    [],
  );

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        Sync Job Runs
      </Typography.Title>
    </Space>
  );

  const toolbarRight = null;

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="shop_id" label="Shop ID">
                <Input allowClear placeholder="Input shop id" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="job_type" label="Job Type">
                <Select allowClear options={jobTypeOptions} placeholder="Select job type" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="Status">
                <Select allowClear options={statusOptions} placeholder="Select status" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12}>
              <Form.Item name="started_range" label="Started Range">
                <DatePicker.RangePicker
                  allowClear
                  showTime
                  style={{ width: '100%' }}
                  placeholder={['Start time', 'End time']}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  Search
                </Button>
                <Button onClick={handleReset}>Reset</Button>
              </Space>
            </Col>
          </Row>
        </Form>
    </Space>
  );

  return (
    <UiModule03MainTableSection<SyncJobRun>
      toolbarLeft={toolbarLeft}
      toolbarRight={toolbarRight}
      header={header}
      rowKey="run_id"
      columns={columns}
      data={data?.items ?? []}
      loading={isLoading || isFetching}
      error={error}
      onRetry={() => refetch()}
      errorTitle="Failed to load sync job runs"
      emptyText="No sync job runs"
      total={data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
      dataTestId="sales-sync-job-runs-main-grid"
      gridPreset="industrial-review-compact"
      selectionCellMode="custom"
      selectionHeaderMode="allInvert"
      showColumnSettings
      columnSettingsKey="sales.sync.job.runs.columns"
    />
  );
};
