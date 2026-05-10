import { Button, Form, Input, Modal, Select, Space, Table, Tag, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  LogisticsInterface,
  LogisticsInterfaceApproval,
  LogisticsInterfaceApprovalStatus,
  LogisticsInterfaceStatus
} from '@/api/logisticsInterfaces';
import {
  createLogisticsInterface,
  createLogisticsInterfaceApproval,
  listLogisticsInterfaceApprovals,
  listLogisticsInterfaces,
  updateLogisticsInterface
} from '@/api/logisticsInterfaces';
import { SettingsPageTemplate, SettingsPageSection } from '@/components/page-templates';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type InterfaceFormValues = {
  carrier_code: string;
  carrier_name: string;
  api_base_url: string;
  template_name: string;
  retry_policy: string;
  alert_channel: string;
  notes?: string;
};

const statusColors: Record<LogisticsInterfaceStatus, string> = {
  HEALTHY: 'green',
  WARNING: 'orange',
  DOWN: 'red'
};

const statusLabels: Record<LogisticsInterfaceStatus, string> = {
  HEALTHY: '健康',
  WARNING: '波动',
  DOWN: '不可用'
};

const approvalColors: Record<LogisticsInterfaceApprovalStatus, string> = {
  APPROVED: 'green',
  PENDING: 'gold'
};

const approvalLabels: Record<LogisticsInterfaceApprovalStatus, string> = {
  APPROVED: '已审批',
  PENDING: '待审批'
};

const statusCycle: Record<LogisticsInterfaceStatus, LogisticsInterfaceStatus> = {
  HEALTHY: 'WARNING',
  WARNING: 'DOWN',
  DOWN: 'HEALTHY'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const LogisticsInterfacesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('shipping.write');
  const queryClient = useQueryClient();
  const [modalOpen, setModalOpen] = useState(false);
  const [approvalOpen, setApprovalOpen] = useState(false);
  const [editing, setEditing] = useState<LogisticsInterface | null>(null);
  const [activeInterfaceId, setActiveInterfaceId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm<InterfaceFormValues>();

  const interfacesQuery = useQuery({
    queryKey: ['logistics-interfaces', tenantId],
    queryFn: () =>
      listLogisticsInterfaces(tenantId, {
        offset: 0,
        limit: 200
      })
  });

  const approvalsQuery = useQuery({
    queryKey: ['logistics-interface-approvals', tenantId, activeInterfaceId],
    queryFn: () =>
      listLogisticsInterfaceApprovals(tenantId, activeInterfaceId as string),
    enabled: Boolean(activeInterfaceId)
  });

  useEffect(() => {
    if (!modalOpen) {
      return;
    }
    if (editing) {
      form.setFieldsValue(editing);
    } else {
      form.resetFields();
      form.setFieldsValue({
        carrier_code: '',
        carrier_name: '',
        api_base_url: '',
        template_name: '',
        retry_policy: '失败重试 3 次/10 分钟',
        alert_channel: '钉钉-物流群'
      });
    }
  }, [editing, form, modalOpen]);

  const configs = interfacesQuery.data?.items ?? [];
  const approvals = approvalsQuery.data ?? [];

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      const now = dayjs().toISOString();
      setSaving(true);
      if (editing) {
        await updateLogisticsInterface(tenantId, editing.interface_id, {
          carrier_name: values.carrier_name,
          api_base_url: values.api_base_url,
          template_name: values.template_name,
          retry_policy: values.retry_policy,
          alert_channel: values.alert_channel,
          approval_status: 'PENDING',
          approved_at: null,
          approved_by: null,
          notes: values.notes ?? null
        });
        await createLogisticsInterfaceApproval(tenantId, editing.interface_id, {
          action: '变更提交',
          operator: '物流管理员',
          comment: values.notes ?? ''
        });
      } else {
        const created = await createLogisticsInterface(tenantId, {
          carrier_code: values.carrier_code,
          carrier_name: values.carrier_name,
          api_base_url: values.api_base_url,
          template_name: values.template_name,
          status: 'HEALTHY',
          last_checked_at: now,
          retry_policy: values.retry_policy,
          alert_channel: values.alert_channel,
          approval_status: 'PENDING',
          notes: values.notes ?? null
        });
        await createLogisticsInterfaceApproval(tenantId, created.interface_id, {
          action: '创建接口配置',
          operator: '物流管理员',
          comment: values.notes ?? ''
        });
      }
      message.success('接口配置已保存，等待审批');
      setModalOpen(false);
      setEditing(null);
      await queryClient.invalidateQueries({ queryKey: ['logistics-interfaces', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleHealthCheck = async (record: LogisticsInterface) => {
    const nextStatus = statusCycle[record.status as LogisticsInterfaceStatus] ?? 'HEALTHY';
    const now = dayjs().toISOString();
    try {
      await updateLogisticsInterface(tenantId, record.interface_id, {
        status: nextStatus,
        last_checked_at: now
      });
      message.success(`健康检查完成，状态：${statusLabels[nextStatus]}`);
      await queryClient.invalidateQueries({ queryKey: ['logistics-interfaces', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '健康检查失败');
    }
  };

  const handleApprove = async (record: LogisticsInterface) => {
    if (!canWrite) {
      message.warning('暂无权限');
      return;
    }
    const now = dayjs().toISOString();
    try {
      await updateLogisticsInterface(tenantId, record.interface_id, {
        approval_status: 'APPROVED',
        approved_by: '物流负责人',
        approved_at: now
      });
      await createLogisticsInterfaceApproval(tenantId, record.interface_id, {
        action: '审批通过',
        operator: '物流负责人',
        comment: '接口变更已确认'
      });
      message.success('审批已通过');
      await queryClient.invalidateQueries({ queryKey: ['logistics-interfaces', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '审批失败');
    }
  };

  const columns = useMemo<ColumnsType<LogisticsInterface>>(
    () => [
      { title: '承运商', dataIndex: 'carrier_name' },
      { title: '接口地址', dataIndex: 'api_base_url', ellipsis: true },
      { title: '面单模板', dataIndex: 'template_name' },
      {
        title: '健康状态',
        dataIndex: 'status',
        render: (value: LogisticsInterfaceStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        )
      },
      { title: '失败重试', dataIndex: 'retry_policy' },
      { title: '告警通道', dataIndex: 'alert_channel' },
      {
        title: '审批状态',
        dataIndex: 'approval_status',
        render: (value: LogisticsInterfaceApprovalStatus) => (
          <Tag color={approvalColors[value]}>{approvalLabels[value]}</Tag>
        )
      },
      {
        title: '上次检测',
        dataIndex: 'last_checked_at',
        render: (value: string) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space size={8}>
            <Button size="small" onClick={() => handleHealthCheck(record)}>
              健康检查
            </Button>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setEditing(record);
                setModalOpen(true);
              }}
            >
              编辑
            </Button>
            <Button
              size="small"
              onClick={() => {
                setActiveInterfaceId(record.interface_id);
                setApprovalOpen(true);
              }}
            >
              审批记录
            </Button>
            <Button
              size="small"
              disabled={!canWrite || record.approval_status === 'APPROVED'}
              onClick={() => handleApprove(record)}
            >
              审批通过
            </Button>
          </Space>
        )
      }
    ],
    [canWrite]
  );

  const approvalColumns: ColumnsType<LogisticsInterfaceApproval> = [
    { title: '动作', dataIndex: 'action' },
    { title: '操作人', dataIndex: 'operator' },
    { title: '说明', dataIndex: 'comment', ellipsis: true },
    { title: '时间', dataIndex: 'created_at', render: (value: string) => formatDateTime(value) }
  ];

  return (
    <>
      <SettingsPageTemplate title="物流接口配置">
        <SettingsPageSection
          title="接口列表"
          actions={
            <Button
              type="primary"
              disabled={!canWrite}
              onClick={() => {
                setEditing(null);
                setModalOpen(true);
              }}
            >
              新增接口
            </Button>
          }
        >
          <Table
            rowKey="interface_id"
            columns={columns}
            dataSource={configs}
            pagination={false}
            loading={interfacesQuery.isLoading}
          />
        </SettingsPageSection>
      </SettingsPageTemplate>
      <Modal
        title={editing ? '编辑接口' : '新增接口'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSave}
        okText="保存"
        confirmLoading={saving}
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="carrier_code"
            label="承运商编码"
            rules={[{ required: true, message: '请输入承运商编码' }]}
          >
            <Input placeholder="例如：SF" disabled={Boolean(editing)} />
          </Form.Item>
          <Form.Item
            name="carrier_name"
            label="承运商名称"
            rules={[{ required: true, message: '请输入承运商名称' }]}
          >
            <Input placeholder="例如：顺丰速运" />
          </Form.Item>
          <Form.Item
            name="api_base_url"
            label="接口地址"
            rules={[{ required: true, message: '请输入接口地址' }]}
          >
            <Input placeholder="https://api.example.com" />
          </Form.Item>
          <Form.Item
            name="template_name"
            label="面单模板"
            rules={[{ required: true, message: '请输入面单模板' }]}
          >
            <Input placeholder="模板名称" />
          </Form.Item>
          <Form.Item
            name="retry_policy"
            label="失败重试策略"
            rules={[{ required: true, message: '请输入重试策略' }]}
          >
            <Input placeholder="例如：失败重试 3 次/10 分钟" />
          </Form.Item>
          <Form.Item
            name="alert_channel"
            label="告警通道"
            rules={[{ required: true, message: '请选择告警通道' }]}
          >
            <Select
              options={[
                { label: '钉钉-物流群', value: '钉钉-物流群' },
                { label: '企业微信-履约预警', value: '企业微信-履约预警' },
                { label: '短信通知', value: '短信通知' }
              ]}
            />
          </Form.Item>
          <Form.Item name="notes" label="变更说明">
            <Input.TextArea rows={3} placeholder="记录接口变更原因与审批说明" />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="审批记录"
        open={approvalOpen}
        onCancel={() => {
          setApprovalOpen(false);
          setActiveInterfaceId(null);
        }}
        footer={[
          <Button key="close" onClick={() => setApprovalOpen(false)}>
            关闭
          </Button>
        ]}
      >
        <Table
          rowKey="approval_id"
          dataSource={approvals}
          pagination={false}
          columns={approvalColumns}
          loading={approvalsQuery.isLoading}
        />
      </Modal>
    </>
  );
};
