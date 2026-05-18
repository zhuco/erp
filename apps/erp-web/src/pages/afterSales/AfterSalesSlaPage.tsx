import {
  Button,
  Col,
  Form,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import type { AfterSalesType } from '@/api/afterSalesOps';
import type {
  AfterSalesSlaRule,
  AfterSalesSlaRuleListQuery,
  AfterSalesSlaRulePayload,
  AfterSalesSlaRuleUpdatePayload,
  AfterSalesSlaStatus
} from '@/api/afterSalesWorkflow';
import {
  createAfterSalesSlaRule,
  deleteAfterSalesSlaRule,
  listAfterSalesSlaRules,
  updateAfterSalesSlaRule
} from '@/api/afterSalesWorkflow';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  after_sales_type?: AfterSalesType;
  status?: AfterSalesSlaStatus;
};

type RuleValues = {
  after_sales_type: AfterSalesType;
  response_hours: number;
  resolution_hours: number;
  escalation_role?: string;
  status?: AfterSalesSlaStatus;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const typeOptions = [
  { label: '仅退款', value: 'REFUND_ONLY' },
  { label: '退货退款', value: 'RETURN_REFUND' },
  { label: '换货', value: 'EXCHANGE' },
  { label: '补发', value: 'RESEND' }
];

const statusColors: Record<AfterSalesSlaStatus, string> = {
  ACTIVE: 'green',
  DISABLED: 'default'
};

export const AfterSalesSlaPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('aftersales.write');
  const [form] = Form.useForm<FilterValues>();
  const [ruleForm] = Form.useForm<RuleValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<IndustrialTablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [modalOpen, setModalOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [editingRule, setEditingRule] = useState<AfterSalesSlaRule | null>(null);
  const [deletingRule, setDeletingRule] = useState<string | null>(null);

  const query = useMemo<AfterSalesSlaRuleListQuery>(() => {
    return {
      after_sales_type: filters.after_sales_type,
      status: filters.status,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters.after_sales_type, filters.status, pagination.current, pagination.pageSize]);

  const { data, isFetching, error, refetch } = useQuery({
    queryKey: ['after-sales-sla-rules', tenantId, query],
    queryFn: () => listAfterSalesSlaRules(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const rules = data?.items ?? [];
  const activeCount = rules.filter((rule) => rule.status === 'ACTIVE').length;
  const disabledCount = rules.filter((rule) => rule.status === 'DISABLED').length;

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openRuleModal = (rule?: AfterSalesSlaRule) => {
    if (rule) {
      setEditingRule(rule);
      ruleForm.setFieldsValue({
        after_sales_type: rule.after_sales_type,
        response_hours: rule.response_hours,
        resolution_hours: rule.resolution_hours,
        escalation_role: rule.escalation_role ?? undefined,
        status: rule.status
      });
    } else {
      setEditingRule(null);
      ruleForm.resetFields();
      ruleForm.setFieldsValue({
        status: 'ACTIVE',
        response_hours: 4,
        resolution_hours: 24
      });
    }
    setModalOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await ruleForm.validateFields();
      setSaving(true);
      if (editingRule) {
        const payload: AfterSalesSlaRuleUpdatePayload = {
          after_sales_type: values.after_sales_type,
          response_hours: values.response_hours,
          resolution_hours: values.resolution_hours,
          escalation_role: values.escalation_role?.trim() || null,
          status: values.status
        };
        await updateAfterSalesSlaRule(tenantId, editingRule.rule_id, payload);
        message.success('SLA规则已更新');
      } else {
        const payload: AfterSalesSlaRulePayload = {
          after_sales_type: values.after_sales_type,
          response_hours: values.response_hours,
          resolution_hours: values.resolution_hours,
          escalation_role: values.escalation_role?.trim() || null,
          status: values.status
        };
        await createAfterSalesSlaRule(tenantId, payload);
        message.success('SLA规则已创建');
      }
      setModalOpen(false);
      setEditingRule(null);
      ruleForm.resetFields();
      await refetch();
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (ruleId: string) => {
    setDeletingRule(ruleId);
    try {
      await deleteAfterSalesSlaRule(tenantId, ruleId);
      message.success('SLA规则已删除');
      await refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除失败');
    } finally {
      setDeletingRule(null);
    }
  };

  const columns = useMemo<ColumnsType<AfterSalesSlaRule>>(
    () => [
      { title: '规则编号', dataIndex: 'rule_id' },
      {
        title: '售后类型',
        dataIndex: 'after_sales_type',
        render: (value: AfterSalesType) =>
          typeOptions.find((item) => item.value === value)?.label ?? value
      },
      { title: '响应时效(小时)', dataIndex: 'response_hours' },
      { title: '关闭时效(小时)', dataIndex: 'resolution_hours' },
      { title: '升级角色', dataIndex: 'escalation_role', render: (value) => value || '-' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: AfterSalesSlaStatus) => (
          <Tag color={statusColors[value]}>{statusOptions.find((item) => item.value === value)?.label ?? value}</Tag>
        )
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => openRuleModal(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除该规则？"
              okText="删除"
              cancelText="取消"
              disabled={!canWrite}
              onConfirm={() => handleDelete(record.rule_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deletingRule === record.rule_id}
              >
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deletingRule]
  );

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }} data-testid="after-sales-sla-title">
        售后 SLA 配置
      </Typography.Title>
      <Typography.Text type="secondary">
        维护不同售后类型的 SLA 响应、关闭与升级规则。
      </Typography.Text>
    </Space>
  );

  const toolbarRight = (
    <Space>
      <Button onClick={() => void refetch()} loading={isFetching}>
        刷新
      </Button>
      {canWrite ? (
        <Button type="primary" data-testid="after-sales-sla-create" onClick={() => openRuleModal()}>
          新增规则
        </Button>
      ) : null}
    </Space>
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" size={24} wrap>
        <div>
          <Typography.Text type="secondary">规则总数</Typography.Text>
          <div>{rules.length}</div>
        </div>
        <div>
          <Typography.Text type="secondary">启用规则</Typography.Text>
          <div>{activeCount}</div>
        </div>
        <div>
          <Typography.Text type="secondary">停用规则</Typography.Text>
          <div>{disabledCount}</div>
        </div>
      </Space>
      <Form form={form} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item label="售后类型" name="after_sales_type">
              <Select allowClear options={typeOptions} placeholder="选择售后类型" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item label="状态" name="status">
              <Select allowClear options={statusOptions} placeholder="选择状态" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
            <Space>
              <Button type="primary" htmlType="submit">
                查询
              </Button>
              <Button onClick={handleReset}>重置</Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<AfterSalesSlaRule>
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        header={header}
        rowKey="rule_id"
        columns={columns}
        data={rules}
        loading={isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="SLA规则加载失败"
        emptyText="暂无SLA规则"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        dataTestId="after-sales-sla-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        columnSettingsKey="after-sales.sla.columns"
      />
      <Modal
        title={editingRule ? '编辑SLA规则' : '新增SLA规则'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSave}
        okText="保存"
        okButtonProps={{ 'data-testid': 'after-sales-sla-submit' }}
        confirmLoading={saving}
      >
        <Form form={ruleForm} layout="vertical">
          <Form.Item
            name="after_sales_type"
            label="售后类型"
            rules={[{ required: true, message: '请选择售后类型' }]}
          >
            <Select data-testid="after-sales-sla-type" options={typeOptions} />
          </Form.Item>
          <Form.Item
            name="response_hours"
            label="响应时效(小时)"
            rules={[{ required: true, message: '请输入响应时效' }]}
          >
            <InputNumber
              data-testid="after-sales-sla-response"
              min={1}
              max={168}
              style={{ width: '100%' }}
            />
          </Form.Item>
          <Form.Item
            name="resolution_hours"
            label="关闭时效(小时)"
            rules={[{ required: true, message: '请输入关闭时效' }]}
          >
            <InputNumber
              data-testid="after-sales-sla-resolution"
              min={1}
              max={720}
              style={{ width: '100%' }}
            />
          </Form.Item>
          <Form.Item name="escalation_role" label="升级角色">
            <Input data-testid="after-sales-sla-role" placeholder="例如：售后主管" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select data-testid="after-sales-sla-status" options={statusOptions} />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
