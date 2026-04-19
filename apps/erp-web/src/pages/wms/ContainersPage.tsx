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
  Space,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Container, ContainerCreatePayload } from '@/api/containers';
import { createContainer, deleteContainer, listContainers, updateContainer } from '@/api/containers';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
};

type FormValues = {
  code: string;
  container_type: string;
  capacity?: number;
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const ContainersPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('inventory.write');
  const queryClient = useQueryClient();
  const [filterForm] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingContainer, setEditingContainer] = useState<Container | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo(
    () => ({
      keyword: filters.keyword?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['containers', tenantId, query],
    queryFn: () => listContainers(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (containerId: string) => deleteContainer(tenantId, containerId),
    onMutate: (containerId) => setDeletingId(containerId),
    onSuccess: () => {
      message.success('容器已删除');
      queryClient.invalidateQueries({ queryKey: ['containers', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除容器失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const columns = useMemo<ColumnsType<Container>>(
    () => [
      { title: '容器编码', dataIndex: 'code' },
      { title: '类型', dataIndex: 'container_type' },
      { title: '容量', dataIndex: 'capacity', render: (value) => value ?? '-' },
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
                setEditingContainer(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  code: record.code,
                  container_type: record.container_type,
                  capacity: record.capacity ?? undefined
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该容器？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.container_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.container_id}
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
    filterForm.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingContainer(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      if (editingContainer) {
        await updateContainer(tenantId, editingContainer.container_id, {
          container_type: values.container_type.trim(),
          capacity: values.capacity
        });
        message.success('容器已更新');
      } else {
        const payload: ContainerCreatePayload = {
          code: values.code.trim(),
          container_type: values.container_type.trim(),
          capacity: values.capacity
        };
        await createContainer(tenantId, payload);
        message.success('容器已创建');
      }
      setDrawerOpen(false);
      setEditingContainer(null);
      await queryClient.invalidateQueries({ queryKey: ['containers', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const toolbarLeft = (
    <Space align="center" size={12} wrap>
      <Typography.Title level={3} style={{ margin: 0 }}>
        容器目录
      </Typography.Title>
      <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
        新建容器
      </Button>
    </Space>
  );

  const header = (
    <Card>
      <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={8}>
            <Form.Item name="keyword" label="关键词">
              <Input allowClear placeholder="编码/类型" />
            </Form.Item>
          </Col>
          <Col xs={24} md={8} style={{ display: 'flex', alignItems: 'end' }}>
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
  );

  return (
    <>
      <UiModule03MainTableSection<Container>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="container_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="容器列表加载失败"
        emptyText="暂无容器"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="containers.table.columns"
      />
      <Drawer
        title={editingContainer ? '编辑容器' : '新建容器'}
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
            name="code"
            label="容器编码"
            rules={[{ required: true, message: '请输入容器编码' }]}
          >
            <Input disabled={Boolean(editingContainer)} />
          </Form.Item>
          <Form.Item
            name="container_type"
            label="容器类型"
            rules={[{ required: true, message: '请输入容器类型' }]}
          >
            <Input placeholder="箱/托盘/周转筐" />
          </Form.Item>
          <Form.Item name="capacity" label="容量">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
