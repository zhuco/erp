import {
  Alert,
  Button,
  Card,
  Col,
  Form,
  Input,
  Row,
  Select,
  Space,
  Tag,
  Tooltip,
  Typography,
  message,
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { SyncJob } from '@/api/syncJobs';
import { listSyncJobs, runSyncJob } from '@/api/syncJobs';
import type { SalesShopConfig } from '@/api/salesShops';
import { getSalesShopConfig } from '@/api/salesShops';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  shop_id?: string;
  job_type?: string;
  status?: string;
};

const jobTypeOptions = [
  { label: 'Order sync', value: 'order_sync' },
  { label: 'Product sync', value: 'product_sync' },
  { label: 'Refund sync', value: 'refund_sync' },
  { label: 'Shipping sync', value: 'shipping_sync' },
];

const statusOptions = [
  { label: 'PENDING', value: 'PENDING' },
  { label: 'RUNNING', value: 'RUNNING' },
  { label: 'FAILED', value: 'FAILED' },
  { label: 'SUCCESS', value: 'SUCCESS' },
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const renderStatusTag = (status: string) => {
  const normalized = status.toUpperCase();
  const color = normalized === 'FAILED' ? 'red' : normalized === 'RUNNING' ? 'blue' : 'green';
  return <Tag color={color}>{status}</Tag>;
};

export const SyncJobsPage = () => {
  const tenantId = useTenantId();
  const orderSyncFlag = useFeatureFlag('flows.order_auto_download');
  const mergeFlag = useFeatureFlag('flows.merge_outside_ship_time');
  const refundSyncFlag = useFeatureFlag('flows.aftersales_download');
  const shippingSyncFlag = useFeatureFlag('flows.logistics_upload');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });
  const [retryingId, setRetryingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      shop_id: filters.shop_id ?? undefined,
      job_type: filters.job_type ?? undefined,
      status: filters.status ?? undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20,
    }),
    [filters, pagination.current, pagination.pageSize],
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sync-jobs', tenantId, query],
    queryFn: () => listSyncJobs(tenantId, query),
    placeholderData: (previousData) => previousData,
  });

  const shopIds = useMemo(() => Array.from(new Set((data?.items ?? []).map((job) => job.shop_id))), [data?.items]);

  const salesConfigQuery = useQuery({
    queryKey: ['sales-shop-config-map', tenantId, shopIds],
    queryFn: async () => {
      if (shopIds.length === 0) {
        return new Map<string, SalesShopConfig>();
      }
      const configs = await Promise.all(shopIds.map((shopId) => getSalesShopConfig(tenantId, shopId)));
      return new Map(configs.map((config) => [config.shop_id, config]));
    },
    enabled: shopIds.length > 0,
  });

  const salesConfigMap = salesConfigQuery.data ?? new Map<string, SalesShopConfig>();

  const resolveDecision = (job: SyncJob) => {
    const config = salesConfigMap.get(job.shop_id);
    if (job.job_type === 'order_sync' && config && !config.auto_download_orders) {
      return { allowed: false, reason: 'Shop order auto download is disabled' };
    }
    if (job.job_type === 'refund_sync' && config && !config.enable_after_sales_download) {
      return { allowed: false, reason: 'Shop after-sales download is disabled' };
    }
    if (job.job_type === 'shipping_sync' && config && !config.enable_logistics_upload) {
      return { allowed: false, reason: 'Shop logistics upload is disabled' };
    }
    if (job.job_type === 'order_sync' && !orderSyncFlag.enabled) {
      return { allowed: false, reason: orderSyncFlag.reason };
    }
    if (job.job_type === 'refund_sync' && !refundSyncFlag.enabled) {
      return { allowed: false, reason: refundSyncFlag.reason };
    }
    if (job.job_type === 'shipping_sync' && !shippingSyncFlag.enabled) {
      return { allowed: false, reason: shippingSyncFlag.reason };
    }
    return { allowed: true };
  };

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleRetry = async (job: SyncJob) => {
    const decision = resolveDecision(job);
    if (!decision.allowed) {
      message.warning(decision.reason || 'This flow is disabled');
      return;
    }
    try {
      setRetryingId(job.job_id);
      await runSyncJob(tenantId, job.shop_id, { job_type: job.job_type });
      message.success('Sync job retried');
      await queryClient.invalidateQueries({ queryKey: ['sync-jobs', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'Retry failed');
    } finally {
      setRetryingId(null);
    }
  };

  const columns = useMemo<ColumnsType<SyncJob>>(
    () => [
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
        title: 'Status',
        dataIndex: 'status',
        render: (_, record) => renderStatusTag(record.status),
      },
      {
        title: 'Cursor Time',
        dataIndex: 'last_time',
        render: (_, record) => formatDateTime(record.last_time),
      },
      {
        title: 'Last Error',
        dataIndex: 'last_error',
        render: (_, record) => record.last_error || '-',
      },
      {
        title: 'Updated At',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at),
      },
      {
        title: 'Actions',
        key: 'actions',
        render: (_, record) => {
          const decision = resolveDecision(record);
          return decision.allowed ? (
            <Button size="small" loading={retryingId === record.job_id} onClick={() => handleRetry(record)}>
              Retry
            </Button>
          ) : (
            <Tooltip title={decision.reason}>
              <span>-</span>
            </Tooltip>
          );
        },
      },
    ],
    [
      retryingId,
      orderSyncFlag.enabled,
      orderSyncFlag.reason,
      refundSyncFlag.enabled,
      refundSyncFlag.reason,
      salesConfigMap,
      shippingSyncFlag.enabled,
      shippingSyncFlag.reason,
    ],
  );

  const disabledFlows = useMemo(() => {
    const names: string[] = [];
    if (!orderSyncFlag.enabled) {
      names.push('Order sync');
    }
    if (!mergeFlag.enabled) {
      names.push('Merge outside ship time');
    }
    if (!refundSyncFlag.enabled) {
      names.push('After-sales sync');
    }
    if (!shippingSyncFlag.enabled) {
      names.push('Logistics upload');
    }
    return names;
  }, [mergeFlag.enabled, orderSyncFlag.enabled, refundSyncFlag.enabled, shippingSyncFlag.enabled]);

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        Sync Jobs
      </Typography.Title>
      {disabledFlows.length ? (
        <Alert type="warning" showIcon message={`Disabled flows: ${disabledFlows.join(', ')}`} />
      ) : null}
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="shop_id" label="Shop ID">
                <Input placeholder="Input shop id" />
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
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  Search
                </Button>
                <Button onClick={handleReset}>Reset</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <UiModule03MainTableSection<SyncJob>
      header={header}
      rowKey="job_id"
      columns={columns}
      data={data?.items ?? []}
      loading={isLoading || isFetching}
      error={error}
      onRetry={() => refetch()}
      errorTitle="Failed to load sync jobs"
      emptyText="No sync jobs"
      total={data?.total ?? 0}
      pagination={pagination}
      onPaginationChange={setPagination}
    />
  );
};
