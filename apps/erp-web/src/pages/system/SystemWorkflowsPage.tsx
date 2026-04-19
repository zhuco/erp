import {
  Button,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Table,
  Tag,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  getSystemWorkflowCatalog,
  updateSystemWorkflowCatalog,
  type NumberRule,
  type NumberRuleStatus,
  type SystemWorkflowCatalog,
  type WorkflowDefinition,
  type WorkflowGovernance,
  type WorkflowPublishStatus,
  type WorkflowStatus
} from '@/api/systemWorkflows';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/QueryState';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type NumberRuleFormValues = {
  doc_type: string;
  org_scope: string;
  prefix: string;
  date_format: string;
  sequence_length: number;
  next_number: number;
  status: NumberRuleStatus;
};

type WorkflowFormValues = {
  doc_type: string;
  name: string;
  version: string;
  status: WorkflowStatus;
  steps: string[];
  engine: string;
  process_key: string;
  definition_id?: string;
  decision_key?: string;
  publish_status: WorkflowPublishStatus;
  approval_mode: string;
  effective_at?: string;
  last_deploy_result?: string;
};

const numberStatusColors: Record<NumberRuleStatus, string> = {
  ACTIVE: 'green',
  DISABLED: 'default'
};

const workflowStatusColors: Record<WorkflowStatus, string> = {
  ACTIVE: 'green',
  DRAFT: 'gold',
  ARCHIVED: 'default'
};

const publishStatusColors: Record<string, string> = {
  DRAFT: 'gold',
  PUBLISHED: 'green',
  ARCHIVED: 'default',
  FAILED: 'red',
  PENDING: 'processing'
};

const docTypeOptions = [
  { label: '采购申请', value: '采购申请' },
  { label: '采购订单', value: '采购订单' },
  { label: '调拨单', value: '调拨单' },
  { label: '出库单', value: '出库单' },
  { label: '售后单', value: '售后单' }
];

const formatDateTime = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

const buildPreview = (rule: NumberRule) => {
  const datePart = dayjs().format(rule.date_format);
  const numberPart = String(rule.next_number).padStart(rule.sequence_length, '0');
  return `${rule.prefix}-${datePart}-${numberPart}`;
};

const toNullableText = (value?: string) => {
  const trimmed = value?.trim();
  return trimmed ? trimmed : undefined;
};

const mergeGovernance = (
  values: WorkflowFormValues,
  base?: WorkflowGovernance
): WorkflowGovernance => ({
  engine: values.engine,
  process_key: values.process_key,
  definition_id: toNullableText(values.definition_id) ?? base?.definition_id ?? null,
  decision_key: toNullableText(values.decision_key) ?? base?.decision_key ?? null,
  publish_status: values.publish_status,
  approval_mode: values.approval_mode,
  effective_at: toNullableText(values.effective_at) ?? base?.effective_at ?? null,
  last_deploy_at: base?.last_deploy_at ?? null,
  last_deploy_result: toNullableText(values.last_deploy_result) ?? base?.last_deploy_result ?? null
});

export const SystemWorkflowsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('SYS.SETTINGS.WRITE');
  const queryClient = useQueryClient();
  const [rules, setRules] = useState<NumberRule[]>([]);
  const [workflows, setWorkflows] = useState<WorkflowDefinition[]>([]);
  const [ruleModalOpen, setRuleModalOpen] = useState(false);
  const [workflowModalOpen, setWorkflowModalOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<NumberRule | null>(null);
  const [editingWorkflow, setEditingWorkflow] = useState<WorkflowDefinition | null>(null);
  const [ruleForm] = Form.useForm<NumberRuleFormValues>();
  const [workflowForm] = Form.useForm<WorkflowFormValues>();

  const catalogQuery = useQuery({
    queryKey: ['system-workflows', tenantId],
    queryFn: () => getSystemWorkflowCatalog(tenantId),
    enabled: tenantReady
  });

  useEffect(() => {
    if (!tenantReady) {
      setRules([]);
      setWorkflows([]);
      return;
    }
    if (catalogQuery.data) {
      setRules(catalogQuery.data.rules);
      setWorkflows(catalogQuery.data.workflows);
    }
  }, [catalogQuery.data, tenantReady]);

  const saveMutation = useMutation({
    mutationFn: (payload: SystemWorkflowCatalog) => updateSystemWorkflowCatalog(tenantId, payload),
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存单据编号与流程配置失败');
    }
  });

  const catalogReady = tenantReady && !!catalogQuery.data;
  const writeDisabled = !canWrite || !catalogReady || saveMutation.isPending;
  const tableLoading = (catalogQuery.isLoading && !catalogQuery.data) || saveMutation.isPending;

  const persistCatalog = async (payload: SystemWorkflowCatalog, successMessage: string) => {
    const saved = await saveMutation.mutateAsync(payload);
    setRules(saved.rules);
    setWorkflows(saved.workflows);
    await queryClient.invalidateQueries({ queryKey: ['system-workflows', tenantId] });
    message.success(successMessage);
  };

  const closeRuleModal = () => {
    setRuleModalOpen(false);
    setEditingRule(null);
  };

  const closeWorkflowModal = () => {
    setWorkflowModalOpen(false);
    setEditingWorkflow(null);
  };

  const openRuleModal = (record?: NumberRule) => {
    setEditingRule(record ?? null);
    ruleForm.resetFields();
    if (record) {
      ruleForm.setFieldsValue(record);
    } else {
      ruleForm.setFieldsValue({
        doc_type: docTypeOptions[0].value,
        org_scope: '总部',
        prefix: 'DOC',
        date_format: 'YYYYMM',
        sequence_length: 4,
        next_number: 1,
        status: 'ACTIVE'
      });
    }
    setRuleModalOpen(true);
  };

  const openWorkflowModal = (record?: WorkflowDefinition) => {
    setEditingWorkflow(record ?? null);
    workflowForm.resetFields();
    if (record) {
      workflowForm.setFieldsValue({
        doc_type: record.doc_type,
        name: record.name,
        version: record.version,
        status: record.status,
        steps: record.steps,
        engine: record.governance?.engine ?? 'CAMUNDA8',
        process_key: record.governance?.process_key ?? '',
        definition_id: record.governance?.definition_id ?? undefined,
        decision_key: record.governance?.decision_key ?? undefined,
        publish_status: (record.governance?.publish_status as WorkflowPublishStatus) ?? 'DRAFT',
        approval_mode: record.governance?.approval_mode ?? 'HUMAN_TASK',
        effective_at: record.governance?.effective_at ?? undefined,
        last_deploy_result: record.governance?.last_deploy_result ?? undefined
      });
    } else {
      workflowForm.setFieldsValue({
        doc_type: docTypeOptions[0].value,
        name: '审批流程',
        version: `v${dayjs().format('YY.MM')}`,
        status: 'DRAFT',
        steps: [],
        engine: 'CAMUNDA8',
        process_key: `wf_${dayjs().format('YYYYMMDD_HHmmss')}`,
        publish_status: 'DRAFT',
        approval_mode: 'HUMAN_TASK',
        last_deploy_result: 'PENDING'
      });
    }
    setWorkflowModalOpen(true);
  };

  const handleSaveRule = async () => {
    try {
      const values = await ruleForm.validateFields();
      const updatedAt = dayjs().toISOString();
      const nextRules = editingRule
        ? rules.map((item) =>
            item.rule_id === editingRule.rule_id
              ? { ...item, ...values, updated_at: updatedAt }
              : item
          )
        : [
            {
              rule_id: `RULE-${Date.now()}`,
              ...values,
              updated_at: updatedAt
            },
            ...rules
          ];
      await persistCatalog({ rules: nextRules, workflows }, editingRule ? '编号规则已更新' : '编号规则已创建');
      closeRuleModal();
    } catch {
      return;
    }
  };

  const handleSaveWorkflow = async () => {
    try {
      const values = await workflowForm.validateFields();
      const updatedAt = dayjs().toISOString();
      let activeId = editingWorkflow?.workflow_id ?? '';

      const nextWorkflows = (() => {
        if (editingWorkflow) {
          return workflows.map((item) =>
            item.workflow_id === editingWorkflow.workflow_id
              ? {
                  ...item,
                  doc_type: values.doc_type,
                  name: values.name,
                  version: values.version,
                  status: values.status,
                  steps: values.steps ?? [],
                  governance: mergeGovernance(values, item.governance),
                  updated_at: updatedAt
                }
              : item
          );
        }

        const workflowId = `WF-${Date.now()}`;
        activeId = workflowId;
        return [
          {
            workflow_id: workflowId,
            doc_type: values.doc_type,
            name: values.name,
            version: values.version,
            status: values.status,
            steps: values.steps ?? [],
            governance: mergeGovernance(values),
            updated_at: updatedAt
          },
          ...workflows
        ];
      })();

      const normalized = values.status === 'ACTIVE'
        ? nextWorkflows.map((item) => {
            if (item.doc_type === values.doc_type && item.workflow_id !== activeId) {
              return {
                ...item,
                status: 'ARCHIVED' as WorkflowStatus,
                governance: item.governance
                  ? { ...item.governance, publish_status: 'ARCHIVED' }
                  : item.governance
              };
            }
            return item;
          })
        : nextWorkflows;

      await persistCatalog({ rules, workflows: normalized }, editingWorkflow ? '流程配置已更新' : '流程配置已新增');
      closeWorkflowModal();
    } catch {
      return;
    }
  };

  const handleActivateWorkflow = async (record: WorkflowDefinition) => {
    if (!canWrite) {
      message.warning('暂无权限');
      return;
    }

    const activatedAt = dayjs().toISOString();
    const nextWorkflows = workflows.map((item) => {
      if (item.workflow_id === record.workflow_id) {
        return {
          ...item,
          status: 'ACTIVE' as WorkflowStatus,
          governance: {
            engine: item.governance?.engine ?? 'CAMUNDA8',
            process_key: item.governance?.process_key ?? record.workflow_id.toLowerCase(),
            definition_id: item.governance?.definition_id ?? null,
            decision_key: item.governance?.decision_key ?? null,
            publish_status: 'PUBLISHED',
            approval_mode: item.governance?.approval_mode ?? 'HUMAN_TASK',
            effective_at: activatedAt,
            last_deploy_at: activatedAt,
            last_deploy_result: 'SUCCESS'
          },
          updated_at: activatedAt
        };
      }
      if (item.doc_type === record.doc_type) {
        return {
          ...item,
          status: 'ARCHIVED' as WorkflowStatus,
          governance: item.governance
            ? { ...item.governance, publish_status: 'ARCHIVED' }
            : item.governance
        };
      }
      return item;
    });

    try {
      await persistCatalog({ rules, workflows: nextWorkflows }, '流程版本已发布并即时生效');
    } catch {
      return;
    }
  };

  const ruleColumns = useMemo<ColumnsType<NumberRule>>(
    () => [
      { title: '单据类型', dataIndex: 'doc_type' },
      { title: '组织范围', dataIndex: 'org_scope' },
      { title: '编号前缀', dataIndex: 'prefix' },
      { title: '日期格式', dataIndex: 'date_format' },
      { title: '序列长度', dataIndex: 'sequence_length' },
      { title: '预览', key: 'preview', render: (_, record) => buildPreview(record) },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: NumberRuleStatus) => <Tag color={numberStatusColors[value]}>{value}</Tag>
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
          <Button size="small" disabled={writeDisabled} onClick={() => openRuleModal(record)}>
            编辑
          </Button>
        )
      }
    ],
    [writeDisabled]
  );

  const workflowColumns = useMemo<ColumnsType<WorkflowDefinition>>(
    () => [
      { title: '单据类型', dataIndex: 'doc_type' },
      { title: '流程名称', dataIndex: 'name' },
      { title: '版本', dataIndex: 'version' },
      {
        title: '流程状态',
        dataIndex: 'status',
        render: (value: WorkflowStatus) => <Tag color={workflowStatusColors[value]}>{value}</Tag>
      },
      {
        title: '发布状态',
        key: 'publish_status',
        render: (_, record) => {
          const publishStatus = record.governance?.publish_status ?? 'DRAFT';
          return <Tag color={publishStatusColors[publishStatus] ?? 'default'}>{publishStatus}</Tag>;
        }
      },
      {
        title: '治理绑定',
        key: 'binding',
        render: (_, record) =>
          record.governance ? `${record.governance.engine} / ${record.governance.process_key}` : '-'
      },
      {
        title: '审批节点',
        dataIndex: 'steps',
        render: (value: string[]) => (value.length ? value.join(' -> ') : '-')
      },
      {
        title: '生效时间',
        key: 'effective_at',
        render: (_, record) => formatDateTime(record.governance?.effective_at)
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
            <Button size="small" disabled={writeDisabled} onClick={() => openWorkflowModal(record)}>
              编辑
            </Button>
            <Button
              size="small"
              disabled={writeDisabled || record.status === 'ACTIVE'}
              onClick={() => void handleActivateWorkflow(record)}
            >
              发布
            </Button>
          </Space>
        )
      }
    ],
    [writeDisabled]
  );

  if (!tenantReady) {
    return (
      <UiModule07SettingsLayout title="单据编号与流程配置">
        <EmptyState description="请先选择租户" />
      </UiModule07SettingsLayout>
    );
  }

  return (
    <UiModule07SettingsLayout title="单据编号与流程配置">
      {catalogQuery.isError ? (
        <ErrorState
          title="加载单据编号与流程配置失败"
          error={catalogQuery.error}
          onRetry={() => void catalogQuery.refetch()}
        />
      ) : null}
      <UiModule07SettingsSection
        title="编号规则"
        actions={
          <Button type="primary" disabled={writeDisabled} onClick={() => openRuleModal()}>
            新增规则
          </Button>
        }
      >
        <Table rowKey="rule_id" columns={ruleColumns} dataSource={rules} pagination={false} loading={tableLoading} />
      </UiModule07SettingsSection>
      <UiModule07SettingsSection
        title="审批流程治理"
        actions={
          <Button type="primary" disabled={writeDisabled} onClick={() => openWorkflowModal()}>
            新增流程
          </Button>
        }
      >
        <Table
          rowKey="workflow_id"
          columns={workflowColumns}
          dataSource={workflows}
          pagination={false}
          loading={tableLoading}
          data-testid="system-workflows-governance-table"
        />
      </UiModule07SettingsSection>
      <Modal
        title={editingRule ? '编辑编号规则' : '新增编号规则'}
        open={ruleModalOpen}
        onCancel={closeRuleModal}
        onOk={() => void handleSaveRule()}
        okText="保存"
        okButtonProps={{ loading: saveMutation.isPending }}
        cancelButtonProps={{ disabled: saveMutation.isPending }}
        destroyOnHidden
      >
        <Form form={ruleForm} layout="vertical">
          <Form.Item
            name="doc_type"
            label="单据类型"
            rules={[{ required: true, message: '请选择单据类型' }]}
          >
            <Select options={docTypeOptions} />
          </Form.Item>
          <Form.Item
            name="org_scope"
            label="组织范围"
            rules={[{ required: true, message: '请输入组织范围' }]}
          >
            <Input placeholder="例如：总部 / 华东事业部" />
          </Form.Item>
          <Form.Item
            name="prefix"
            label="编号前缀"
            rules={[{ required: true, message: '请输入编号前缀' }]}
          >
            <Input placeholder="例如：PO" />
          </Form.Item>
          <Form.Item
            name="date_format"
            label="日期格式"
            rules={[{ required: true, message: '请输入日期格式' }]}
          >
            <Input placeholder="YYYYMMDD" />
          </Form.Item>
          <Form.Item
            name="sequence_length"
            label="序列长度"
            rules={[{ required: true, message: '请输入序列长度' }]}
          >
            <InputNumber min={2} max={10} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="next_number"
            label="下一个编号"
            rules={[{ required: true, message: '请输入下一个编号' }]}
          >
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select
              options={[
                { label: '生效', value: 'ACTIVE' },
                { label: '停用', value: 'DISABLED' }
              ]}
            />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title={editingWorkflow ? '编辑流程' : '新增流程'}
        open={workflowModalOpen}
        onCancel={closeWorkflowModal}
        onOk={() => void handleSaveWorkflow()}
        okText="保存"
        okButtonProps={{ loading: saveMutation.isPending }}
        cancelButtonProps={{ disabled: saveMutation.isPending }}
        destroyOnHidden
      >
        <Form form={workflowForm} layout="vertical">
          <Form.Item
            name="doc_type"
            label="单据类型"
            rules={[{ required: true, message: '请选择单据类型' }]}
          >
            <Select options={docTypeOptions} />
          </Form.Item>
          <Form.Item
            name="name"
            label="流程名称"
            rules={[{ required: true, message: '请输入流程名称' }]}
          >
            <Input placeholder="例如：采购订单审批流程" />
          </Form.Item>
          <Form.Item
            name="version"
            label="流程版本"
            rules={[{ required: true, message: '请输入流程版本' }]}
          >
            <Input placeholder="例如：v2.0" />
          </Form.Item>
          <Form.Item name="status" label="流程状态">
            <Select
              options={[
                { label: '草稿', value: 'DRAFT' },
                { label: '生效', value: 'ACTIVE' },
                { label: '归档', value: 'ARCHIVED' }
              ]}
            />
          </Form.Item>
          <Form.Item name="steps" label="审批节点">
            <Select mode="tags" placeholder="输入审批节点，例如：采购经理" />
          </Form.Item>
          <Form.Item
            name="engine"
            label="流程引擎"
            rules={[{ required: true, message: '请选择流程引擎' }]}
          >
            <Select options={[{ label: 'Camunda 8', value: 'CAMUNDA8' }]} />
          </Form.Item>
          <Form.Item
            name="process_key"
            label="流程键（BPMN）"
            rules={[{ required: true, message: '请输入流程键' }]}
          >
            <Input placeholder="例如：purchase_order_approval" />
          </Form.Item>
          <Form.Item name="definition_id" label="定义 ID">
            <Input placeholder="例如：purchase_order_approval:21:default" />
          </Form.Item>
          <Form.Item name="decision_key" label="规则键（DMN）">
            <Input placeholder="例如：purchase_order_gate" />
          </Form.Item>
          <Form.Item name="publish_status" label="发布状态">
            <Select
              options={[
                { label: '草稿', value: 'DRAFT' },
                { label: '已发布', value: 'PUBLISHED' },
                { label: '已归档', value: 'ARCHIVED' },
                { label: '发布失败', value: 'FAILED' }
              ]}
            />
          </Form.Item>
          <Form.Item
            name="approval_mode"
            label="审批模式"
            rules={[{ required: true, message: '请输入审批模式' }]}
          >
            <Select
              options={[
                { label: '人工任务', value: 'HUMAN_TASK' },
                { label: '自动审批', value: 'AUTO' }
              ]}
            />
          </Form.Item>
          <Form.Item name="effective_at" label="生效时间（ISO8601）">
            <Input placeholder="例如：2026-04-09T08:10:00Z" />
          </Form.Item>
          <Form.Item name="last_deploy_result" label="最近部署结果">
            <Input placeholder="例如：SUCCESS / FAILED / PENDING" />
          </Form.Item>
        </Form>
      </Modal>
    </UiModule07SettingsLayout>
  );
};
