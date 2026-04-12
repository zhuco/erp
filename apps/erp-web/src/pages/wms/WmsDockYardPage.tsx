import { Button, Space, Table, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  checkInWmsDockAppointment,
  departWmsDockAppointment,
  listWmsDockAppointments,
  startWmsDockLoading,
  type WmsDockAppointment
} from '@/api/wmsExecution';
import { ErrorState } from '@/components/QueryState';
import { useTenantId } from '@/features/tenant';

const statusColorMap: Record<WmsDockAppointment['status'], string> = {
  SCHEDULED: 'default',
  CHECKED_IN: 'blue',
  LOADING: 'gold',
  DEPARTED: 'green'
};

export const WmsDockYardPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: ['wms-dock-appointments', tenantId],
    queryFn: () => listWmsDockAppointments(tenantId, { offset: 0, limit: 50 }),
    keepPreviousData: true
  });

  const checkinMutation = useMutation({
    mutationFn: (appointmentId: string) => checkInWmsDockAppointment(tenantId, appointmentId),
    onSuccess: async () => {
      message.success('车辆已签到');
      await queryClient.invalidateQueries({ queryKey: ['wms-dock-appointments', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '签到失败');
    }
  });

  const loadingMutation = useMutation({
    mutationFn: (appointmentId: string) => startWmsDockLoading(tenantId, appointmentId),
    onSuccess: async () => {
      message.success('已开始装车');
      await queryClient.invalidateQueries({ queryKey: ['wms-dock-appointments', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '开始装车失败');
    }
  });

  const departMutation = useMutation({
    mutationFn: (appointmentId: string) => departWmsDockAppointment(tenantId, appointmentId),
    onSuccess: async () => {
      message.success('车辆已离场');
      await queryClient.invalidateQueries({ queryKey: ['wms-dock-appointments', tenantId] });
    },
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '离场失败');
    }
  });

  const columns: ColumnsType<WmsDockAppointment> = [
    { title: '预约ID', dataIndex: 'appointment_id' },
    { title: '月台', dataIndex: 'dock_no' },
    { title: '承运商', dataIndex: 'carrier_name' },
    { title: '车牌号', dataIndex: 'vehicle_no' },
    { title: 'ETA', dataIndex: 'eta_at' },
    {
      title: '状态',
      dataIndex: 'status',
      render: (value: WmsDockAppointment['status']) => (
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
            data-testid="wms-dock-checkin"
            disabled={record.status !== 'SCHEDULED'}
            loading={checkinMutation.isPending}
            onClick={() => checkinMutation.mutate(record.appointment_id)}
          >
            签到
          </Button>
          <Button
            size="small"
            data-testid="wms-dock-loading"
            disabled={record.status !== 'CHECKED_IN'}
            loading={loadingMutation.isPending}
            onClick={() => loadingMutation.mutate(record.appointment_id)}
          >
            装车
          </Button>
          <Button
            size="small"
            data-testid="wms-dock-depart"
            disabled={record.status !== 'LOADING'}
            loading={departMutation.isPending}
            onClick={() => departMutation.mutate(record.appointment_id)}
          >
            离场
          </Button>
        </Space>
      )
    }
  ];

  if (query.error) {
    return (
      <ErrorState title="月台预约加载失败" error={query.error} onRetry={() => query.refetch()} />
    );
  }

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }}>
      <Typography.Title level={3} data-testid="wms-dock-title" style={{ margin: 0 }}>
        月台与场院协同
      </Typography.Title>
      <div data-testid="wms-dock-table">
        <Table<WmsDockAppointment>
          rowKey="appointment_id"
          loading={query.isLoading || query.isFetching}
          dataSource={query.data?.items ?? []}
          columns={columns}
          pagination={false}
        />
      </div>
    </Space>
  );
};
