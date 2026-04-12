import {
  Button,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Switch,
  Table,
  Tag,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  LogisticsExceptionType,
  LogisticsExceptionWorkOrder
} from '@/api/logisticsExceptions';
import {
  assignLogisticsExceptionWorkOrder,
  createLogisticsExceptionType,
  escalateLogisticsExceptionWorkOrder,
  listLogisticsExceptionTypes,
  listLogisticsExceptionWorkOrders,
  resolveLogisticsExceptionWorkOrder,
  updateLogisticsExceptionType
} from '@/api/logisticsExceptions';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type ExceptionFormValues = {
  type_code: string;
  name: string;
  sla_hours: number;
  severity: LogisticsExceptionType['severity'];
  enabled: boolean;
};

type AssignFormValues = {
  assignee: string;
};

type ResolveFormValues = {
  resolution: string;
};

const severityColors: Record<LogisticsExceptionType['severity'], string> = {
  LOW: 'default',
  MEDIUM: 'orange',
  HIGH: 'red'
};

const statusLabels: Record<LogisticsExceptionWorkOrder['status'], string> = {
  OPEN: '待处理',
  IN_PROGRESS: '处理中',
  ESCALATED: '已升级',
  RESOLVED: '已完成'
};

const statusColors: Record<LogisticsExceptionWorkOrder['status'], string> = {
  OPEN: 'blue',
  IN_PROGRESS: 'purple',
  ESCALATED: 'orange',
  RESOLVED: 'green'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const LogisticsExceptionsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [typeModalOpen, setTypeModalOpen] = useState(false);
  const [editingType, setEditingType] = useState<LogisticsExceptionType | null>(null);
  const [assignModalOpen, setAssignModalOpen] = useState(false);
  const [resolveModalOpen, setResolveModalOpen] = useState(false);
  const [activeOrder, setActiveOrder] = useState<LogisticsExceptionWorkOrder | null>(null);
  const [typeForm] = Form.useForm<ExceptionFormValues>();
  const [assignForm] = Form.useForm<AssignFormValues>();
  const [resolveForm] = Form.useForm<ResolveFormValues>();

  const typesQuery = useQuery({
    queryKey: ['logistics-exception-types', tenantId],
    queryFn: () => listLogisticsExceptionTypes(tenantId, {})
  });

  const workOrdersQuery = useQuery({
    queryKey: ['logistics-exception-work-orders', tenantId],
    queryFn: () =>
      listLogisticsExceptionWorkOrders(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const types = typesQuery.data ?? [];
  const workOrders = workOrdersQuery.data?.items ?? [];

  const typeOptions = useMemo(
    () =>
      types
        .filter((item) => item.enabled)
        .map((item) => ({ label: item.name, value: item.type_code })),
    [types]
  );

  const openTypeModal = (record?: LogisticsExceptionType) => {
    setEditingType(record ?? null);
    if (record) {
      typeForm.setFieldsValue(record);
    } else {
      typeForm.resetFields();
      typeForm.setFieldsValue({
        type_code: '',
        name: '',
        sla_hours: 6,
        severity: 'MEDIUM',
        enabled: true
      });
    }
    setTypeModalOpen(true);
  };

  const saveType = async () => {
    if (!canWrite) {
      message.warning('暂无权限');
      return;
    }
    try {
      const values = await typeForm.validateFields();
      if (editingType) {
        await updateLogisticsExceptionType(tenantId, editingType.type_code, {
          name: values.name,
          sla_hours: values.sla_hours,
          severity: values.severity,
          enabled: values.enabled
        });
      } else {
        await createLogisticsExceptionType(tenantId, values);
      }
      message.success('异常类型已保存');
      setTypeModalOpen(false);
      setEditingType(null);
      await queryClient.invalidateQueries({ queryKey: ['logistics-exception-types', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  };

  const handleAssign = async () => {
    if (!activeOrder) {
      return;
    }
    try {
      const values = await assignForm.validateFields();
      await assignLogisticsExceptionWorkOrder(tenantId, activeOrder.ticket_id, values);
      setAssignModalOpen(false);
      message.success('已完成分派');
      await queryClient.invalidateQueries({
        queryKey: ['logistics-exception-work-orders', tenantId]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '分派失败');
    }
  };

  const handleResolve = async () => {
    if (!activeOrder) {
      return;
    }
    try {
      const values = await resolveForm.validateFields();
      await resolveLogisticsExceptionWorkOrder(tenantId, activeOrder.ticket_id, values);
      setResolveModalOpen(false);
      message.success('处理结果已回写');
      await queryClient.invalidateQueries({
        queryKey: ['logistics-exception-work-orders', tenantId]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '处理失败');
    }
  };

  const handleEscalate = async (record: LogisticsExceptionWorkOrder) => {
    if (!canWrite) {
      return;
    }
    try {
      await escalateLogisticsExceptionWorkOrder(tenantId, record.ticket_id);
      message.info('已触发升级提醒');
      await queryClient.invalidateQueries({
        queryKey: ['logistics-exception-work-orders', tenantId]
      });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '升级失败');
    }
  };

  const typeColumns = useMemo<ColumnsType<LogisticsExceptionType>>(
    () => [
      { title: '类型名称', dataIndex: 'name' },
      { title: '代码', dataIndex: 'type_code' },
      { title: 'SLA(小时)', dataIndex: 'sla_hours' },
      {
        title: '优先级',
        dataIndex: 'severity',
        render: (value: LogisticsExceptionType['severity']) => (
          <Tag color={severityColors[value]}>{value}</Tag>
        )
      },
      {
        title: '启用',
        dataIndex: 'enabled',
        render: (value: boolean, record) => (
          <Switch
            size="small"
            checked={value}
            disabled={!canWrite}
            onChange={async (checked) => {
              try {
                await updateLogisticsExceptionType(tenantId, record.type_code, {
                  enabled: checked
                });
                await queryClient.invalidateQueries({
                  queryKey: ['logistics-exception-types', tenantId]
                });
              } catch (err) {
                message.error(err instanceof Error ? err.message : '更新失败');
              }
            }}
          />
        )
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value: string) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Button size="small" disabled={!canWrite} onClick={() => openTypeModal(record)}>
            编辑
          </Button>
        )
      }
    ],
    [canWrite, queryClient, tenantId]
  );

  const orderColumns = useMemo<ColumnsType<LogisticsExceptionWorkOrder>>(
    () => [
      { title: '工单号', dataIndex: 'ticket_id' },
      { title: '订单号', dataIndex: 'order_id' },
      { title: '运单号', dataIndex: 'tracking_no' },
      { title: '承运商', dataIndex: 'carrier_code' },
      {
        title: '异常类型',
        dataIndex: 'exception_type',
        render: (value: string) =>
          types.find((item) => item.type_code === value)?.name ?? value
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: LogisticsExceptionWorkOrder['status']) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        )
      },
      { title: '负责人', dataIndex: 'assignee', render: (value?: string | null) => value ?? '-' },
      { title: 'SLA 截止', dataIndex: 'due_at', render: formatDateTime },
      {
        title: '升级',
        dataIndex: 'escalation_level',
        render: (value: number) => (value > 0 ? `已升级${value}次` : '未升级')
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space size={8}>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setActiveOrder(record);
                assignForm.setFieldsValue({ assignee: record.assignee ?? '' });
                setAssignModalOpen(true);
              }}
            >
              分派
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => handleEscalate(record)}>
              升级
            </Button>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setActiveOrder(record);
                resolveForm.setFieldsValue({ resolution: record.resolution ?? '' });
                setResolveModalOpen(true);
              }}
            >
              处理完成
            </Button>
          </Space>
        )
      }
    ],
    [assignForm, canWrite, resolveForm, types]
  );

  return (
    <div data-testid="logistics-exceptions-page">
      <UiModule07SettingsLayout title="物流异常工单">
        <UiModule07SettingsSection
          title="异常类型配置"
          actions={
            <Button type="primary" disabled={!canWrite} onClick={() => openTypeModal()}>
              新增类型
            </Button>
          }
        >
          <Table
            rowKey="type_code"
            columns={typeColumns}
            dataSource={types}
            pagination={false}
            loading={typesQuery.isLoading}
          />
        </UiModule07SettingsSection>
        <UiModule07SettingsSection title="异常处理工单">
          <Table
            rowKey="ticket_id"
            columns={orderColumns}
            dataSource={workOrders}
            loading={workOrdersQuery.isLoading}
          />
        </UiModule07SettingsSection>
      </UiModule07SettingsLayout>
      <Modal
        title={editingType ? '编辑异常类型' : '新增异常类型'}
        open={typeModalOpen}
        onCancel={() => setTypeModalOpen(false)}
        onOk={saveType}
        okText="保存"
      >
        <Form form={typeForm} layout="vertical">
          <Form.Item
            name="type_code"
            label="类型代码"
            rules={[{ required: true, message: '请输入类型代码' }]}
          >
            <Input placeholder="例如：STUCK" disabled={Boolean(editingType)} />
          </Form.Item>
          <Form.Item
            name="name"
            label="类型名称"
            rules={[{ required: true, message: '请输入类型名称' }]}
          >
            <Input placeholder="例如：停滞未更新" />
          </Form.Item>
          <Form.Item
            name="sla_hours"
            label="SLA 时效(小时)"
            rules={[{ required: true, message: '请输入时效' }]}
          >
            <InputNumber min={1} max={72} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="severity"
            label="优先级"
            rules={[{ required: true, message: '请选择优先级' }]}
          >
            <Select
              options={[
                { label: '低', value: 'LOW' },
                { label: '中', value: 'MEDIUM' },
                { label: '高', value: 'HIGH' }
              ]}
            />
          </Form.Item>
          <Form.Item name="enabled" label="启用状态" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="工单分派"
        open={assignModalOpen}
        onCancel={() => setAssignModalOpen(false)}
        onOk={handleAssign}
        okText="确认分派"
      >
        <Form form={assignForm} layout="vertical">
          <Form.Item
            name="assignee"
            label="负责人"
            rules={[{ required: true, message: '请输入负责人' }]}
          >
            <Input placeholder="例如：物流客服A" />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="处理结果"
        open={resolveModalOpen}
        onCancel={() => setResolveModalOpen(false)}
        onOk={handleResolve}
        okText="确认完成"
      >
        <Form form={resolveForm} layout="vertical">
          <Form.Item
            name="resolution"
            label="处理说明"
            rules={[{ required: true, message: '请输入处理说明' }]}
          >
            <Input.TextArea rows={3} placeholder="例如：重新安排派送并通知客户" />
          </Form.Item>
          <Form.Item label="回写订单">
            <Select options={typeOptions} disabled placeholder="回写异常类型" />
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};
