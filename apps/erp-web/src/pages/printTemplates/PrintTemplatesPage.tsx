import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Row,
  Select,
  Space,
  Switch,
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
  PrintTemplate,
  PrintTemplateCreatePayload,
  PrintTemplateFormat,
  PrintTemplateListQuery,
  PrintTemplateUpdatePayload
} from '@/api/printTemplates';
import {
  createPrintTemplate,
  deletePrintTemplate,
  listPrintTemplates,
  updatePrintTemplate
} from '@/api/printTemplates';
import { PlatformTag } from '@/components/PlatformTag';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import type { PlatformCode } from '@/features/platform';
import { platformOptions } from '@/features/platform';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  platform?: PlatformCode;
  shop_id?: string;
  template_type?: PrintTemplateListQuery['template_type'];
  enabled?: boolean;
};

type FormValues = {
  shop_id: string;
  platform: PlatformCode;
  template_type: PrintTemplateListQuery['template_type'];
  version: number;
  content_format: PrintTemplateFormat | string;
  content?: string;
  enabled: boolean;
};

const templateTypeOptions = [
  { label: '闈㈠崟', value: 'SHIPPING_LABEL' },
  { label: 'Pick List', value: 'PICK_LIST' },
  { label: 'Pack List', value: 'PACK_LIST' },
  { label: '鍙戠エ', value: 'INVOICE' }
];

const templateFormatOptions = [
  { label: 'HTML', value: 'HTML' },
  { label: 'ZPL', value: 'ZPL' },
  { label: 'TEXT', value: 'TEXT' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const PrintTemplatesPage = () => {
  const tenantId = useTenantId();
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<PrintTemplate | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo<PrintTemplateListQuery>(
    () => ({
      platform: filters.platform,
      shop_id: filters.shop_id?.trim() || undefined,
      template_type: filters.template_type,
      enabled: filters.enabled,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['print-templates', tenantId, query],
    queryFn: () => listPrintTemplates(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (templateId: string) => deletePrintTemplate(tenantId, templateId),
    onMutate: (templateId) => {
      setDeletingId(templateId);
    },
    onSuccess: () => {
      message.success('Template deleted');
      queryClient.invalidateQueries({ queryKey: ['print-templates', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除打印模板失败');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const columns = useMemo<ColumnsType<PrintTemplate>>(
    () => [
      {
        title: '平台',
        dataIndex: 'platform',
        render: (_, record) => <PlatformTag platform={record.platform} />
      },
      {
        title: '店铺',
        dataIndex: 'shop_id'
      },
      {
        title: '绫诲瀷',
        dataIndex: 'template_type',
        render: (_, record) =>
          record.template_type === 'SHIPPING_LABEL' ? '闈㈠崟' : record.template_type
      },
      {
        title: '鐗堟湰',
        dataIndex: 'version'
      },
      {
        title: '鏍煎紡',
        dataIndex: 'content_format',
        render: (_, record) => record.content_format ?? '-'
      },
      {
        title: 'Status',
        dataIndex: 'enabled',
        render: (_, record) =>
          record.enabled ? <Tag color="green">鍚敤</Tag> : <Tag>鍋滅敤</Tag>
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
              onClick={() => {
                setEditingTemplate(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  shop_id: record.shop_id,
                  platform: record.platform as PlatformCode,
                  template_type: record.template_type as PrintTemplateListQuery['template_type'],
                  version: record.version,
                  content_format: record.content_format ?? 'HTML',
                  content:
                    typeof record.content === 'string'
                      ? record.content
                      : record.content
                        ? JSON.stringify(record.content, null, 2)
                        : undefined,
                  enabled: record.enabled
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除璇ユā鏉匡紵"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.template_id)}
            >
              <Button
                size="small"
                danger
                loading={deleteMutation.isPending && deletingId === record.template_id}
              >
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [deleteMutation.isPending, deletingId, drawerForm]
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
    setEditingTemplate(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ enabled: true, version: 1, content_format: 'HTML' });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const trimmedContent = values.content?.trim();
      let content: unknown = undefined;
      if (trimmedContent) {
        try {
          content = JSON.parse(trimmedContent);
        } catch {
          content = trimmedContent;
        }
      }
      const payload: PrintTemplateCreatePayload = {
        shop_id: values.shop_id.trim(),
        platform: values.platform,
        template_type: values.template_type,
        version: values.version,
        content_format: values.content_format as PrintTemplateFormat,
        content,
        enabled: values.enabled
      };
      setSaving(true);
      if (editingTemplate) {
        const updatePayload: PrintTemplateUpdatePayload = {
          content: payload.content,
          enabled: payload.enabled,
          content_format: payload.content_format
        };
        await updatePrintTemplate(tenantId, editingTemplate.template_id, updatePayload);
        message.success('Template updated');
      } else {
        await createPrintTemplate(tenantId, payload);
        message.success('Template created');
      }
      setDrawerOpen(false);
      setEditingTemplate(null);
      await queryClient.invalidateQueries({ queryKey: ['print-templates', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} data-testid="print-templates-title" style={{ margin: 0 }}>
          打印模板
        </Typography.Title>
        <Button type="primary" onClick={openCreateDrawer}>
          鏂板缓模板
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={6}>
              <Form.Item name="platform" label="平台">
                <Select allowClear options={platformOptions} placeholder="选择平台" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="shop_id" label="店铺">
                <Input allowClear placeholder="输入店铺 ID" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="template_type" label="绫诲瀷">
                <Select allowClear options={templateTypeOptions} placeholder="閫夋嫨绫诲瀷" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name='enabled' label='Enabled'>
                <Select
                  allowClear
                  options={[
                    { label: '鍚敤', value: true },
                    { label: '鍋滅敤', value: false }
                  ]}
                  placeholder='Select status'
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={12} style={{ display: 'flex', alignItems: 'end' }}>
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
      <UiModule03MainTableSection<PrintTemplate>
        rowKey="template_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="打印模板加载失败"
        emptyText="暂无打印模板"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="print-templates.table.columns"
      />
      <Drawer
        title={editingTemplate ? '编辑模板' : '鏂板缓模板'}
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
            name="shop_id"
            label="店铺ID"
            rules={[{ required: true, message: '璇疯緭鍏ュ簵閾篒D' }]}
          >
            <Input placeholder="输入店铺ID" disabled={Boolean(editingTemplate)} />
          </Form.Item>
          <Form.Item
            name="platform"
            label="平台"
            rules={[{ required: true, message: '璇烽€夋嫨平台' }]}
          >
            <Select options={platformOptions} disabled={Boolean(editingTemplate)} />
          </Form.Item>
          <Form.Item
            name="template_type"
            label="绫诲瀷"
            rules={[{ required: true, message: '璇烽€夋嫨模板绫诲瀷' }]}
          >
            <Select options={templateTypeOptions} disabled={Boolean(editingTemplate)} />
          </Form.Item>
          <Form.Item
            name="version"
            label="鐗堟湰"
            rules={[{ required: true, message: 'Please enter version' }]}
          >
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="content_format"
            label="鏍煎紡"
            rules={[{ required: true, message: '璇烽€夋嫨鏍煎紡' }]}
          >
            <Select options={templateFormatOptions} />
          </Form.Item>
          <Form.Item name="content" label="模板鍐呭">
            <Input.TextArea rows={6} placeholder="鏀寔HTML鎴朖SON" />
          </Form.Item>
          <Form.Item name="enabled" label="鍚敤" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Drawer>
    </Space>
  );
};

