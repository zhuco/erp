import { Button, Form, Select, Space, message } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useQuery } from '@tanstack/react-query';

import {
  getUserDataScopes,
  listUsers,
  updateUserDataScopes,
  type DataScopeResponse,
  type User
} from '@/api/system';
import { listShops } from '@/api/shops';
import { listWarehouses } from '@/api/warehouses';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type ScopeFormValues = {
  shop_ids: string[];
  warehouse_ids: string[];
  merchant_ids: string[];
  organization_ids: string[];
};

type ExtendedDataScope = DataScopeResponse & {
  merchant_ids?: string[];
  organization_ids?: string[];
};

export const DataScopesPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const [selectedUserId, setSelectedUserId] = useState<string>();
  const [saving, setSaving] = useState(false);
  const [form] = Form.useForm<ScopeFormValues>();

  const usersQuery = useQuery({
    queryKey: ['users', tenantId, 'scope-page'],
    queryFn: () => listUsers(tenantId, {}),
    enabled: tenantReady
  });

  const shopsQuery = useQuery({
    queryKey: ['shops', tenantId, 'scope-page'],
    queryFn: () => listShops(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'scope-page'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: tenantReady
  });

  const userOptions = useMemo(
    () =>
      (usersQuery.data ?? []).map((user: User) => ({
        label: user.email,
        value: user.user_id
      })),
    [usersQuery.data]
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

  useEffect(() => {
    if (!selectedUserId) {
      form.resetFields();
      return;
    }
    const loadScopes = async () => {
      try {
        const scopes = (await getUserDataScopes(
          tenantId,
          selectedUserId
        )) as ExtendedDataScope;
        form.setFieldsValue({
          shop_ids: scopes.shop_ids ?? [],
          warehouse_ids: scopes.warehouse_ids ?? [],
          merchant_ids: scopes.merchant_ids ?? [],
          organization_ids: scopes.organization_ids ?? []
        });
      } catch (error) {
        message.error(error instanceof Error ? error.message : '加载数据权限失败');
      }
    };
    void loadScopes();
  }, [form, selectedUserId, tenantId]);

  const handleSave = async () => {
    if (!selectedUserId) {
      message.warning('请选择用户');
      return;
    }
    try {
      const values = await form.validateFields();
      setSaving(true);
      const payload = {
        shop_ids: values.shop_ids ?? [],
        warehouse_ids: values.warehouse_ids ?? [],
        merchant_ids: values.merchant_ids ?? [],
        organization_ids: values.organization_ids ?? []
      };
      await updateUserDataScopes(
        tenantId,
        selectedUserId,
        payload as unknown as DataScopeUpdateRequest
      );
      message.success('数据权限已更新');
    } catch (error) {
      if (error instanceof Error) {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <UiModule07SettingsLayout title="数据权限">
      <UiModule07SettingsSection
        title="用户数据权限"
        actions={
          <Button type="primary" onClick={handleSave} loading={saving}>
            保存
          </Button>
        }
      >
        <Space direction="vertical" style={{ width: '100%' }} size={16}>
          <Select
            placeholder="选择用户"
            options={userOptions}
            loading={usersQuery.isFetching}
            value={selectedUserId}
            onChange={setSelectedUserId}
            allowClear
          />
          <Form form={form} layout="vertical">
            <Form.Item name="shop_ids" label="可访问店铺">
              <Select
                mode="multiple"
                allowClear
                options={shopOptions}
                loading={shopsQuery.isFetching}
              />
            </Form.Item>
            <Form.Item name="warehouse_ids" label="可访问仓库">
              <Select
                mode="multiple"
                allowClear
                options={warehouseOptions}
                loading={warehousesQuery.isFetching}
              />
            </Form.Item>
            <Form.Item name="merchant_ids" label="可访问商家">
              <Select mode="tags" allowClear placeholder="输入或选择商家 ID" />
            </Form.Item>
            <Form.Item name="organization_ids" label="可访问组织">
              <Select mode="tags" allowClear placeholder="输入或选择组织 ID" />
            </Form.Item>
          </Form>
        </Space>
      </UiModule07SettingsSection>
    </UiModule07SettingsLayout>
  );
};
