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

import type {
  WarehouseArea,
  WarehouseAreaCreatePayload,
  WarehouseAreaListQuery,
  WarehouseAreaUpdatePayload
} from '@/api/warehouseAreas';
import {
  createWarehouseArea,
  deleteWarehouseArea,
  listWarehouseAreas,
  updateWarehouseArea
} from '@/api/warehouseAreas';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  warehouse_id?: string;
  area_type?: string;
  status?: string;
  keyword?: string;
};

type FormValues = {
  warehouse_id: string;
  code: string;
  name: string;
  area_type?: string;
  status?: string;
  priority?: number;
};

const areaTypeOptions = [
  { label: '存储', value: 'STORAGE' },
  { label: '拣货', value: 'PICKING' },
  { label: '收货', value: 'RECEIVING' },
  { label: '发货', value: 'SHIPPING' }
];

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '禁用', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const renderAreaType = (value?: string | null) => {
  const match = areaTypeOptions.find((item) => item.value === value);
  return match?.label ?? value ?? '-';
};

export const WarehouseAreasPage = () => {
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
  const [editingArea, setEditingArea] = useState<WarehouseArea | null>(null);
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const query = useMemo<WarehouseAreaListQuery>(
    () => ({
      warehouse_id: filters.warehouse_id?.trim() || undefined,
      area_type: filters.area_type?.trim() || undefined,
      status: filters.status?.trim() || undefined,
      keyword: filters.keyword?.trim() || undefined,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['warehouse-areas', tenantId, query],
    queryFn: () => listWarehouseAreas(tenantId, query),
    placeholderData: (previousData) => previousData
  });

  const deleteMutation = useMutation({
    mutationFn: (areaId: string) => deleteWarehouseArea(tenantId, areaId),
    onMutate: (areaId) => setDeletingId(areaId),
    onSuccess: () => {
      message.success('库区已删除');
      queryClient.invalidateQueries({ queryKey: ['warehouse-areas', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除库区失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const columns = useMemo<ColumnsType<WarehouseArea>>(
    () => [
      { title: '库区编码', dataIndex: 'code' },
      { title: '库区名称', dataIndex: 'name' },
      { title: '仓库ID', dataIndex: 'warehouse_id' },
      {
        title: '类型',
        dataIndex: 'area_type',
        render: (value) => renderAreaType(value)
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'DISABLED' ? <Tag color="red">禁用</Tag> : <Tag color="green">启用</Tag>
      },
      { title: '优先级', dataIndex: 'priority' },
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
                setEditingArea(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  warehouse_id: record.warehouse_id,
                  code: record.code,
                  name: record.name,
                  area_type: record.area_type,
                  status: record.status,
                  priority: record.priority
                });
              }}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该库区？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteMutation.mutate(record.area_id)}
            >
              <Button
                size="small"
                danger
                disabled={!canWrite}
                loading={deleteMutation.isPending && deletingId === record.area_id}
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
    setEditingArea(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      area_type: 'STORAGE',
      status: 'ACTIVE',
      priority: 0
    });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      setSaving(true);
      if (editingArea) {
        const payload: WarehouseAreaUpdatePayload = {
          name: values.name.trim(),
          area_type: values.area_type,
          status: values.status,
          priority: values.priority ?? 0
        };
        await updateWarehouseArea(tenantId, editingArea.area_id, payload);
        message.success('库区已更新');
      } else {
        const payload: WarehouseAreaCreatePayload = {
          warehouse_id: values.warehouse_id.trim(),
          code: values.code.trim(),
          name: values.name.trim(),
          area_type: values.area_type,
          status: values.status,
          priority: values.priority ?? 0
        };
        await createWarehouseArea(tenantId, payload);
        message.success('库区已创建');
      }
      setDrawerOpen(false);
      setEditingArea(null);
      await queryClient.invalidateQueries({ queryKey: ['warehouse-areas', tenantId] });
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
        库区目录
      </Typography.Title>
      <Button type="primary" disabled={!canWrite} onClick={openCreateDrawer}>
        新建库区
      </Button>
    </Space>
  );

  const header = (
    <Card>
      <Form form={filterForm} layout="vertical" onFinish={handleSearch}>
        <Row gutter={16}>
          <Col xs={24} md={6}>
            <Form.Item name="warehouse_id" label="仓库ID">
              <Input allowClear placeholder="仓库ID" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="area_type" label="类型">
              <Select allowClear options={areaTypeOptions} placeholder="全部类型" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="status" label="状态">
              <Select allowClear options={statusOptions} placeholder="全部状态" />
            </Form.Item>
          </Col>
          <Col xs={24} md={6}>
            <Form.Item name="keyword" label="关键词">
              <Input allowClear placeholder="编码/名称" />
            </Form.Item>
          </Col>
          <Col xs={24} md={24} style={{ display: 'flex', alignItems: 'end' }}>
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
      <UiModule03MainTableSection<WarehouseArea>
        toolbarLeft={toolbarLeft}
        header={header}
        rowKey="area_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="库区列表加载失败"
        emptyText="暂无库区"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="warehouse-areas.table.columns"
      />
      <Drawer
        title={editingArea ? '编辑库区' : '新建库区'}
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
            name="warehouse_id"
            label="仓库ID"
            rules={[{ required: true, message: '请输入仓库ID' }]}
          >
            <Input disabled={Boolean(editingArea)} />
          </Form.Item>
          <Form.Item
            name="code"
            label="库区编码"
            rules={[{ required: true, message: '请输入库区编码' }]}
          >
            <Input disabled={Boolean(editingArea)} />
          </Form.Item>
          <Form.Item
            name="name"
            label="库区名称"
            rules={[{ required: true, message: '请输入库区名称' }]}
          >
            <Input />
          </Form.Item>
          <Form.Item name="area_type" label="类型">
            <Select options={areaTypeOptions} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
          <Form.Item name="priority" label="优先级">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
