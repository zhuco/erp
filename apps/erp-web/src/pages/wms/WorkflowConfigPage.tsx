import { Alert, Button, Form, Select, Space, Switch, message } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import {
  getWarehouseWorkflowConfig,
  updateWarehouseWorkflowConfig,
  type WarehouseWorkflowConfigPayload
} from '@/api/workflowConfigs';
import { listWarehouses } from '@/api/warehouses';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

export const WorkflowConfigPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('wms.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<WarehouseWorkflowConfigPayload>();
  const [activeWarehouseId, setActiveWarehouseId] = useState<string>();

  const warehousesQuery = useQuery({
    queryKey: ['warehouses', tenantId, 'workflow-config'],
    queryFn: () => listWarehouses(tenantId, { offset: 0, limit: 200 }),
    enabled: Boolean(tenantId)
  });

  useEffect(() => {
    if (activeWarehouseId || !warehousesQuery.data?.items?.length) {
      return;
    }
    const defaultWarehouse =
      warehousesQuery.data.items.find((item) => item.is_default) ??
      warehousesQuery.data.items[0];
    if (defaultWarehouse) {
      setActiveWarehouseId(defaultWarehouse.warehouse_id);
    }
  }, [activeWarehouseId, warehousesQuery.data]);

  const warehouseOptions = useMemo(
    () =>
      (warehousesQuery.data?.items ?? []).map((warehouse) => ({
        label: `${warehouse.name} (${warehouse.code})`,
        value: warehouse.warehouse_id
      })),
    [warehousesQuery.data]
  );

  const workflowQuery = useQuery({
    queryKey: ['workflow-config', tenantId, activeWarehouseId],
    queryFn: () => getWarehouseWorkflowConfig(tenantId, activeWarehouseId ?? ''),
    enabled: Boolean(activeWarehouseId)
  });

  useEffect(() => {
    if (workflowQuery.data) {
      form.setFieldsValue({
        enable_wave: workflowQuery.data.enable_wave,
        require_inspection: workflowQuery.data.require_inspection,
        require_weighing: workflowQuery.data.require_weighing,
        require_pack: workflowQuery.data.require_pack
      });
    }
  }, [form, workflowQuery.data]);

  const handleSave = async () => {
    if (!activeWarehouseId) {
      message.error('请选择仓库');
      return;
    }
    try {
      const values = await form.validateFields();
      await updateWarehouseWorkflowConfig(tenantId, activeWarehouseId, values);
      message.success('流程配置已更新');
      queryClient.invalidateQueries({ queryKey: ['workflow-config', tenantId, activeWarehouseId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    }
  };

  return (
    <UiModule07SettingsLayout title="作业流程配置">
      <UiModule07SettingsSection title="流程配置">
        <Form
          form={form}
          layout="vertical"
          onValuesChange={(changedValues) => {
            if ('require_pack' in changedValues && !changedValues.require_pack) {
              form.setFieldsValue({ require_inspection: false, require_weighing: false });
            }
          }}
        >
          <Form.Item label="仓库">
            <Select
              allowClear
              placeholder="选择仓库"
              options={warehouseOptions}
              value={activeWarehouseId}
              onChange={(value) => setActiveWarehouseId(value)}
            />
          </Form.Item>
          {!activeWarehouseId ? (
            <Alert type="info" message="请选择仓库后配置流程" />
          ) : null}
          <Form.Item
            name="enable_wave"
            label="启用波次"
            valuePropName="checked"
            rules={[{ required: true, message: '请选择是否启用波次' }]}
          >
            <Switch disabled={!activeWarehouseId} data-testid="workflow-config-enable-wave-switch" />
          </Form.Item>
          <Form.Item
            name="require_pack"
            label="必须打包登记"
            valuePropName="checked"
            rules={[{ required: true, message: '请选择是否必须打包登记' }]}
          >
            <Switch disabled={!activeWarehouseId} />
          </Form.Item>
          <Form.Item shouldUpdate>
            {({ getFieldValue }) => (
              <Form.Item
                name="require_inspection"
                label="必须验货"
                valuePropName="checked"
                rules={[{ required: true, message: '请选择是否必须验货' }]}
              >
                <Switch
                  disabled={!activeWarehouseId || !getFieldValue('require_pack')}
                  data-testid="workflow-config-require-inspection-switch"
                />
              </Form.Item>
            )}
          </Form.Item>
          <Form.Item shouldUpdate>
            {({ getFieldValue }) => (
              <Form.Item
                name="require_weighing"
                label="必须称重"
                valuePropName="checked"
                rules={[{ required: true, message: '请选择是否必须称重' }]}
              >
                <Switch
                  disabled={!activeWarehouseId || !getFieldValue('require_pack')}
                  data-testid="workflow-config-require-weighing-switch"
                />
              </Form.Item>
            )}
          </Form.Item>
          <Space>
            <Button
              type="primary"
              onClick={handleSave}
              disabled={!canWrite || !activeWarehouseId}
              data-testid="workflow-config-save"
            >
              保存配置
            </Button>
          </Space>
        </Form>
      </UiModule07SettingsSection>
    </UiModule07SettingsLayout>
  );
};
