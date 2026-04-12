import { Button, Form, Input, Modal, Select, Space, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useCallback, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import {
  createUser,
  getUserDataScopes,
  getUserRoles,
  listRoles,
  listUsers,
  updateUser,
  updateUserDataScopes,
  updateUserRoles,
  type Role,
  type User
} from '@/api/system';
import { listShops } from '@/api/shops';
import { listWarehouses } from '@/api/warehouses';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type UserFormValues = {
  email: string;
  password?: string;
  status?: string;
  role_ids?: string[];
};

type ScopeFormValues = {
  shop_ids: string[];
  warehouse_ids: string[];
};

const statusOptions = [
  { label: 'ACTIVE', value: 'ACTIVE' },
  { label: 'DISABLED', value: 'DISABLED' }
];

const formatDateTime = (value?: string) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

export const UsersPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const [pagination, setPagination] = useState({ current: 1, pageSize: 20 });
  const [modalOpen, setModalOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<User | null>(null);
  const [savingUser, setSavingUser] = useState(false);
  const [dataScopeOpen, setDataScopeOpen] = useState(false);
  const [scopingUser, setScopingUser] = useState<User | null>(null);
  const [savingScope, setSavingScope] = useState(false);
  const [userForm] = Form.useForm<UserFormValues>();
  const [scopeForm] = Form.useForm<ScopeFormValues>();

  const usersQuery = useQuery({
    queryKey: ['users', tenantId],
    queryFn: () => listUsers(tenantId, {}),
    enabled: tenantReady
  });

  const rolesQuery = useQuery({
    queryKey: ['roles', tenantId, 'options'],
    queryFn: () => listRoles(tenantId),
    enabled: tenantReady
  });

  const shopsQuery = useQuery({
    queryKey: ['shops', tenantId, 'scopes'],
    queryFn: () => listShops(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'scopes'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const roleOptions = useMemo(
    () =>
      (rolesQuery.data ?? []).map((role: Role) => ({
        label: role.name,
        value: role.role_id
      })),
    [rolesQuery.data]
  );

  const shopOptions = useMemo(
    () =>
      (shopsQuery.data?.items ?? []).map((shop) => ({
        label: `${shop.name} (${shop.platform})`,
        value: shop.shop_id
      })),
    [shopsQuery.data?.items]
  );

  const warehouseOptions = useMemo(
    () =>
      (warehousesQuery.data?.items ?? []).map((warehouse) => ({
        label: `${warehouse.name} (${warehouse.code})`,
        value: warehouse.warehouse_id
      })),
    [warehousesQuery.data?.items]
  );

  const openCreate = () => {
    setEditingUser(null);
    userForm.resetFields();
    setModalOpen(true);
  };

  const openEdit = useCallback(
    async (user: User) => {
      setEditingUser(user);
      userForm.setFieldsValue({
        email: user.email,
        status: user.status,
        password: ''
      });
      setModalOpen(true);
      try {
        const roles = await getUserRoles(tenantId, user.user_id);
        userForm.setFieldsValue({ role_ids: roles.roles.map((role) => role.role_id) });
      } catch (error) {
        message.error(error instanceof Error ? error.message : '加载用户角色失败');
      }
    },
    [tenantId, userForm]
  );

  const openDataScopes = useCallback(
    async (user: User) => {
      setScopingUser(user);
      setDataScopeOpen(true);
      scopeForm.resetFields();
      try {
        const scopes = await getUserDataScopes(tenantId, user.user_id);
        scopeForm.setFieldsValue({
          shop_ids: scopes.shop_ids ?? [],
          warehouse_ids: scopes.warehouse_ids ?? []
        });
      } catch (error) {
        message.error(error instanceof Error ? error.message : '加载数据权限失败');
      }
    },
    [scopeForm, tenantId]
  );

  const handleSubmit = async () => {
    try {
      const values = await userForm.validateFields();
      setSavingUser(true);
      if (editingUser) {
        await updateUser(tenantId, editingUser.user_id, {
          email: values.email,
          status: values.status,
          password: values.password || undefined
        });
        if (values.role_ids) {
          await updateUserRoles(tenantId, editingUser.user_id, {
            role_ids: values.role_ids
          });
        }
        message.success('用户已更新');
      } else {
        await createUser(tenantId, {
          email: values.email,
          password: values.password ?? '',
          status: values.status,
          role_ids: values.role_ids
        });
        message.success('用户已创建');
      }
      setModalOpen(false);
      await usersQuery.refetch();
    } catch (error) {
      if (error instanceof Error) {
        message.error(error.message);
      }
    } finally {
      setSavingUser(false);
    }
  };

  const handleScopeSubmit = async () => {
    if (!scopingUser) {
      return;
    }
    try {
      const values = await scopeForm.validateFields();
      setSavingScope(true);
      await updateUserDataScopes(tenantId, scopingUser.user_id, values);
      message.success('数据权限已更新');
      setDataScopeOpen(false);
    } catch (error) {
      if (error instanceof Error) {
        message.error(error.message);
      }
    } finally {
      setSavingScope(false);
    }
  };

  const data = usersQuery.data ?? [];
  const current = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  const pageData = data.slice((current - 1) * pageSize, current * pageSize);

  const columns = useMemo<ColumnsType<User>>(
    () => [
      { title: '邮箱', dataIndex: 'email' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={record.status === 'ACTIVE' ? 'green' : 'red'}>{record.status}</Tag>
        )
      },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      },
      {
        title: '操作',
        dataIndex: 'actions',
        render: (_, record) => (
          <Space>
            <Button type="link" onClick={() => openEdit(record)}>
              编辑
            </Button>
            <Button type="link" onClick={() => openDataScopes(record)}>
              数据权限
            </Button>
          </Space>
        )
      }
    ],
    [openEdit, openDataScopes]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          用户管理
        </Typography.Title>
        <Button type="primary" onClick={openCreate}>
          新建用户
        </Button>
      </Space>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<User>
        header={header}
        columns={columns}
        data={pageData}
        loading={usersQuery.isFetching}
        error={usersQuery.error}
        onRetry={() => usersQuery.refetch()}
        errorTitle="加载用户失败"
        emptyText="暂无用户"
        total={data.length}
        pagination={pagination}
        onPaginationChange={setPagination}
        rowKey="user_id"
        enablePanels={false}
      />
      <Modal
        title={editingUser ? '编辑用户' : '新建用户'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSubmit}
        confirmLoading={savingUser}
      >
        <Form form={userForm} layout="vertical">
          <Form.Item
            name="email"
            label="邮箱"
            rules={[{ required: true, message: '请输入邮箱' }]}
          >
            <Input placeholder="user@example.com" />
          </Form.Item>
          <Form.Item
            name="password"
            label={editingUser ? '重置密码（可选）' : '密码'}
            rules={editingUser ? [] : [{ required: true, message: '请输入密码' }]}
          >
            <Input.Password placeholder={editingUser ? '留空表示不修改' : '请输入密码'} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} placeholder="选择状态" />
          </Form.Item>
          <Form.Item name="role_ids" label="角色">
            <Select
              mode="multiple"
              allowClear
              placeholder="选择角色"
              options={roleOptions}
              loading={rolesQuery.isFetching}
            />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title={scopingUser ? `数据权限 - ${scopingUser.email}` : '数据权限'}
        open={dataScopeOpen}
        onCancel={() => setDataScopeOpen(false)}
        onOk={handleScopeSubmit}
        confirmLoading={savingScope}
      >
        <Form form={scopeForm} layout="vertical">
          <Form.Item name="shop_ids" label="可访问店铺">
            <Select
              mode="multiple"
              allowClear
              placeholder="选择店铺"
              options={shopOptions}
              loading={shopsQuery.isFetching}
            />
          </Form.Item>
          <Form.Item name="warehouse_ids" label="可访问仓库">
            <Select
              mode="multiple"
              allowClear
              placeholder="选择仓库"
              options={warehouseOptions}
              loading={warehousesQuery.isFetching}
            />
          </Form.Item>
        </Form>
      </Modal>
    </>
  );
};
