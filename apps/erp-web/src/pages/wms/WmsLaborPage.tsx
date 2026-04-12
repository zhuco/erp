import { Button, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  assignWmsLaborTask,
  completeWmsLaborTask,
  listWmsLaborTasks,
  startWmsLaborTask,
  type WmsLaborTask
} from '@/api/wmsExecution';
import { ErrorState } from '@/components/QueryState';
import { useTenantId } from '@/features/tenant';

const statusColorMap: Record<WmsLaborTask['status'], string> = {
  UNASSIGNED: 'default',
  ASSIGNED: 'blue',
  IN_PROGRESS: 'gold',
  DONE: 'green'
};

export const WmsLaborPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['wms-labor-tasks', tenantId],
    queryFn: () => listWmsLaborTasks(tenantId, { offset: 0, limit: 50 }),
    keepPreviousData: true
  });

  const assignMutation = useMutation({
    mutationFn: (taskId: string) =>
      assignWmsLaborTask(tenantId, taskId, {
        assignee: '当班作业员'
      }),
    onSuccess: async () => {
      message.success('任务已分配');
      await queryClient.invalidateQueries({ queryKey: ['wms-labor-tasks', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '任务分配失败');
    }
  });

  const startMutation = useMutation({
    mutationFn: (taskId: string) => startWmsLaborTask(tenantId, taskId),
    onSuccess: async () => {
      message.success('任务已开始');
      await queryClient.invalidateQueries({ queryKey: ['wms-labor-tasks', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '任务开始失败');
    }
  });

  const completeMutation = useMutation({
    mutationFn: (taskId: string) => completeWmsLaborTask(tenantId, taskId),
    onSuccess: async () => {
      message.success('任务已完成');
      await queryClient.invalidateQueries({ queryKey: ['wms-labor-tasks', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '任务完成失败');
    }
  });

  const columns: ColumnsType<WmsLaborTask> = [
    { title: '任务ID', dataIndex: 'task_id' },
    { title: '库区', dataIndex: 'zone_code' },
    { title: '任务类型', dataIndex: 'task_type' },
    { title: '负责人', dataIndex: 'assignee', render: (value) => value ?? '-' },
    { title: '计划件数', dataIndex: 'planned_qty' },
    { title: '已完成件数', dataIndex: 'finished_qty' },
    {
      title: '状态',
      dataIndex: 'status',
      render: (value: WmsLaborTask['status']) => (
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
            data-testid="wms-labor-assign"
            disabled={record.status !== 'UNASSIGNED'}
            loading={assignMutation.isPending}
            onClick={() => assignMutation.mutate(record.task_id)}
          >
            分配
          </Button>
          <Button
            size="small"
            data-testid="wms-labor-start"
            disabled={record.status !== 'ASSIGNED'}
            loading={startMutation.isPending}
            onClick={() => startMutation.mutate(record.task_id)}
          >
            开始
          </Button>
          <Button
            size="small"
            data-testid="wms-labor-complete"
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

  if (query.error) {
    return (
      <ErrorState title="labor 任务加载失败" error={query.error} onRetry={() => query.refetch()} />
    );
  }

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      <Typography.Title level={3} data-testid="wms-labor-title" style={{ margin: 0 }}>
        Labor 作业任务池
      </Typography.Title>
      <div data-testid="wms-labor-table">
        <Table<WmsLaborTask>
          rowKey="task_id"
          loading={query.isLoading || query.isFetching}
          dataSource={query.data?.items ?? []}
          columns={columns}
          pagination={false}
        />
      </div>
    </Space>
  );
};
