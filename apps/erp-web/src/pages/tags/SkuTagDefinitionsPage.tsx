import {
  Alert,
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
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { SkuTagDefinition, SkuTagDefinitionCreatePayload } from '@/api/skuTagDefinitions';
import {
  createSkuTagDefinition,
  deleteSkuTagDefinition,
  listSkuTagDefinitions,
  updateSkuTagDefinition
} from '@/api/skuTagDefinitions';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useFeatureFlag } from '@/features/featureFlags';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
};

type FormValues = {
  name: string;
  color?: string;
  priority?: number;
  status?: string;
};

const statusOptions = [
  { label: '鍚敤', value: 'ACTIVE' },
  { label: '鍋滅敤', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

type SkuTagDefinitionsPageProps = {
  showTitle?: boolean;
};

export const SkuTagDefinitionsPage = ({ showTitle = true }: SkuTagDefinitionsPageProps) => {
  const tenantId = useTenantId();
  const canWrite = useCan('sku_tags.write');
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
  const [editingTag, setEditingTag] = useState<SkuTagDefinition | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

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
    queryKey: ['sku-tag-definitions', tenantId, query],
    queryFn: () => listSkuTagDefinitions(tenantId, query),
    enabled: featureFlag.enabled,
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (tagId: string) => deleteSkuTagDefinition(tenantId, tagId),
    onMutate: (tagId) => setDeletingId(tagId),
    onSuccess: () => {
      message.success('SKU tag deleted');
      queryClient.invalidateQueries({ queryKey: ['sku-tag-definitions', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除货品标记失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const columns = useMemo<ColumnsType<SkuTagDefinition>>(
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
                  status: record.status
                });
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
    drawerForm.setFieldsValue({ status: 'ACTIVE', priority: 0 });
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: SkuTagDefinitionCreatePayload = {
        name: values.name.trim(),
        color: values.color?.trim() || undefined,
        priority: values.priority,
        status: values.status?.trim() || undefined
      };
      setSaving(true);
      if (editingTag) {
        await updateSkuTagDefinition(tenantId, editingTag.tag_id, payload);
        message.success('SKU tag updated');
      } else {
        await createSkuTagDefinition(tenantId, payload);
        message.success('SKU tag created');
      }
      setDrawerOpen(false);
      setEditingTag(null);
      await queryClient.invalidateQueries({ queryKey: ['sku-tag-definitions', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  if (!featureFlag.enabled) {
    return (
      <Alert
        message='Tag system is disabled'
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
            货品标记
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
      <UiModule03MainTableSection<SkuTagDefinition>
        rowKey="tag_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="货品标记加载失败"
        emptyText="暂无货品标记"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="sku-tags.table.columns"
      />
      <Drawer
        title={editingTag ? '编辑货品标记' : '鏂板缓货品标记'}
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
            <Input placeholder="渚嬪锛氭槗纰庡搧" />
          </Form.Item>
          <Form.Item name="color" label="棰滆壊">
            <Input placeholder="渚嬪锛?13c2c2" />
          </Form.Item>
          <Form.Item name="priority" label="Priority">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="Status" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
        </Form>
      </Drawer>
    </Space>
  );
};

