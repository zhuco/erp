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
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { MemberLevel, MemberLevelCreatePayload } from '@/api/memberLevels';
import {
  createMemberLevel,
  deleteMemberLevel,
  listMemberLevels,
  updateMemberLevel
} from '@/api/memberLevels';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
};

type FormValues = {
  name: string;
  level_rank?: number;
  discount_rate?: number;
  rules?: string;
  status?: string;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'INACTIVE' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const MemberLevelsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('customers.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingLevel, setEditingLevel] = useState<MemberLevel | null>(null);
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
    queryKey: ['member-levels', tenantId, query],
    queryFn: () => listMemberLevels(tenantId, query),
    keepPreviousData: true
  });

  const deleteMutation = useMutation({
    mutationFn: (levelId: string) => deleteMemberLevel(tenantId, levelId),
    onMutate: (levelId) => {
      setDeletingId(levelId);
    },
    onSuccess: () => {
      message.success('会员等级已删除');
      queryClient.invalidateQueries({ queryKey: ['member-levels', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除会员等级失败');
    },
    onSettled: () => {
      setDeletingId(null);
    }
  });

  const columns = useMemo<ColumnsType<MemberLevel>>(
    () => [
      {
        title: '等级名称',
        dataIndex: 'name'
      },
      {
        title: '等级排序',
        dataIndex: 'level_rank'
      },
      {
        title: '折扣',
        dataIndex: 'discount_rate',
        render: (value?: number | null) => (value == null ? '-' : `${(value * 100).toFixed(0)}%`)
      },
      {
        title: '规则',
        dataIndex: 'rules'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value?: string | null) => (
          <Tag color={value === 'ACTIVE' ? 'green' : 'default'}>{value ?? '-'}</Tag>
        )
      },
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
                setEditingLevel(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  name: record.name,
                  level_rank: record.level_rank,
                  discount_rate: record.discount_rate ?? undefined,
                  rules: record.rules ?? undefined,
                  status: record.status ?? undefined
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除会员等级？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.level_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.level_id}
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
    setEditingLevel(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: MemberLevelCreatePayload = {
        name: values.name.trim(),
        level_rank: values.level_rank ?? 0,
        discount_rate: values.discount_rate,
        rules: values.rules?.trim() || undefined,
        status: values.status
      };
      setSaving(true);
      if (editingLevel) {
        await updateMemberLevel(tenantId, editingLevel.level_id, payload);
        message.success('会员等级已更新');
      } else {
        await createMemberLevel(tenantId, payload);
        message.success('会员等级已创建');
      }
      setDrawerOpen(false);
      setEditingLevel(null);
      await queryClient.invalidateQueries({ queryKey: ['member-levels', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          会员等级
        </Typography.Title>
        <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
          新建等级
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="等级名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select allowClear options={statusOptions} placeholder="全部" />
              </Form.Item>
            </Col>
            <Col xs={24} md={10} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  查询
                </Button>
                <Button onClick={handleReset}>重置</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<MemberLevel>
        header={header}
        rowKey="level_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="加载会员等级失败"
        emptyText="暂无会员等级"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="member.levels.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editingLevel ? '编辑会员等级' : '新建会员等级'}
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
            label="等级名称"
            rules={[{ required: true, message: '请输入等级名称' }]}
          >
            <Input placeholder="等级名称" />
          </Form.Item>
          <Form.Item name="level_rank" label="等级排序">
            <InputNumber min={0} style={{ width: '100%' }} placeholder="等级排序" />
          </Form.Item>
          <Form.Item name="discount_rate" label="折扣">
            <InputNumber
              min={0}
              max={1}
              step={0.01}
              style={{ width: '100%' }}
              placeholder="例如 0.95"
            />
          </Form.Item>
          <Form.Item name="rules" label="规则">
            <Input.TextArea rows={3} placeholder="等级规则" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select allowClear options={statusOptions} placeholder="选择状态" />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
