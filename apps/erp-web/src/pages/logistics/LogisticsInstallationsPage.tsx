import {
  Button,
  DatePicker,
  Descriptions,
  Drawer,
  Form,
  Input,
  Modal,
  Select,
  Space,
  Tag,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { LogisticsInstallationTask } from '@/api/logisticsInstallations';
import {
  addLogisticsInstallationAppointment,
  createLogisticsInstallationTask,
  listLogisticsInstallationTasks,
  updateLogisticsInstallationTask
} from '@/api/logisticsInstallations';
import { SettingsPageTemplate, SettingsPageSection } from '@/components/page-templates';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { IndustrialCard, IndustrialTable } from '@/components/industrial';

type TaskFormValues = {
  order_id: string;
  customer_name: string;
  address: string;
  service_type: string;
  assigned_team?: string;
  scheduled_at?: dayjs.Dayjs;
};

type AppointmentFormValues = {
  scheduled_at: dayjs.Dayjs;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const statusColors: Record<LogisticsInstallationTask['status'], string> = {
  SCHEDULED: 'blue',
  IN_PROGRESS: 'purple',
  DONE: 'green',
  EXCEPTION: 'red'
};

const statusLabels: Record<LogisticsInstallationTask['status'], string> = {
  SCHEDULED: '已预约',
  IN_PROGRESS: '服务中',
  DONE: '已完成',
  EXCEPTION: '异常待处理'
};

export const LogisticsInstallationsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [taskModalOpen, setTaskModalOpen] = useState(false);
  const [appointmentModalOpen, setAppointmentModalOpen] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);
  const [activeTaskId, setActiveTaskId] = useState<string | null>(null);
  const [taskForm] = Form.useForm<TaskFormValues>();
  const [appointmentForm] = Form.useForm<AppointmentFormValues>();

  const tasksQuery = useQuery({
    queryKey: ['logistics-installation-tasks', tenantId],
    queryFn: () =>
      listLogisticsInstallationTasks(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const tasks = tasksQuery.data?.items ?? [];
  const activeTask = useMemo(
    () => tasks.find((task) => task.task_id === activeTaskId) ?? null,
    [activeTaskId, tasks]
  );

  const openDetail = (task: LogisticsInstallationTask) => {
    setActiveTaskId(task.task_id);
    setDetailOpen(true);
  };

  const handleCreateTask = async () => {
    if (!canWrite) {
      message.warning('暂无权限');
      return;
    }
    try {
      const values = await taskForm.validateFields();
      await createLogisticsInstallationTask(tenantId, {
        order_id: values.order_id,
        customer_name: values.customer_name,
        address: values.address,
        service_type: values.service_type,
        assigned_team: values.assigned_team ?? null,
        scheduled_at: values.scheduled_at?.toISOString() ?? null
      });
      setTaskModalOpen(false);
      taskForm.resetFields();
      message.success('安装任务已创建');
      await queryClient.invalidateQueries({
        queryKey: ['logistics-installation-tasks', tenantId]
      });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  };

  const handleAddAppointment = async () => {
    if (!activeTaskId) {
      return;
    }
    try {
      const values = await appointmentForm.validateFields();
      await addLogisticsInstallationAppointment(tenantId, activeTaskId, {
        scheduled_at: values.scheduled_at.toISOString()
      });
      setAppointmentModalOpen(false);
      appointmentForm.resetFields();
      message.success('已新增预约');
      await queryClient.invalidateQueries({
        queryKey: ['logistics-installation-tasks', tenantId]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '新增预约失败');
    }
  };

  const handleComplete = async (task: LogisticsInstallationTask) => {
    if (!canWrite) {
      return;
    }
    try {
      await updateLogisticsInstallationTask(tenantId, task.task_id, {
        status: 'DONE'
      });
      message.success('服务完成已回写');
      await queryClient.invalidateQueries({
        queryKey: ['logistics-installation-tasks', tenantId]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '回写失败');
    }
  };

  const handleException = async (task: LogisticsInstallationTask) => {
    if (!canWrite) {
      return;
    }
    try {
      await updateLogisticsInstallationTask(tenantId, task.task_id, {
        status: 'EXCEPTION',
        remedy_ticket_id: `REM-${Date.now()}`,
        exception_note: '安装失败，已生成补救工单'
      });
      message.warning('已生成补救工单');
      await queryClient.invalidateQueries({
        queryKey: ['logistics-installation-tasks', tenantId]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '处理失败');
    }
  };

  const columns = useMemo<ColumnsType<LogisticsInstallationTask>>(
    () => [
      { title: '任务号', dataIndex: 'task_id' },
      { title: '订单号', dataIndex: 'order_id' },
      { title: '客户', dataIndex: 'customer_name' },
      { title: '服务类型', dataIndex: 'service_type' },
      { title: '服务团队', dataIndex: 'assigned_team', render: (value?: string | null) => value ?? '-' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: LogisticsInstallationTask['status']) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        )
      },
      {
        title: '预约次数',
        dataIndex: 'appointments',
        render: (value?: LogisticsInstallationTask['appointments']) => value?.length ?? 0
      },
      { title: '更新时间', dataIndex: 'updated_at', render: formatDateTime },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space size={8}>
            <Button size="small" onClick={() => openDetail(record)}>
              查看
            </Button>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setActiveTaskId(record.task_id);
                setAppointmentModalOpen(true);
              }}
            >
              新增预约
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => handleComplete(record)}>
              完成
            </Button>
            <Button size="small" disabled={!canWrite} danger onClick={() => handleException(record)}>
              异常
            </Button>
          </Space>
        )
      }
    ],
    [canWrite]
  );

  return (
    <>
      <SettingsPageTemplate title="配送安装服务">
        <SettingsPageSection
          title="安装任务"
          actions={
            <Button type="primary" disabled={!canWrite} onClick={() => setTaskModalOpen(true)}>
              新增任务
            </Button>
          }
        >
          <IndustrialTable
            rowKey="task_id"
            columns={columns}
            dataSource={tasks}
            loading={tasksQuery.isLoading}
          />
        </SettingsPageSection>
      </SettingsPageTemplate>
      <Modal
        title="新增安装任务"
        open={taskModalOpen}
        onCancel={() => setTaskModalOpen(false)}
        onOk={handleCreateTask}
        okText="创建"
      >
        <Form form={taskForm} layout="vertical">
          <Form.Item name="order_id" label="订单号" rules={[{ required: true, message: '请输入订单号' }]}>
            <Input placeholder="例如：ORD-100230" />
          </Form.Item>
          <Form.Item
            name="customer_name"
            label="客户姓名"
            rules={[{ required: true, message: '请输入客户姓名' }]}
          >
            <Input placeholder="例如：张先生" />
          </Form.Item>
          <Form.Item
            name="address"
            label="安装地址"
            rules={[{ required: true, message: '请输入安装地址' }]}
          >
            <Input placeholder="例如：上海市浦东新区示例路 100 号" />
          </Form.Item>
          <Form.Item
            name="service_type"
            label="服务类型"
            rules={[{ required: true, message: '请选择服务类型' }]}
          >
            <Select
              options={[
                { label: '送装一体', value: '送装一体' },
                { label: '安装服务', value: '安装服务' },
                { label: '配送服务', value: '配送服务' }
              ]}
            />
          </Form.Item>
          <Form.Item name="assigned_team" label="服务团队">
            <Input placeholder="例如：华东安装组" />
          </Form.Item>
          <Form.Item name="scheduled_at" label="首次预约">
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="新增预约"
        open={appointmentModalOpen}
        onCancel={() => setAppointmentModalOpen(false)}
        onOk={handleAddAppointment}
        okText="保存预约"
      >
        <Form form={appointmentForm} layout="vertical">
          <Form.Item
            name="scheduled_at"
            label="预约时间"
            rules={[{ required: true, message: '请选择预约时间' }]}
          >
            <DatePicker showTime style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Modal>
      <Drawer
        title="服务详情"
        open={detailOpen}
        onClose={() => setDetailOpen(false)}
        width={520}
        destroyOnHidden
      >
        {activeTask ? (
          <Space direction="vertical" size={16} style={{ width: '100%' }}>
            <Descriptions size="small" bordered column={1}>
              <Descriptions.Item label="任务号">{activeTask.task_id}</Descriptions.Item>
              <Descriptions.Item label="订单号">{activeTask.order_id}</Descriptions.Item>
              <Descriptions.Item label="客户">{activeTask.customer_name}</Descriptions.Item>
              <Descriptions.Item label="地址">{activeTask.address}</Descriptions.Item>
              <Descriptions.Item label="服务类型">{activeTask.service_type}</Descriptions.Item>
              <Descriptions.Item label="服务团队">
                {activeTask.assigned_team ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="状态">{statusLabels[activeTask.status]}</Descriptions.Item>
              <Descriptions.Item label="补救工单">
                {activeTask.remedy_ticket_id ?? '-'}
              </Descriptions.Item>
              <Descriptions.Item label="异常说明">
                {activeTask.exception_note ?? '-'}
              </Descriptions.Item>
            </Descriptions>
            <IndustrialCard size="small" title="预约记录">
              <IndustrialTable
                rowKey="appointment_id"
                pagination={false}
                dataSource={activeTask.appointments ?? []}
                columns={[
                  { title: '预约时间', dataIndex: 'scheduled_at', render: formatDateTime },
                  { title: '状态', dataIndex: 'status' }
                ]}
              />
            </IndustrialCard>
          </Space>
        ) : null}
      </Drawer>
    </>
  );
};
