import {
  Alert,
  Button,
  Col,
  Form,
  Input,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { StockSyncJob, StockSyncJobStatus } from '@/api/inventorySyncJobs';
import {
  listInventorySyncJobEvents,
  listInventorySyncJobs,
  retryInventorySyncJob,
  runInventorySyncJob
} from '@/api/inventorySyncJobs';
import { listWarehouses } from '@/api/warehouses';
import { ErrorState } from '@/components/QueryState';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FilterValues = {
  shop_id?: string;
  sku_id?: string;
  warehouse_id?: string;
  status?: StockSyncJobStatus;
};

const statusColors: Record<StockSyncJobStatus, string> = {
  PENDING: 'processing',
  SENDING: 'gold',
  SENT: 'success',
  FAILED: 'error'
};

export const InventorySyncJobsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('inventory.write');
  const inventoryFeature = useFeatureFlag('inventory.enabled');
  const inventorySyncFlag = useFeatureFlag('flows.inventory_sync');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [filters, setFilters] = useState<FilterValues>({ status: 'FAILED' });
  const [pagination, setPagination] = useState({
    current: 1,
    pageSize: 20
  });

  const warehouseQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'sync-jobs'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady && inventoryFeature.enabled && inventorySyncFlag.enabled,
    retry: false
  });

  const warehouseOptions = useMemo(
    () =>
      warehouseQuery.data?.items.map((warehouse) => ({
        label: warehouse.name,
        value: warehouse.warehouse_id
      })) ?? [],
    [warehouseQuery.data?.items]
  );

  const warehouseMap = useMemo(() => {
    const entries = warehouseQuery.data?.items ?? [];
    return new Map(entries.map((warehouse) => [warehouse.warehouse_id, warehouse.name]));
  }, [warehouseQuery.data?.items]);

  const query = useMemo(
    () => ({
      shop_id: filters.shop_id?.trim() || undefined,
      sku_id: filters.sku_id?.trim() || undefined,
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      status: filters.status,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const jobsQuery = useQuery({
    queryKey: ['inventory-sync-jobs', tenantId, query],
    queryFn: () => listInventorySyncJobs(tenantId, query),
    enabled: tenantReady && inventoryFeature.enabled && inventorySyncFlag.enabled,
    retry: false
  });

  const handleRefresh = () => {
    warehouseQuery.refetch();
    jobsQuery.refetch();
  };

  const retryMutation = useMutation({
    mutationFn: (jobId: string) => retryInventorySyncJob(tenantId, jobId),
    onSuccess: () => {
      message.success('已重新提交同步任务');
      queryClient.invalidateQueries({ queryKey: ['inventory-sync-jobs', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '重试失败');
    }
  });

  const runMutation = useMutation({
    mutationFn: (jobId: string) => runInventorySyncJob(tenantId, jobId),
    onSuccess: () => {
      message.success('已触发同步任务');
      queryClient.invalidateQueries({ queryKey: ['inventory-sync-jobs', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '执行失败');
    }
  });

  const columns = useMemo<ColumnsType<StockSyncJob>>(
    () => [
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: StockSyncJobStatus) => (
          <Tag color={statusColors[value] ?? 'default'}>{value}</Tag>
        )
      },
      {
        title: '店铺',
        dataIndex: 'shop_id'
      },
      {
        title: 'SKU ID',
        dataIndex: 'sku_id'
      },
      {
        title: '仓库',
        dataIndex: 'warehouse_id',
        render: (value: string) => warehouseMap.get(value) ?? value
      },
      {
        title: '目标库存',
        dataIndex: 'target_qty'
      },
      {
        title: '尝试次数',
        dataIndex: 'attempts'
      },
      {
        title: '成功数',
        dataIndex: 'success_count',
        render: (value) => value ?? 0
      },
      {
        title: '失败数',
        dataIndex: 'failure_count',
        render: (value) => value ?? 0
      },
      {
        title: '耗时(ms)',
        dataIndex: 'last_duration_ms',
        render: (value) => (value != null ? value : '-')
      },
      {
        title: '失败原因',
        dataIndex: 'last_error',
        render: (value) => value ?? '-'
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at'
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={
                !canWrite ||
                record.status === 'SENDING' ||
                record.status === 'FAILED' ||
                !inventorySyncFlag.enabled
              }
              loading={runMutation.isPending}
              onClick={() => runMutation.mutate(record.job_id)}
            >
              执行
            </Button>
            <Button
              size="small"
              disabled={!canWrite || record.status !== 'FAILED' || !inventorySyncFlag.enabled}
              loading={retryMutation.isPending}
              onClick={() => retryMutation.mutate(record.job_id)}
            >
              重试
            </Button>
            <Button
              size="small"
              disabled={!record.last_error}
              onClick={async () => {
                try {
                  const response = await listInventorySyncJobEvents(
                    tenantId,
                    record.job_id,
                    'ERROR'
                  );
                  const blob = new Blob([JSON.stringify(response.items, null, 2)], {
                    type: 'application/json;charset=utf-8'
                  });
                  const url = URL.createObjectURL(blob);
                  const link = document.createElement('a');
                  link.href = url;
                  link.download = `stock-sync-errors-${record.job_id}.json`;
                  link.click();
                  URL.revokeObjectURL(url);
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '下载失败原因失败');
                }
              }}
            >
              下载失败
            </Button>
          </Space>
        )
      }
    ],
    [
      canWrite,
      inventorySyncFlag.enabled,
      retryMutation.isPending,
      runMutation.isPending,
      tenantId,
      warehouseMap
    ]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({ status: 'FAILED' });
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        库存同步任务
      </Typography.Title>
      <Typography.Text type="secondary">统一查看库存同步状态、失败原因与重试动作</Typography.Text>
    </Space>
  );

  const toolbarRight = (
    <Button icon={<ReloadOutlined />} onClick={handleRefresh} disabled={!inventorySyncFlag.enabled}>
      刷新
    </Button>
  );

  const tableHeader = (
    <Form form={form} layout="vertical" onFinish={handleSearch} initialValues={filters}>
      <Row gutter={[16, 16]}>
        <Col xs={24} md={6}>
          <Form.Item name="shop_id" label="店铺 ID">
            <Input allowClear placeholder="输入店铺 ID" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="sku_id" label="SKU ID">
            <Input allowClear placeholder="输入 SKU ID" />
          </Form.Item>
        </Col>
        <Col xs={24} md={6}>
          <Form.Item name="warehouse_id" label="仓库">
            <Select allowClear options={warehouseOptions} placeholder="选择仓库" />
          </Form.Item>
        </Col>
        <Col xs={24} md={4}>
          <Form.Item name="status" label="状态">
            <Select
              allowClear
              options={[
                { value: 'FAILED', label: 'FAILED' },
                { value: 'PENDING', label: 'PENDING' },
                { value: 'SENDING', label: 'SENDING' },
                { value: 'SENT', label: 'SENT' }
              ]}
            />
          </Form.Item>
        </Col>
        <Col xs={24} md={2} style={{ display: 'flex', alignItems: 'end' }}>
          <Space>
            <Button type="primary" htmlType="submit">
              查询
            </Button>
            <Button onClick={handleReset}>重置</Button>
          </Space>
        </Col>
      </Row>
    </Form>
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      {!inventorySyncFlag.enabled ? (
        <Alert
          type="warning"
          showIcon
          message="库存同步流程已关闭"
          description={inventorySyncFlag.reason}
        />
      ) : null}
      {warehouseQuery.error ? (
        <ErrorState
          title="仓库加载失败"
          error={warehouseQuery.error}
          onRetry={() => warehouseQuery.refetch()}
        />
      ) : null}
      <UiModule03MainTableSection<StockSyncJob>
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        header={tableHeader}
        rowKey="job_id"
        columns={columns}
        data={jobsQuery.data?.items ?? []}
        loading={jobsQuery.isFetching}
        error={jobsQuery.error}
        onRetry={() => jobsQuery.refetch()}
        errorTitle="库存同步任务加载失败"
        emptyText="暂无同步任务"
        total={jobsQuery.data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        dataTestId="inventory-sync-jobs-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        columnSettingsKey="inventory.sync.jobs.table.columns"
        enablePanels={false}
      />
    </Space>
  );
};
