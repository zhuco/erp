import {
  Button,
  Form,
  Input,
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
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  getSystemCustomFieldsCatalog,
  updateSystemCustomFieldsCatalog,
  type CustomField,
  type FieldAudit,
  type FieldStatus,
  type FieldType,
  type SystemCustomFieldsCatalog
} from '@/api/systemCustomFields';
import { getSystemWorkflowCatalog } from '@/api/systemWorkflows';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/QueryState';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FieldFormValues = {
  entity: string;
  label: string;
  field_key: string;
  field_type: FieldType;
  required: boolean;
  compatible: boolean;
  status: FieldStatus;
  description?: string;
};

const fieldTypeOptions = [
  { label: '文本', value: 'TEXT' },
  { label: '数值', value: 'NUMBER' },
  { label: '日期', value: 'DATE' },
  { label: '下拉选择', value: 'SELECT' }
];

const entityOptions = [
  { label: '订单', value: '订单' },
  { label: '商品', value: '商品' },
  { label: '采购', value: '采购' },
  { label: '仓储', value: '仓储' },
  { label: '客户', value: '客户' }
];

const statusColors: Record<FieldStatus, string> = {
  ACTIVE: 'green',
  DISABLED: 'default'
};

const formatDateTime = (value: string) => dayjs(value).format('YYYY-MM-DD HH:mm');

export const SystemCustomFieldsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('SYS.SETTINGS.WRITE');
  const queryClient = useQueryClient();
  const [fields, setFields] = useState<CustomField[]>([]);
  const [audits, setAudits] = useState<FieldAudit[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [auditOpen, setAuditOpen] = useState(false);
  const [editingField, setEditingField] = useState<CustomField | null>(null);
  const [activeAuditFieldId, setActiveAuditFieldId] = useState<string | null>(null);
  const [entityFilter, setEntityFilter] = useState<string | undefined>();
  const [form] = Form.useForm<FieldFormValues>();

  const catalogQuery = useQuery({
    queryKey: ['system-custom-fields', tenantId],
    queryFn: () => getSystemCustomFieldsCatalog(tenantId),
    enabled: tenantReady
  });
  const workflowQuery = useQuery({
    queryKey: ['system-workflows', tenantId],
    queryFn: () => getSystemWorkflowCatalog(tenantId),
    enabled: tenantReady
  });

  useEffect(() => {
    if (!tenantReady) {
      setFields([]);
      setAudits([]);
      return;
    }
    if (catalogQuery.data) {
      setFields(catalogQuery.data.fields);
      setAudits(catalogQuery.data.audits);
    }
  }, [catalogQuery.data, tenantReady]);

  const saveMutation = useMutation({
    mutationFn: (payload: SystemCustomFieldsCatalog) =>
      updateSystemCustomFieldsCatalog(tenantId, payload),
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存自定义字段失败');
    }
  });

  const catalogReady = tenantReady && !!catalogQuery.data;
  const writeDisabled = !canWrite || !catalogReady || saveMutation.isPending;
  const tableLoading = (catalogQuery.isLoading && !catalogQuery.data) || saveMutation.isPending;

  const persistCatalog = async (payload: SystemCustomFieldsCatalog, successMessage: string) => {
    const saved = await saveMutation.mutateAsync(payload);
    setFields(saved.fields);
    setAudits(saved.audits);
    await queryClient.invalidateQueries({ queryKey: ['system-custom-fields', tenantId] });
    message.success(successMessage);
  };

  const closeFieldModal = () => {
    setModalOpen(false);
    setEditingField(null);
  };

  const activeAuditRecords = useMemo(
    () => audits.filter((audit) => audit.field_id === activeAuditFieldId),
    [activeAuditFieldId, audits]
  );

  const openModal = (record?: CustomField) => {
    setEditingField(record ?? null);
    form.resetFields();
    if (record) {
      form.setFieldsValue(record);
    } else {
      form.setFieldsValue({
        entity: entityOptions[0].value,
        field_type: 'TEXT',
        required: false,
        compatible: true,
        status: 'ACTIVE'
      });
    }
    setModalOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      const updatedAt = dayjs().toISOString();
      if (editingField) {
        const nextFields = fields.map((item) =>
          item.field_id === editingField.field_id
            ? { ...item, ...values, updated_at: updatedAt }
            : item
        );
        const nextAudits = [
          {
            record_id: `AUD-${Date.now()}`,
            field_id: editingField.field_id,
            action: '更新字段',
            operator: '系统管理员',
            comment: values.description,
            created_at: updatedAt
          },
          ...audits
        ];
        await persistCatalog(
          {
            fields: nextFields,
            audits: nextAudits
          },
          '字段已更新'
        );
      } else {
        const fieldId = `CF-${Date.now()}`;
        const nextFields = [{ field_id: fieldId, ...values, updated_at: updatedAt }, ...fields];
        const nextAudits = [
          {
            record_id: `AUD-${Date.now()}`,
            field_id: fieldId,
            action: '创建字段',
            operator: '系统管理员',
            comment: values.description,
            created_at: updatedAt
          },
          ...audits
        ];
        await persistCatalog(
          {
            fields: nextFields,
            audits: nextAudits
          },
          '字段已新增'
        );
      }
      closeFieldModal();
    } catch {
      return;
    }
  };

  const handleAuditView = (record: CustomField) => {
    setActiveAuditFieldId(record.field_id);
    setAuditOpen(true);
  };

  const filteredFields = useMemo(
    () => fields.filter((field) => (entityFilter ? field.entity === entityFilter : true)),
    [entityFilter, fields]
  );
  const activeGovernance = useMemo(
    () => workflowQuery.data?.workflows.find((item) => item.status === 'ACTIVE')?.governance,
    [workflowQuery.data]
  );

  const columns = useMemo<ColumnsType<CustomField>>(
    () => [
      { title: '业务对象', dataIndex: 'entity' },
      { title: '字段名称', dataIndex: 'label' },
      { title: '字段Key', dataIndex: 'field_key' },
      {
        title: '类型',
        dataIndex: 'field_type',
        render: (value: FieldType) =>
          fieldTypeOptions.find((option) => option.value === value)?.label ?? value
      },
      {
        title: '必填',
        dataIndex: 'required',
        render: (value: boolean) => (value ? '是' : '否')
      },
      {
        title: '历史兼容',
        dataIndex: 'compatible',
        render: (value: boolean) => (value ? <Tag color="blue">兼容</Tag> : '-')
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: FieldStatus) => <Tag color={statusColors[value]}>{value}</Tag>
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
          <Space size={8}>
            <Button size="small" disabled={writeDisabled} onClick={() => openModal(record)}>
              编辑
            </Button>
            <Button size="small" onClick={() => handleAuditView(record)}>
              审计记录
            </Button>
          </Space>
        )
      }
    ],
    [writeDisabled]
  );

  if (!tenantReady) {
    return (
      <UiModule07SettingsLayout title="自定义字段">
        <EmptyState description="请先选择租户" />
      </UiModule07SettingsLayout>
    );
  }

  return (
    <UiModule07SettingsLayout title="自定义字段">
      {catalogQuery.isError ? (
        <ErrorState
          title="加载自定义字段失败"
          error={catalogQuery.error}
          onRetry={() => void catalogQuery.refetch()}
        />
      ) : null}
      <UiModule07SettingsSection
        title="字段列表"
        actions={
          <Space>
            <Button
              onClick={() => message.info('已导出字段模板')}
              disabled={!canWrite || !catalogReady}
            >
              导出模板
            </Button>
            <Button
              onClick={() => message.info('字段导入功能建设中')}
              disabled={!canWrite || !catalogReady}
            >
              导入字段
            </Button>
            <Button type="primary" disabled={writeDisabled} onClick={() => openModal()}>
              新增字段
            </Button>
          </Space>
        }
      >
        <Space direction="vertical" size={16} style={{ width: '100%' }}>
          <Space wrap data-testid="system-custom-fields-governance-summary">
            <Tag color="blue">治理链路</Tag>
            <Tag color="processing">发布状态: {activeGovernance?.publish_status ?? 'DRAFT'}</Tag>
            <Tag>流程引擎: {activeGovernance?.engine ?? 'CAMUNDA8'}</Tag>
            <Tag>流程键: {activeGovernance?.process_key ?? '-'}</Tag>
            <Tag>生效时间: {activeGovernance?.effective_at ?? '-'}</Tag>
          </Space>
          <Space wrap>
            <Select
              allowClear
              placeholder="筛选业务对象"
              options={entityOptions}
              value={entityFilter}
              onChange={(value) => setEntityFilter(value)}
              style={{ width: 200 }}
            />
            <Button onClick={() => setEntityFilter(undefined)}>重置</Button>
          </Space>
          <Table
            rowKey="field_id"
            columns={columns}
            dataSource={filteredFields}
            pagination={false}
            loading={tableLoading}
          />
        </Space>
      </UiModule07SettingsSection>
      <Modal
        title={editingField ? '编辑字段' : '新增字段'}
        open={modalOpen}
        onCancel={closeFieldModal}
        onOk={() => void handleSave()}
        okText="保存"
        okButtonProps={{ loading: saveMutation.isPending }}
        cancelButtonProps={{ disabled: saveMutation.isPending }}
        destroyOnClose
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="entity"
            label="业务对象"
            rules={[{ required: true, message: '请选择业务对象' }]}
          >
            <Select options={entityOptions} />
          </Form.Item>
          <Form.Item
            name="label"
            label="字段名称"
            rules={[{ required: true, message: '请输入字段名称' }]}
          >
            <Input placeholder="例如：渠道备注" />
          </Form.Item>
          <Form.Item
            name="field_key"
            label="字段Key"
            rules={[{ required: true, message: '请输入字段Key' }]}
          >
            <Input placeholder="例如：channel_note" />
          </Form.Item>
          <Form.Item
            name="field_type"
            label="字段类型"
            rules={[{ required: true, message: '请选择字段类型' }]}
          >
            <Select options={fieldTypeOptions} />
          </Form.Item>
          <Form.Item name="required" label="必填" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="compatible" label="历史数据兼容" valuePropName="checked">
            <Switch />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select
              options={[
                { label: '生效', value: 'ACTIVE' },
                { label: '停用', value: 'DISABLED' }
              ]}
            />
          </Form.Item>
          <Form.Item name="description" label="字段说明">
            <Input.TextArea rows={3} placeholder="描述字段用途或范围" />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="字段审计记录"
        open={auditOpen}
        onCancel={() => setAuditOpen(false)}
        footer={[
          <Button key="close" onClick={() => setAuditOpen(false)}>
            关闭
          </Button>
        ]}
      >
        <Table<FieldAudit>
          rowKey="record_id"
          dataSource={activeAuditRecords}
          pagination={false}
          columns={[
            { title: '动作', dataIndex: 'action' },
            { title: '操作人', dataIndex: 'operator' },
            { title: '说明', dataIndex: 'comment', ellipsis: true },
            {
              title: '时间',
              dataIndex: 'created_at',
              render: (value: string) => formatDateTime(value)
            }
          ]}
        />
      </Modal>
    </UiModule07SettingsLayout>
  );
};
