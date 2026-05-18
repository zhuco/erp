import {
  Button,
  Col,
  Descriptions,
  Drawer,
  Form,
  Input,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import { apiBaseUrl } from '@/utils/env';
import type {
  PrintJobDetail,
  PrintJobDispatchStatus,
  PrintJobEvent,
  PrintJobListQuery,
  PrintJobStatus,
  PrintJobSummary
} from '@/api/printJobs';
import { dispatchPrintJob, getPrintJobDetail, listPrintJobs } from '@/api/printJobs';
import { ErrorState, LoadingState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { retryPrintJobAndPrint } from '@/features/printing/printJobFlow';
import { useTenantId } from '@/features/tenant';
import { IndustrialCard, IndustrialTable } from '@/components/industrial';

type FilterValues = {
  status?: PrintJobStatus;
  job_type?: string;
  ref_type?: string;
  ref_id?: string;
};

const statusOptions = [
  { label: 'PENDING', value: 'PENDING' },
  { label: 'RUNNING', value: 'RUNNING' },
  { label: 'DONE', value: 'DONE' },
  { label: 'FAILED', value: 'FAILED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const statusColors: Record<PrintJobStatus, string> = {
  PENDING: 'blue',
  RUNNING: 'gold',
  DONE: 'green',
  FAILED: 'red'
};

const dispatchStatusColors: Record<PrintJobDispatchStatus, string> = {
  QUEUED: 'blue',
  DISPATCHED: 'gold',
  PRINTED: 'green',
  FAILED: 'red'
};

export const PrintJobsPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [retryingId, setRetryingId] = useState<string | null>(null);
  const [dispatchingId, setDispatchingId] = useState<string | null>(null);

  const query = useMemo<PrintJobListQuery>(
    () => ({
      status: filters.status,
      job_type: filters.job_type?.trim() || undefined,
      ref_type: filters.ref_type?.trim() || undefined,
      ref_id: filters.ref_id?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['print-jobs', tenantId, query],
    queryFn: () => listPrintJobs(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const {
    data: detail,
    isLoading: detailLoading,
    isFetching: detailFetching,
    error: detailError,
    refetch: detailRefetch
  } = useQuery<PrintJobDetail>({
    queryKey: ['print-job-detail', tenantId, activeJobId],
    queryFn: () => getPrintJobDetail(tenantId, activeJobId ?? ''),
    enabled: Boolean(activeJobId)
  });
  const detailBusy = detailLoading || detailFetching;

  const handleRetry = async (printJobId: string) => {
    setRetryingId(printJobId);
    try {
      await retryPrintJobAndPrint(tenantId, printJobId);
      message.success('Retry submitted');
      await queryClient.invalidateQueries({ queryKey: ['print-jobs', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['print-job-detail', tenantId, printJobId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '重试失败');
    } finally {
      setRetryingId(null);
    }
  };

  const handleDispatch = async (printJobId: string) => {
    setDispatchingId(printJobId);
    try {
      await dispatchPrintJob(tenantId, printJobId);
      message.success('Redispatch submitted');
      await queryClient.invalidateQueries({ queryKey: ['print-jobs', tenantId] });
      await queryClient.invalidateQueries({ queryKey: ['print-job-detail', tenantId, printJobId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '派发失败');
    } finally {
      setDispatchingId(null);
    }
  };

  const columns = useMemo<ColumnsType<PrintJobSummary>>(
    () => [
      {
        title: 'Status',
        dataIndex: 'status',
        render: (_, record) => <Tag color={statusColors[record.status]}>{record.status}</Tag>
      },
      {
        title: 'Dispatch Status',
        dataIndex: 'dispatch_status',
        render: (_, record) =>
          record.dispatch_status ? (
            <Tag color={dispatchStatusColors[record.dispatch_status]}>
              {record.dispatch_status}
            </Tag>
          ) : (
            '-'
          )
      },
      {
        title: '绫诲瀷',
        dataIndex: 'job_type',
        render: (_, record) => record.job_type ?? '-'
      },
      {
        title: 'Triggered At',
        dataIndex: 'device_name',
        render: (_, record) =>
          record.device_name || record.device_fingerprint || '-'
      },
      {
        title: '鍏宠仈绫诲瀷',
        dataIndex: 'ref_type',
        render: (_, record) => record.ref_type ?? '-'
      },
      {
        title: '鍏宠仈ID',
        dataIndex: 'ref_id',
        render: (_, record) => record.ref_id ?? '-'
      },
      {
        title: '灏濊瘯娆℃暟',
        dataIndex: 'attempts',
        render: (_, record) => record.attempts ?? 0
      },
      {
        title: '杈撳嚭',
        dataIndex: 'output_url',
        render: (_, record) =>
          record.output_url ? (
            <a href={`${apiBaseUrl}${record.output_url}`} target="_blank" rel="noreferrer">
              下载
            </a>
          ) : (
            '-'
          )
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '鎿嶄綔',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              data-testid="print-job-detail"
              onClick={() => {
                setActiveJobId(record.job_id);
                setDetailOpen(true);
              }}
            >
              详情
            </Button>
            {record.status === 'FAILED' ? (
              <Button
                size="small"
                loading={retryingId === record.job_id}
                onClick={() => handleRetry(record.job_id)}
              >
                重试
              </Button>
            ) : null}
            {record.dispatch_status === 'FAILED' ? (
              <Button
                size="small"
                loading={dispatchingId === record.job_id}
                onClick={() => handleDispatch(record.job_id)}
              >
                閲嶆淳
              </Button>
            ) : null}
          </Space>
        )
      }
    ],
    [dispatchingId, handleDispatch, handleRetry, retryingId]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const eventColumns = useMemo<ColumnsType<PrintJobEvent>>(
    () => [
      {
        title: '鏃堕棿',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      },
      {
        title: 'Status',
        dataIndex: 'status',
        render: (_, record) => <Tag color={statusColors[record.status]}>{record.status}</Tag>
      },
      {
        title: '说明',
        dataIndex: 'error',
        render: (_, record) => record.error ?? '-'
      },
      {
        title: 'Actions',
        dataIndex: 'actor_type',
        render: (_, record) => record.actor_type ?? '-'
      }
    ],
    []
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} data-testid="print-jobs-title">
        打印任务
      </Typography.Title>
      <IndustrialCard>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name='status' label='Status'>
                <Select allowClear options={statusOptions} placeholder='Select status' />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="job_type" label="绫诲瀷">
                <Input allowClear placeholder='e.g. SHIPPING_LABEL' />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="ref_type" label="鍏宠仈绫诲瀷">
                <Input allowClear placeholder='e.g. SHIPMENT' />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="ref_id" label="鍏宠仈ID">
                <Input allowClear placeholder="杈撳叆鍏宠仈ID" />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  鏌ヨ
                </Button>
                <Button onClick={handleReset}>閲嶇疆</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </IndustrialCard>
      <UiModule03MainTableSection<PrintJobSummary>
        rowKey="job_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="打印任务加载失败"
        emptyText="暂无打印任务"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="print-jobs.table.columns"
      />
      <Drawer
        title="打印任务详情"
        open={detailOpen}
        onClose={() => {
          setDetailOpen(false);
          setActiveJobId(null);
        }}
        width={640}
        destroyOnHidden
      >
        {detailBusy && !detail ? (
          <LoadingState loading={detailBusy} onRetry={() => detailRefetch()} />
        ) : null}
        {detailError ? (
          <ErrorState
            title="打印任务详情加载失败"
            error={detailError}
            onRetry={() => detailRefetch()}
          />
        ) : null}
        {detail ? (
          <Space direction="vertical" size={12} style={{ width: '100%' }}>
            <Descriptions column={2} bordered size="small">
              <Descriptions.Item label="任务 ID">{detail.job_id}</Descriptions.Item>
              <Descriptions.Item label="绫诲瀷">{detail.job_type ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="鎵ц璁惧">
                {detail.device_name || detail.device_fingerprint || '-'}
              </Descriptions.Item>
              <Descriptions.Item label="鍏宠仈绫诲瀷">{detail.ref_type ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="鍏宠仈ID">{detail.ref_id ?? '-'}</Descriptions.Item>
              <Descriptions.Item label="Status">
                <Tag color={statusColors[detail.status]}>{detail.status}</Tag>
              </Descriptions.Item>
              <Descriptions.Item label="Dispatch Status">
                {detail.dispatch_status ? (
                  <Tag color={dispatchStatusColors[detail.dispatch_status]}>
                    {detail.dispatch_status}
                  </Tag>
                ) : (
                  '-'
                )}
              </Descriptions.Item>
              <Descriptions.Item label="灏濊瘯娆℃暟">{detail.attempts ?? 0}</Descriptions.Item>
              <Descriptions.Item label="杈撳嚭">
                {detail.output_url ? (
                  <a href={`${apiBaseUrl}${detail.output_url}`} target="_blank" rel="noreferrer">
                    下载
                  </a>
                ) : (
                  '-'
                )}
              </Descriptions.Item>
              <Descriptions.Item label="鍒涘缓鏃堕棿">
                {formatDateTime(detail.created_at)}
              </Descriptions.Item>
              <Descriptions.Item label="更新时间">
                {formatDateTime(detail.updated_at)}
              </Descriptions.Item>
            </Descriptions>
            {detail.last_error ? (
              <ErrorState
                title="打印失败"
                description={detail.last_error}
                hint="You can retry printing or check the printer connection."
                action={
                  detail.status === 'FAILED' ? (
                    <Button
                      type="primary"
                      loading={retryingId === detail.job_id}
                      onClick={() => handleRetry(detail.job_id)}
                    >
                      重试打印
                    </Button>
                  ) : null
                }
              />
            ) : detail.status === 'FAILED' ? (
              <Button
                type="primary"
                loading={retryingId === detail.job_id}
                onClick={() => handleRetry(detail.job_id)}
              >
                重试打印
              </Button>
            ) : null}
            <IndustrialTable<PrintJobEvent>
              rowKey={(record) => `${record.status}-${record.created_at}`}
              columns={eventColumns}
              dataSource={detail.events}
              pagination={false}
              size="small"
              title={() => '浜嬩欢鍘嗗彶'}
            />
          </Space>
        ) : null}
      </Drawer>
    </Space>
  );
};

