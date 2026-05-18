import {
  Button,
  Col,
  Descriptions,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  OrderReviewRule,
  OrderReviewRuleCreatePayload
} from '@/api/orderReviewRules';
import {
  createOrderReviewRule,
  deleteOrderReviewRule,
  listOrderReviewRules,
  updateOrderReviewRule
} from '@/api/orderReviewRules';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { listSettings, upsertSetting } from '@/api/system';

type FilterValues = {
  enabled?: boolean;
  status?: string;
};

type FormValues = {
  name: string;
  action: string;
  enabled: boolean;
  priority?: number;
  reason?: string;
  conditions_json?: string;
};

type TemplateOption = {
  label: string;
  value: string;
  payload: unknown;
};

const isTemplateOption = (item: unknown): item is TemplateOption => {
  if (!item || typeof item !== 'object') {
    return false;
  }
  const value = item as { label?: unknown; value?: unknown };
  return typeof value.label === 'string' && typeof value.value === 'string';
};

const actionOptions = [
  { label: '放行', value: 'PASS' },
  { label: '拦截', value: 'FLAG' },
  { label: '拒绝', value: 'REJECT' }
];

const enabledOptions = [
  { label: '全部', value: 'ALL' },
  { label: '启用', value: 'ENABLED' },
  { label: '停用', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const OrderReviewRulesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('orders.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<OrderReviewRule | null>(null);
  const [saving, setSaving] = useState(false);
  const [conditionsError, setConditionsError] = useState<string | null>(null);
  const [templateKey, setTemplateKey] = useState<string | null>(null);
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [templateName, setTemplateName] = useState('');
  const [templateSaving, setTemplateSaving] = useState(false);

  const templateDefaults: TemplateOption[] = [
    {
      label: '高金额拦截（>=10000）',
      value: 'amount_high',
      payload: [{ field_path: 'total_cent', op: '>=', value: 10000 }]
    },
    {
      label: '地址异常拦截',
      value: 'address_invalid',
      payload: [{ field_path: 'address_invalid', op: '=', value: true }]
    },
    {
      label: '高SKU件数',
      value: 'sku_count',
      payload: [{ field_path: 'sku_count', op: '>=', value: 10 }]
    }
  ];

  const templateSettingKey = 'order.review.templates';
  const templateQuery = useQuery({
    queryKey: ['order-review-templates', tenantId],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: templateSettingKey
      }),
    enabled: Boolean(tenantId)
  });

  const templateOptions = useMemo(() => {
    const savedTemplates = templateQuery.data?.[0]?.value_json?.templates ?? [];
    const normalized = Array.isArray(savedTemplates)
      ? (savedTemplates as unknown[])
      : [];
    const merged = [...templateDefaults];
    normalized.forEach((item) => {
      if (!isTemplateOption(item)) {
        return;
      }
      if (!merged.some((entry) => entry.value === item.value)) {
        merged.push(item);
      }
    });
    return merged;
  }, [templateDefaults, templateQuery.data]);

  const query = useMemo(() => {
    const enabled =
      filters.enabled === undefined
        ? undefined
        : Boolean(filters.enabled);
    return {
      enabled,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    };
  }, [filters.enabled, pagination.current, pagination.pageSize]);

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['order-review-rules', tenantId, query],
    queryFn: () => listOrderReviewRules(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const summary = useMemo(() => {
    const items = data?.items ?? [];
    const enabledCount = items.filter((item) => item.enabled).length;
    const disabledCount = items.length - enabledCount;
    const latestUpdated = items.reduce<string | null>((latest, item) => {
      const timestamp = item.updated_at ?? item.created_at;
      if (!timestamp) {
        return latest;
      }
      if (!latest || timestamp > latest) {
        return timestamp;
      }
      return latest;
    }, null);
    return {
      total: data?.total ?? 0,
      enabledCount,
      disabledCount,
      latestUpdated
    };
  }, [data?.items, data?.total]);

  const latestUpdatedLabel = summary.latestUpdated ? formatDateTime(summary.latestUpdated) : '-';

  const deleteMutation = useMutation({
    mutationFn: (ruleId: string) => deleteOrderReviewRule(tenantId, ruleId),
    onSuccess: () => {
      message.success('审单规则已删除');
      queryClient.invalidateQueries({ queryKey: ['order-review-rules', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除审单规则失败');
    }
  });

  const columns = useMemo<ColumnsType<OrderReviewRule>>(
    () => [
      { title: '规则名称', dataIndex: 'name' },
      {
        title: '动作',
        dataIndex: 'action',
        render: (value) => {
          const meta = actionOptions.find((option) => option.value === value);
          const color = value === 'REJECT' ? 'red' : value === 'FLAG' ? 'orange' : 'green';
          return <Tag color={color}>{meta?.label ?? value}</Tag>;
        }
      },
      { title: '优先级', dataIndex: 'priority' },
      {
        title: '状态',
        dataIndex: 'enabled',
        render: (value) => (value ? <Tag color="green">启用</Tag> : <Tag color="red">停用</Tag>)
      },
      {
        title: '命中条件',
        dataIndex: 'conditions_json',
        render: (value) => {
          if (!value) {
            return '-';
          }
          if (Array.isArray(value)) {
            return value.length > 0 ? <Tag>已配置</Tag> : '-';
          }
          if (typeof value === 'object') {
            return Object.keys(value).length > 0 ? <Tag>已配置</Tag> : '-';
          }
          return <Tag>已配置</Tag>;
        }
      },
      { title: '原因', dataIndex: 'reason', render: (value) => value || '-' },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setEditingRule(record);
                drawerForm.setFieldsValue({
                  name: record.name,
                  action: record.action,
                  enabled: record.enabled,
                  priority: record.priority,
                  reason: record.reason,
                  conditions_json: record.conditions_json
                    ? JSON.stringify(record.conditions_json, null, 2)
                    : '[]'
                });
                setConditionsError(null);
                setTemplateKey(null);
                setDrawerOpen(true);
              }}
            >
              编辑
            </Button>
            <Button
              size="small"
              danger
              disabled={!canWrite}
              onClick={() => deleteMutation.mutate(record.rule_id)}
            >
              删除
            </Button>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation, drawerForm]
  );

  const handleSearch = (values: FilterValues) => {
    if (values.status === 'ENABLED') {
      setFilters({ enabled: true });
    } else if (values.status === 'DISABLED') {
      setFilters({ enabled: false });
    } else {
      setFilters({});
    }
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingRule(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      enabled: true,
      priority: 0,
      action: 'PASS',
      conditions_json: '[]'
    });
    setConditionsError(null);
    setTemplateKey(null);
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      let conditions: Record<string, unknown> | unknown[] | undefined = undefined;
      if (values.conditions_json?.trim()) {
        try {
          conditions = JSON.parse(values.conditions_json);
          setConditionsError(null);
        } catch {
          setConditionsError('条件格式必须为合法 JSON');
          return;
        }
      }
      const payload: OrderReviewRuleCreatePayload = {
        name: values.name.trim(),
        action: values.action,
        enabled: values.enabled,
        priority: values.priority ?? 0,
        reason: values.reason?.trim() || '',
        conditions_json: conditions ?? []
      };
      setSaving(true);
      if (editingRule) {
        await updateOrderReviewRule(tenantId, editingRule.rule_id, payload);
        message.success('审单规则已更新');
      } else {
        await createOrderReviewRule(tenantId, payload);
        message.success('审单规则已创建');
      }
      setDrawerOpen(false);
      setEditingRule(null);
      await queryClient.invalidateQueries({ queryKey: ['order-review-rules', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleSaveTemplate = async () => {
    const name = templateName.trim();
    if (!name) {
      message.warning('请输入模板名称');
      return;
    }
    const conditionsRaw = drawerForm.getFieldValue('conditions_json') as string | undefined;
    if (!conditionsRaw) {
      message.warning('请先填写条件');
      return;
    }
    let payload: Record<string, unknown> | unknown[] = [];
    try {
      payload = JSON.parse(conditionsRaw);
    } catch {
      message.warning('条件格式必须为合法 JSON');
      return;
    }
    const currentSetting = templateQuery.data?.[0];
    const existing = Array.isArray(currentSetting?.value_json?.templates)
      ? (currentSetting?.value_json?.templates as unknown[]).filter(isTemplateOption)
      : [];
    const template = {
      label: name,
      value: `custom_${Date.now()}`,
      payload
    };
    const value = {
      templates: [...existing, template]
    };
    setTemplateSaving(true);
    try {
      await upsertSetting(tenantId, {
        scope_type: 'TENANT',
        scope_id: null,
        key: templateSettingKey,
        value,
        expected_version: currentSetting?.version
      });
      message.success('模板已保存');
      setTemplateModalOpen(false);
      setTemplateName('');
      templateQuery.refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '模板保存失败');
    } finally {
      setTemplateSaving(false);
    }
  };

  const toolbarLeft = (
    <Space direction="vertical" size={2}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        审单规则
      </Typography.Title>
    </Space>
  );

  const toolbarRight = (
    <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
    新建规则
    </Button>
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
        <Descriptions size="small" column={4} bordered>
          <Descriptions.Item label="规则总数">{summary.total}</Descriptions.Item>
          <Descriptions.Item label="启用">{summary.enabledCount}</Descriptions.Item>
          <Descriptions.Item label="停用">{summary.disabledCount}</Descriptions.Item>
          <Descriptions.Item label="最近更新">{latestUpdatedLabel}</Descriptions.Item>
        </Descriptions>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select
                  allowClear
                  options={enabledOptions}
                  placeholder="选择状态"
                  style={{ minWidth: 160 }}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
              <Form.Item>
                <Space>
                  <Button type="primary" htmlType="submit">
                    查询
                  </Button>
                  <Button onClick={handleReset}>重置</Button>
                </Space>
              </Form.Item>
            </Col>
          </Row>
        </Form>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<OrderReviewRule>
        toolbarLeft={toolbarLeft}
        toolbarRight={toolbarRight}
        header={header}
        rowKey="rule_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="审单规则加载失败"
        emptyText="暂无审单规则"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        dataTestId="order-review-rules-main-grid"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
        columnSettingsKey="order-review-rules.table.columns"
      />
      <Drawer
        title={editingRule ? '编辑审单规则' : '新建审单规则'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={560}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical">
          <Form.Item
            name="name"
            label="规则名称"
            rules={[{ required: true, message: '请输入规则名称' }]}
          >
            <Input placeholder="例如：高金额订单拦截" />
          </Form.Item>
          <Form.Item name="action" label="动作" rules={[{ required: true }]}>
            <Select options={actionOptions} />
          </Form.Item>
          <Form.Item name="enabled" label="启用状态" valuePropName="checked">
            <Switch checkedChildren="启用" unCheckedChildren="停用" />
          </Form.Item>
          <Form.Item name="priority" label="优先级">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="reason" label="命中原因">
            <Input placeholder="例如：高风险订单" />
          </Form.Item>
          <Form.Item label="条件模板">
            <Space style={{ width: '100%' }} align="start">
              <Select
                allowClear
                value={templateKey ?? undefined}
                options={templateOptions.map((option) => ({
                  label: option.label,
                  value: option.value
                }))}
                placeholder="选择模板快速填充"
                onChange={(value) => {
                  if (!value) {
                    setTemplateKey(null);
                    return;
                  }
                  const template = templateOptions.find((item) => item.value === value);
                  if (template) {
                    drawerForm.setFieldsValue({
                      conditions_json: JSON.stringify(template.payload, null, 2)
                    });
                    setTemplateKey(value);
                    setConditionsError(null);
                  }
                }}
                style={{ flex: 1 }}
              />
              <Button onClick={() => setTemplateModalOpen(true)}>保存模板</Button>
            </Space>
          </Form.Item>
          <Form.Item
            name="conditions_json"
            label="命中条件（JSON）"
            extra="支持 OrderReviewCondition 结构，空数组表示不命中"
            validateStatus={conditionsError ? 'error' : ''}
            help={conditionsError ?? undefined}
          >
            <Input.TextArea rows={6} placeholder='例如：[{"field_path":"total_cent","op":">","value":10000}]' />
          </Form.Item>
        </Form>
      </Drawer>
      <Modal
        title="保存条件模板"
        open={templateModalOpen}
        okText="保存"
        cancelText="取消"
        confirmLoading={templateSaving}
        onOk={handleSaveTemplate}
        onCancel={() => setTemplateModalOpen(false)}
      >
        <Input
          value={templateName}
          onChange={(event) => setTemplateName(event.target.value)}
          placeholder="请输入模板名称"
        />
      </Modal>
    </>
  );
};
