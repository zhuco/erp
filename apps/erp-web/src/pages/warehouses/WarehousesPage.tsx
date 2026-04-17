import {
  Button,
  Card,
  Col,
  Descriptions,
  Drawer,
  Form,
  Input,
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
import type { Key } from 'react';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { Warehouse, WarehouseCreatePayload, WarehouseUpdatePayload } from '@/api/warehouses';
import {
  createWarehouse,
  deleteWarehouse,
  listWarehouses,
  setDefaultWarehouse,
  updateWarehouse
} from '@/api/warehouses';
import { ServerTable } from '@/components/ServerTable';
import { WorkbenchListPage } from '@/components/workbench/WorkbenchListPage';
import { UiModule02ToolbarRight } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  keyword?: string;
  status?: string;
};

type FormValues = {
  name: string;
  code: string;
  status?: string;
  is_default?: boolean;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

export const WarehousesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('warehouses.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingWarehouse, setEditingWarehouse] = useState<Warehouse | null>(null);
  const [saving, setSaving] = useState(false);
  const [selectedWarehouse, setSelectedWarehouse] = useState<Warehouse | null>(null);
  const [settingDefaultId, setSettingDefaultId] = useState<string | null>(null);
  const [statusUpdatingId, setStatusUpdatingId] = useState<string | null>(null);
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
    queryKey: ['warehouses', tenantId, query],
    queryFn: () => listWarehouses(tenantId, query),
    keepPreviousData: true
  });

  const defaultMutation = useMutation({
    mutationFn: (warehouseId: string) => setDefaultWarehouse(tenantId, warehouseId),
    onMutate: (warehouseId) => setSettingDefaultId(warehouseId),
    onSuccess: () => {
      message.success('默认仓库已更新');
      queryClient.invalidateQueries({ queryKey: ['warehouses', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '设置默认仓库失败');
    },
    onSettled: () => setSettingDefaultId(null)
  });

  const deleteMutation = useMutation({
    mutationFn: (warehouseId: string) => deleteWarehouse(tenantId, warehouseId),
    onMutate: (warehouseId) => setDeletingId(warehouseId),
    onSuccess: () => {
      message.success('仓库已删除');
      queryClient.invalidateQueries({ queryKey: ['warehouses', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除仓库失败');
    },
    onSettled: () => setDeletingId(null)
  });

  const statusMutation = useMutation({
    mutationFn: ({ warehouseId, status }: { warehouseId: string; status: string }) =>
      updateWarehouse(tenantId, warehouseId, { status }),
    onMutate: ({ warehouseId }) => setStatusUpdatingId(warehouseId),
    onSuccess: () => {
      message.success('仓库状态已更新');
      queryClient.invalidateQueries({ queryKey: ['warehouses', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '更新仓库状态失败');
    },
    onSettled: () => setStatusUpdatingId(null)
  });

  const columns = useMemo<ColumnsType<Warehouse>>(
    () => [
      {
        title: '仓库名称',
        dataIndex: 'name'
      },
      {
        title: '仓库编码',
        dataIndex: 'code'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) => (value === 'DISABLED' ? <Tag color="red">停用</Tag> : '启用')
      },
      {
        title: '默认仓库',
        dataIndex: 'is_default',
        render: (value) => (value ? <Tag color="green">默认</Tag> : '-')
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at ?? record.created_at)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => {
          const disabled = !canWrite;
          const nextStatus = record.status === 'DISABLED' ? 'ACTIVE' : 'DISABLED';
          return (
            <Space>
              <Button
                size="small"
                disabled={disabled}
                onClick={() => {
                  setEditingWarehouse(record);
                  drawerForm.setFieldsValue({
                    name: record.name,
                    code: record.code,
                    status: record.status ?? 'ACTIVE',
                    is_default: record.is_default
                  });
                  setDrawerOpen(true);
                }}
              >
                编辑
              </Button>
              <Popconfirm
                title={record.status === 'DISABLED' ? '确认启用该仓库？' : '确认停用该仓库？'}
                okText={record.status === 'DISABLED' ? '启用' : '停用'}
                cancelText="取消"
                onConfirm={() =>
                  statusMutation.mutate({ warehouseId: record.warehouse_id, status: nextStatus })
                }
                disabled={disabled}
              >
                <Button
                  size="small"
                  loading={statusUpdatingId === record.warehouse_id}
                  disabled={disabled}
                >
                  {record.status === 'DISABLED' ? '启用' : '停用'}
                </Button>
              </Popconfirm>
              <Button
                size="small"
                disabled={disabled || record.is_default}
                loading={settingDefaultId === record.warehouse_id}
                onClick={() => defaultMutation.mutate(record.warehouse_id)}
              >
                设为默认
              </Button>
              <Popconfirm
                title="确认删除该仓库？"
                okText="删除"
                cancelText="取消"
                onConfirm={() => deleteMutation.mutate(record.warehouse_id)}
                disabled={disabled}
              >
                <Button
                  size="small"
                  danger
                  disabled={disabled}
                  loading={deletingId === record.warehouse_id}
                >
                  删除
                </Button>
              </Popconfirm>
            </Space>
          );
        }
      }
    ],
    [
      canWrite,
      defaultMutation,
      deleteMutation,
      drawerForm,
      settingDefaultId,
      statusMutation,
      statusUpdatingId,
      deletingId
    ]
  );

  const selectedRowKeys: Key[] = selectedWarehouse ? [selectedWarehouse.warehouse_id] : [];

  const detailContent = selectedWarehouse ? (
    <Descriptions size="small" column={1} bordered>
      <Descriptions.Item label="仓库名称">{selectedWarehouse.name}</Descriptions.Item>
      <Descriptions.Item label="仓库编码">{selectedWarehouse.code}</Descriptions.Item>
      <Descriptions.Item label="状态">
        {selectedWarehouse.status === 'DISABLED' ? '停用' : '启用'}
      </Descriptions.Item>
      <Descriptions.Item label="默认仓库">
        {selectedWarehouse.is_default ? '是' : '否'}
      </Descriptions.Item>
      <Descriptions.Item label="更新时间">
        {formatDateTime(selectedWarehouse.updated_at ?? selectedWarehouse.created_at)}
      </Descriptions.Item>
    </Descriptions>
  ) : (
    <Typography.Text type="secondary">请选择一行查看详情</Typography.Text>
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
    setEditingWarehouse(null);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ status: 'ACTIVE', is_default: false });
    setDrawerOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const payload: WarehouseCreatePayload | WarehouseUpdatePayload = {
        name: values.name.trim(),
        code: values.code.trim(),
        status: values.status?.trim() || undefined
      };
      setSaving(true);
      if (editingWarehouse) {
        await updateWarehouse(tenantId, editingWarehouse.warehouse_id, payload);
        message.success('仓库已更新');
      } else {
        const createPayload: WarehouseCreatePayload = {
          ...payload,
          is_default: values.is_default
        };
        await createWarehouse(tenantId, createPayload);
        message.success('仓库已创建');
      }
      setDrawerOpen(false);
      setEditingWarehouse(null);
      await queryClient.invalidateQueries({ queryKey: ['warehouses', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <>
      <WorkbenchListPage
        filter={
          <Card size="small">
            <Form form={form} layout="vertical" onFinish={handleSearch}>
              <Row gutter={12}>
                <Col xs={24}>
                  <Form.Item name="keyword" label="关键字">
                    <Input allowClear placeholder="仓库名称/编码" />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Form.Item name="status" label="状态">
                    <Select allowClear options={statusOptions} placeholder="全部状态" />
                  </Form.Item>
                </Col>
                <Col xs={24}>
                  <Space>
                    <Button type="primary" htmlType="submit" size="small">
                      查询
                    </Button>
                    <Button onClick={handleReset} size="small">
                      重置
                    </Button>
                  </Space>
                </Col>
              </Row>
            </Form>
          </Card>
        }
        toolbarLeft={
          <>
            <Typography.Title level={4} style={{ margin: 0 }}>
              仓库管理
            </Typography.Title>
            <Button type="primary" size="small" disabled={!canWrite} onClick={openCreateDrawer}>
              新建仓库
            </Button>
          </>
        }
        toolbarRight={
          <UiModule02ToolbarRight
            onExport={() => message.info('导出功能待接入')}
            onPrint={() => message.info('打印功能待接入')}
            onHelp={() => message.info('帮助功能待接入')}
          />
        }
        table={
          <ServerTable<Warehouse>
            rowKey="warehouse_id"
            columns={columns}
            data={data?.items ?? []}
            loading={isLoading || isFetching}
            error={error}
            onRetry={() => refetch()}
            errorTitle="仓库列表加载失败"
            emptyText="暂无仓库"
            total={data?.total ?? 0}
            pagination={pagination}
            onPaginationChange={setPagination}
            columnSettingsKey="warehouses.table.columns"
            rowSelection={{
              selectedRowKeys,
              onChange: (keys, rows) => {
                setSelectedWarehouse(rows[0] ?? null);
              }
            }}
            onRow={(record) => ({
              onClick: () => setSelectedWarehouse(record)
            })}
            rowClassName={(record) =>
              record.warehouse_id === selectedWarehouse?.warehouse_id ? 'ant-table-row-selected' : ''
            }
          />
        }
        detail={detailContent}
        bottomTabs={<Typography.Text type="secondary">请选择一行查看详情</Typography.Text>}
      />
      <Drawer
        title={editingWarehouse ? '编辑仓库' : '新建仓库'}
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
            label="仓库名称"
            rules={[{ required: true, message: '请输入仓库名称' }]}
          >
            <Input placeholder="例如：上海主仓" />
          </Form.Item>
          <Form.Item
            name="code"
            label="仓库编码"
            rules={[{ required: true, message: '请输入仓库编码' }]}
          >
            <Input placeholder="例如：SH-01" />
          </Form.Item>
          <Form.Item name="status" label="状态" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
          {!editingWarehouse ? (
            <Form.Item name="is_default" label="设为默认" valuePropName="checked">
              <Switch />
            </Form.Item>
          ) : null}
        </Form>
      </Drawer>
    </>
  );
};
