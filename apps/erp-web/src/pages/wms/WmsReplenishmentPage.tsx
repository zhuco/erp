import { Button, Space, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  claimWmsReplenishmentTask,
  completeWmsReplenishmentTask,
  listWmsReplenishmentTasks,
  type WmsReplenishmentTask
} from '@/api/wmsExecution';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useTenantId } from '@/features/tenant';

const statusColorMap: Record<WmsReplenishmentTask['status'], string> = {
  PENDING: 'default',
  IN_PROGRESS: 'blue',
  DONE: 'green'
};

export const WmsReplenishmentPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['wms-replenishment-tasks', tenantId],
    queryFn: () => listWmsReplenishmentTasks(tenantId, { offset: 0, limit: 50 }),
    placeholderData: (previousData) => previousData
  });

  const claimMutation = useMutation({
    mutationFn: (taskId: string) =>
      claimWmsReplenishmentTask(tenantId, taskId, {
        assignee: '补货员A'
      }),
    onSuccess: async () => {
      message.success('补货任务已认领');
      await queryClient.invalidateQueries({ queryKey: ['wms-replenishment-tasks', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '认领补货任务失败');
    }
  });

  const completeMutation = useMutation({
    mutationFn: (taskId: string) => completeWmsReplenishmentTask(tenantId, taskId),
    onSuccess: async () => {
      message.success('补货任务已完成');
      await queryClient.invalidateQueries({ queryKey: ['wms-replenishment-tasks', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '完成补货任务失败');
    }
  });

  const columns: ColumnsType<WmsReplenishmentTask> = [
    { title: '任务ID', dataIndex: 'task_id' },
    { title: 'SKU', dataIndex: 'sku_id' },
    { title: '来源库位', dataIndex: 'from_location_id' },
    { title: '目标库位', dataIndex: 'to_location_id' },
    { title: '补货数量', dataIndex: 'qty' },
    { title: '负责人', dataIndex: 'assignee', render: (value) => value ?? '-' },
    {
      title: '状态',
      dataIndex: 'status',
      render: (value: WmsReplenishmentTask['status']) => (
        <Tag color={statusColorMap[value] ?? 'default'}>{value}</Tag>
      )
    },
    {
      title: '操作',
      key: 'actions',
      render: (_, record) => (
        <Space>
          <Button
            size="small"
            data-testid="wms-replenishment-claim"
            disabled={record.status !== 'PENDING'}
            loading={claimMutation.isPending}
            onClick={() => claimMutation.mutate(record.task_id)}
          >
            认领
          </Button>
          <Button
            size="small"
            data-testid="wms-replenishment-complete"
            disabled={record.status !== 'IN_PROGRESS'}
            loading={completeMutation.isPending}
            onClick={() => completeMutation.mutate(record.task_id)}
          >
            完成
          </Button>
        </Space>
      )
    }
  ];

  const tableData = query.data?.items ?? [];

  return (
    <UiModule03MainTableSection<WmsReplenishmentTask>
      toolbarLeft={
        <Typography.Title level={3} data-testid="wms-replenishment-title" style={{ margin: 0 }}>
          补货任务执行台
        </Typography.Title>
      }
      rowKey="task_id"
      columns={columns}
      data={tableData}
      loading={query.isLoading || query.isFetching}
      error={query.error}
      onRetry={() => query.refetch()}
      errorTitle="补货任务加载失败"
      emptyText="暂无补货任务"
      total={tableData.length}
      pagination={{ current: 1, pageSize: 50 }}
      onPaginationChange={() => undefined}
      dataTestId="wms-replenishment-main-grid"
      gridPreset="industrial-review-compact"
      selectionCellMode="custom"
      selectionHeaderMode="allInvert"
      showColumnSettings
      columnSettingsKey="wms.replenishment.table.columns"
    />
  );
};
