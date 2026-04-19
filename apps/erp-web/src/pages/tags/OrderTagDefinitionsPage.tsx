import {
  Alert,
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Switch,
  Popconfirm,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  OrderTagDefinition,
  OrderTagDefinitionCreatePayload
} from '@/api/orderTagDefinitions';
import {
  createOrderTagDefinition,
  deleteOrderTagDefinition,
  listOrderTagDefinitions,
  updateOrderTagDefinition
} from '@/api/orderTagDefinitions';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';
import { listSettings, upsertSetting } from '@/api/system';

type FilterValues = {
  keyword?: string;
  status?: string;
};

type FormValues = {
  name: string;
  color?: string;
  priority?: number;
  status?: string;
  auto_apply?: boolean;
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

const statusOptions = [
  { label: '鍚敤', value: 'ACTIVE' },
  { label: '鍋滅敤', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

type OrderTagDefinitionsPageProps = {
  showTitle?: boolean;
};

export const OrderTagDefinitionsPage = ({ showTitle = true }: OrderTagDefinitionsPageProps) => {
  const tenantId = useTenantId();
  const canWrite = useCan('order_tags.write');
  const featureFlag = useFeatureFlag('directory.tag_system');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingTag, setEditingTag] = useState<OrderTagDefinition | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [conditionsError, setConditionsError] = useState<string | null>(null);
  const [templateKey, setTemplateKey] = useState<string | null>(null);
  const [templateModalOpen, setTemplateModalOpen] = useState(false);
  const [templateName, setTemplateName] = useState('');
  const [templateSaving, setTemplateSaving] = useState(false);

  const templateDefaults: TemplateOption[] = [
    {
      label: 'Urgent Order (total>=5000)',
      value: 'urgent_high',
      payload: [{ field_path: 'total_cent', op: '>=', value: 5000 }]
    },
    {
      label: '高SKU浠舵暟标记',
      value: 'sku_count',
      payload: [{ field_path: 'sku_count', op: '>=', value: 5 }]
    },
    {
      label: '鍦板潃异常标记',
      value: 'address_invalid',
      payload: [{ field_path: 'address_invalid', op: '=', value: true }]
    }
  ];

  const templateSettingKey = 'order.tag.templates';
  const templateQuery = useQuery({
    queryKey: ['order-tag-templates', tenantId],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: templateSettingKey
      }),
    enabled: Boolean(tenantId) && featureFlag.enabled
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

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['order-tag-definitions', tenantId, query],
    queryFn: () => listOrderTagDefinitions(tenantId, query),
    enabled: featureFlag.enabled,
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (tagId: string) => deleteOrderTagDefinition(tenantId, tagId),
    onMutate: (tagId) => setDeletingId(tagId),
    onSuccess: () => {
      message.success('Order tag deleted');
      queryClient.invalidateQueries({ queryKey: ['order-tag-definitions', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除订单标记失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const columns = useMemo<ColumnsType<OrderTagDefinition>>(
    () => [
      {
        title: '标记鍚嶇О',
        dataIndex: 'name'
      },
      {
        title: '棰滆壊',
        dataIndex: 'color',
        render: (value) =>
          value ? (
            <Tag color={value} style={{ borderColor: value }}>
              {value}
            </Tag>
          ) : (
            '-'
          )
      },
      {
        title: 'Priority',
        dataIndex: 'priority'
      },
      {
        title: 'Status',
        dataIndex: 'status',
        render: (value) => (value === 'DISABLED' ? <Tag color="red">鍋滅敤</Tag> : '鍚敤')
      },
      {
        title: '榛樿标记',
        dataIndex: 'auto_apply',
        render: (value) =>
          value ? <Tag color="green">鑷姩鎵撴爣</Tag> : <Tag>鎵嬪姩</Tag>
      },
      {
        title: '鍛戒腑鏉′欢',
        dataIndex: 'conditions_json',
        render: (value) => {
          if (!value) {
            return '-';
          }
          if (Array.isArray(value)) {
            return value.length > 0 ? <Tag>Configured</Tag> : '-';
          }
          if (typeof value === 'object') {
            return Object.keys(value).length > 0 ? <Tag>Configured</Tag> : '-';
          }
          return <Tag>Configured</Tag>;
        }
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '鎿嶄綔',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={!canWrite}
              onClick={() => {
                setEditingTag(record);
                drawerForm.setFieldsValue({
                  name: record.name,
                  color: record.color ?? undefined,
                  priority: record.priority,
                  status: record.status,
                  auto_apply: record.auto_apply,
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
            <Popconfirm
              title="确认删除璇ユ爣璁帮紵"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.tag_id)}
              disabled={!canWrite}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deletingId === record.tag_id}
              >
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteMutation, deletingId, drawerForm]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingTag(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      status: 'ACTIVE',
      priority: 0,
      auto_apply: false,
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
          setConditionsError('鏉′欢鏍煎紡蹇呴』涓哄悎娉?JSON');
          return;
        }
      }
      const payload: OrderTagDefinitionCreatePayload = {
        name: values.name.trim(),
        color: values.color?.trim() || undefined,
        priority: values.priority,
        status: values.status?.trim() || undefined,
        auto_apply: values.auto_apply ?? false,
        conditions_json: conditions ?? []
      };
      setSaving(true);
      if (editingTag) {
        await updateOrderTagDefinition(tenantId, editingTag.tag_id, payload);
        message.success('Order tag updated');
      } else {
        await createOrderTagDefinition(tenantId, payload);
        message.success('Order tag created');
      }
      setDrawerOpen(false);
      setEditingTag(null);
      await queryClient.invalidateQueries({ queryKey: ['order-tag-definitions', tenantId] });
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
      message.warning('Please enter template name');
      return;
    }
    const conditionsRaw = drawerForm.getFieldValue('conditions_json') as string | undefined;
    if (!conditionsRaw) {
      message.warning('璇峰厛濉啓鏉′欢');
      return;
    }
    let payload: Record<string, unknown> | unknown[] = [];
    try {
      payload = JSON.parse(conditionsRaw);
    } catch {
      message.warning('鏉′欢鏍煎紡蹇呴』涓哄悎娉?JSON');
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
      message.success('Template saved');
      setTemplateModalOpen(false);
      setTemplateName('');
      templateQuery.refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '模板保存失败');
    } finally {
      setTemplateSaving(false);
    }
  };

  if (!featureFlag.enabled) {
    return (
      <Alert
        message="Tag system is disabled"
        description={featureFlag.reason || 'Please enable tag system in settings'}
        type="warning"
        showIcon
      />
    );
  }

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        {showTitle ? (
          <Typography.Title level={3} style={{ margin: 0 }}>
            订单标记
          </Typography.Title>
        ) : (
          <span />
        )}
        <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
          鏂板缓标记
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="Keyword">
                <Input allowClear placeholder="标记鍚嶇О" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8}>
              <Form.Item name="status" label="Status">
                <Select allowClear options={statusOptions} placeholder="All status" />
              </Form.Item>
            </Col>
            <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  鏌ヨ
                </Button>
                <Button onClick={handleReset}>閲嶇疆</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      <UiModule03MainTableSection<OrderTagDefinition>
        rowKey="tag_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="订单标记加载失败"
        emptyText="暂无订单标记"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="order-tags.table.columns"
      />
      <Drawer
        title={editingTag ? '编辑订单标记' : '鏂板缓订单标记'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
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
            label="标记鍚嶇О"
            rules={[{ required: true, message: 'Please enter tag name' }]}
          >
            <Input placeholder="e.g. Urgent order" />
          </Form.Item>
          <Form.Item name="color" label="棰滆壊">
            <Input placeholder="渚嬪锛?ff4d4f" />
          </Form.Item>
          <Form.Item name="priority" label="Priority">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="auto_apply" label="榛樿标记" valuePropName="checked">
            <Switch checkedChildren="鑷姩鎵撴爣" unCheckedChildren="鎵嬪姩" />
          </Form.Item>
          <Form.Item label="鏉′欢模板">
            <Space style={{ width: '100%' }} align="start">
              <Select
                allowClear
                value={templateKey ?? undefined}
                options={templateOptions.map((option) => ({
                  label: option.label,
                  value: option.value
                }))}
                placeholder="Select template to fill conditions"
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
            label="Hit Conditions (JSON)"
            extra="鏀寔 OrderReviewCondition 缁撴瀯锛岀┖鏁扮粍琛ㄧず鍏ㄩ儴鍛戒腑"
            validateStatus={conditionsError ? 'error' : ''}
            help={conditionsError ?? undefined}
          >
            <Input.TextArea rows={6} placeholder='渚嬪锛歔{"field_path":"total_cent","op":">","value":10000}]' />
          </Form.Item>
          <Form.Item name="status" label="Status" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
        </Form>
      </Drawer>
      <Modal
        title="保存鏉′欢模板"
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
          placeholder="Please input template name"
        />
      </Modal>
    </Space>
  );
};

