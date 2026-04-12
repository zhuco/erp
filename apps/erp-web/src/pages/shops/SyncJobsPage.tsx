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
  message
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
import { ServerTable } from '@/components/ServerTable';
import { useFeatureFlag } from '@/features/featureFlags';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  shop_id?: string;
  job_type?: string;
  status?: string;
};

const jobTypeOptions = [
  { label: '订单同步', value: 'order_sync' },
  { label: '商品同步', value: 'product_sync' },
  { label: '退款同步', value: 'refund_sync' },
  { label: '发货同步', value: 'shipping_sync' }
];

const statusOptions = [
  { label: '待处理', value: 'PENDING' },
  { label: '运行中', value: 'RUNNING' },
  { label: '失败', value: 'FAILED' },
  { label: '完成', value: 'SUCCESS' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const renderStatusTag = (status: string) => {
  const normalized = status.toUpperCase();
  const color =
    normalized === 'FAILED' ? 'red' : normalized === 'RUNNING' ? 'blue' : 'green';
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
    pageSize: 20
  });
  const [retryingId, setRetryingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      shop_id: filters.shop_id ?? undefined,
      job_type: filters.job_type ?? undefined,
      status: filters.status ?? undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sync-jobs', tenantId, query],
    queryFn: () => listSyncJobs(tenantId, query),
    keepPreviousData: true
  });

  const shopIds = useMemo(
    () => Array.from(new Set((data?.items ?? []).map((job) => job.shop_id))),
    [data?.items]
  );

  const salesConfigQuery = useQuery({
    queryKey: ['sales-shop-config-map', tenantId, shopIds],
    queryFn: async () => {
      if (shopIds.length === 0) {
        return new Map<string, SalesShopConfig>();
      }
      const configs = await Promise.all(
        shopIds.map((shopId) => getSalesShopConfig(tenantId, shopId))
      );
      return new Map(configs.map((config) => [config.shop_id, config]));
    },
    enabled: shopIds.length > 0
  });

  const salesConfigMap = salesConfigQuery.data ?? new Map<string, SalesShopConfig>();

  const resolveDecision = (job: SyncJob) => {
    const config = salesConfigMap.get(job.shop_id);
    if (job.job_type === 'order_sync' && config && !config.auto_download_orders) {
      return { allowed: false, reason: '店铺自动下载已关闭' };
    }
    if (job.job_type === 'refund_sync' && config && !config.enable_after_sales_download) {
      return { allowed: false, reason: '店铺售后下载已关闭' };
    }
    if (job.job_type === 'shipping_sync' && config && !config.enable_logistics_upload) {
      return { allowed: false, reason: '店铺物流上传已关闭' };
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
      message.warning(decision.reason || '该流程已关闭');
      return;
    }
    try {
      setRetryingId(job.job_id);
      await runSyncJob(tenantId, job.shop_id, { job_type: job.job_type });
      message.success('同步任务已重试');
      await queryClient.invalidateQueries({ queryKey: ['sync-jobs', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '重试失败');
    } finally {
      setRetryingId(null);
    }
  };

  const columns = useMemo<ColumnsType<SyncJob>>(
    () => [
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
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => renderStatusTag(record.status)
      },
      {
        title: '游标时间',
        dataIndex: 'last_time',
        render: (_, record) => formatDateTime(record.last_time)
      },
      {
        title: '错误信息',
        dataIndex: 'last_error',
        render: (_, record) => record.last_error || '-'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => {
          const decision = resolveDecision(record);
          return decision.allowed ? (
            <Button
              size="small"
              loading={retryingId === record.job_id}
              onClick={() => handleRetry(record)}
            >
              重试
            </Button>
          ) : (
            <Tooltip title={decision.reason}>
              <span>-</span>
            </Tooltip>
          );
        }
      }
    ],
    [
      handleRetry,
      orderSyncFlag.enabled,
      orderSyncFlag.reason,
      refundSyncFlag.enabled,
      refundSyncFlag.reason,
      retryingId,
      salesConfigMap,
      shippingSyncFlag.enabled,
      shippingSyncFlag.reason
    ]
  );

  const disabledFlows = useMemo(() => {
    const names: string[] = [];
    if (!orderSyncFlag.enabled) {
      names.push('订单同步');
    }
    if (!mergeFlag.enabled) {
      names.push('非发货时间合并');
    }
    if (!refundSyncFlag.enabled) {
      names.push('售后下载');
    }
    if (!shippingSyncFlag.enabled) {
      names.push('物流上传');
    }
    return names;
  }, [
    mergeFlag.enabled,
    orderSyncFlag.enabled,
    refundSyncFlag.enabled,
    shippingSyncFlag.enabled
  ]);

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        同步任务
      </Typography.Title>
      {disabledFlows.length ? (
        <Alert type="warning" showIcon message={`以下流程已关闭：${disabledFlows.join('、')}`} />
      ) : null}
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="shop_id" label="店铺ID">
                <Input placeholder="输入店铺ID" />
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
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
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
      <ServerTable<SyncJob>
        rowKey="job_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="同步任务加载失败"
        emptyText="暂无同步任务"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
      />
    </Space>
  );
};
