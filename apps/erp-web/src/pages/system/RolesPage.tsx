import { Button, Form, Input, Modal, Select, Space, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useCallback, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import {
  createRole,
  listPermissions,
  listRolePermissions,
  listRoles,
  updateRole,
  updateRolePermissions,
  type PermissionDefinition,
  type Role
} from '@/api/system';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type RoleFormValues = {
  name: string;
  permissions?: string[];
};

const formatDateTime = (value?: string) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');

export const RolesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const [pagination, setPagination] = useState({ current: 1, pageSize: 20 });
  const [modalOpen, setModalOpen] = useState(false);
  const [editingRole, setEditingRole] = useState<Role | null>(null);
  const [saving, setSaving] = useState(false);
  const [loadingRolePerms, setLoadingRolePerms] = useState(false);
  const [form] = Form.useForm<RoleFormValues>();

  const rolesQuery = useQuery({
    queryKey: ['roles', tenantId],
    queryFn: () => listRoles(tenantId),
    enabled: tenantReady
  });

  const permissionsQuery = useQuery({
    queryKey: ['permission-catalog', tenantId],
    queryFn: () => listPermissions(tenantId),
    enabled: tenantReady
  });

  const permissionOptions = useMemo(
    () =>
      (permissionsQuery.data ?? []).map((perm: PermissionDefinition) => ({
        label: `${perm.module} · ${perm.name}`,
        value: perm.code
      })),
    [permissionsQuery.data]
  );

  const openCreate = () => {
    setEditingRole(null);
    form.resetFields();
    setModalOpen(true);
  };

  const openEdit = useCallback(
    async (role: Role) => {
      setEditingRole(role);
      setModalOpen(true);
      setLoadingRolePerms(true);
      form.setFieldsValue({ name: role.name, permissions: [] });
      try {
        const perms = await listRolePermissions(tenantId, role.role_id);
        form.setFieldsValue({ name: role.name, permissions: perms });
      } catch (error) {
        message.error(error instanceof Error ? error.message : '加载角色权限失败');
      } finally {
        setLoadingRolePerms(false);
      }
    },
    [form, tenantId]
  );

  const handleSubmit = async () => {
    try {
      const values = await form.validateFields();
      setSaving(true);
      if (editingRole) {
        await updateRole(tenantId, editingRole.role_id, { name: values.name });
        await updateRolePermissions(tenantId, editingRole.role_id, {
          permissions: values.permissions ?? []
        });
        message.success('角色已更新');
      } else {
        await createRole(tenantId, {
          name: values.name,
          permissions: values.permissions ?? []
        });
        message.success('角色已创建');
      }
      setModalOpen(false);
      await rolesQuery.refetch();
    } catch (error) {
      if (error instanceof Error && error.name === 'Error') {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const data = rolesQuery.data ?? [];
  const current = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  const pageData = data.slice((current - 1) * pageSize, current * pageSize);

  const columns = useMemo<ColumnsType<Role>>(
    () => [
      { title: '角色名称', dataIndex: 'name' },
      {
        title: '创建时间',
        dataIndex: 'created_at',
        render: (_, record) => formatDateTime(record.created_at)
      },
      {
        title: '操作',
        dataIndex: 'actions',
        render: (_, record) => (
          <Button type="link" onClick={() => openEdit(record)}>
            编辑
          </Button>
        )
      }
    ],
    [openEdit]
  );

  const header = (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }}>
          角色管理
        </Typography.Title>
        <Button type="primary" onClick={openCreate}>
          新建角色
        </Button>
      </Space>
    </Space>
  );

  return (
    <>
      <UiModule03MainTableSection<Role>
        header={header}
        columns={columns}
        data={pageData}
        loading={rolesQuery.isFetching}
        error={rolesQuery.error}
        onRetry={() => rolesQuery.refetch()}
        errorTitle="加载角色失败"
        emptyText="暂无角色"
        total={data.length}
        pagination={pagination}
        onPaginationChange={setPagination}
        rowKey="role_id"
        enablePanels={false}
      />
      <Modal
        title={editingRole ? '编辑角色' : '新建角色'}
        open={modalOpen}
        onCancel={() => setModalOpen(false)}
        onOk={handleSubmit}
        confirmLoading={saving}
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="name"
            label="角色名称"
            rules={[{ required: true, message: '请输入角色名称' }]}
          >
            <Input placeholder="例如：运营主管" />
          </Form.Item>
          <Form.Item name="permissions" label="权限列表">
            <Select
              mode="multiple"
              allowClear
              placeholder={loadingRolePerms ? '加载中...' : '选择权限'}
              options={permissionOptions}
              loading={permissionsQuery.isFetching || loadingRolePerms}
              filterOption={(input, option) =>
                (option?.label ?? '').toString().toLowerCase().includes(input.toLowerCase())
              }
            />
          </Form.Item>
        </Form>
        {permissionsQuery.error && (
          <Space direction="vertical">
            <span>权限目录加载失败</span>
          </Space>
        )}
      </Modal>
    </>
  );
};
