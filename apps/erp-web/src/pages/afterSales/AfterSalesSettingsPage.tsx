import {
  Button,
  Drawer,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  SalesShopConfig,
  SalesShopConfigListQuery,
  SalesShopConfigRequest
} from '@/api/salesShops';
import { listSalesShopConfigs, updateSalesShopConfig } from '@/api/salesShops';
import { listSettings, upsertSetting } from '@/api/system';
import type {
  AfterSalesReason,
  AfterSalesReasonListQuery,
  AfterSalesQualityRule,
  AfterSalesQualityRuleListQuery,
  AfterSalesReasonPayload,
  AfterSalesQualityRulePayload
} from '@/api/afterSalesSettings';
import {
  createAfterSalesQualityRule,
  createAfterSalesReason,
  deleteAfterSalesQualityRule,
  deleteAfterSalesReason,
  listAfterSalesQualityRules,
  listAfterSalesReasons,
  updateAfterSalesQualityRule,
  updateAfterSalesReason
} from '@/api/afterSalesSettings';
import { ServerTable } from '@/components/ServerTable';
import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

const parseSettingBool = (value?: Record<string, unknown> | null) => {
  if (!value) {
    return undefined;
  }
  if (typeof value === 'boolean') {
    return value;
  }
  if (typeof value === 'object' && 'value' in value) {
    const raw = value as { value?: unknown };
    if (typeof raw.value === 'boolean') {
      return raw.value;
    }
  }
  return undefined;
};

type ReasonFormValues = {
  reason_code?: string;
  label: string;
  category?: string;
  status?: string;
  sort_order?: number;
};

type QualityRuleFormValues = {
  after_sales_type: string;
  result_code: string;
  status?: string;
  sort_order?: number;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const afterSalesTypeOptions = [
  { label: '仅退款', value: 'REFUND_ONLY' },
  { label: '退货退款', value: 'RETURN_REFUND' },
  { label: '换货', value: 'EXCHANGE' },
  { label: '补发', value: 'RESEND' }
];

const qualityResultOptions = [
  { label: '可二次销售', value: 'RESALE' },
  { label: '报废', value: 'SCRAP' },
  { label: '返修', value: 'REPAIR' }
];

export const AfterSalesSettingsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const queryClient = useQueryClient();
  const canWriteSettings = useCan('SYS.SETTINGS.WRITE');
  const canWrite = useCan('settings.read');
  const canWriteAfterSales = useCan('aftersales.write');
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 10
  });
  const [updatingShopId, setUpdatingShopId] = useState<string | null>(null);
  const [updatingLink, setUpdatingLink] = useState(false);
  const [reasonForm] = Form.useForm<ReasonFormValues>();
  const [ruleForm] = Form.useForm<QualityRuleFormValues>();
  const [reasonPagination, setReasonPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [rulePagination, setRulePagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [reasonDrawerOpen, setReasonDrawerOpen] = useState(false);
  const [ruleDrawerOpen, setRuleDrawerOpen] = useState(false);
  const [editingReason, setEditingReason] = useState<AfterSalesReason | null>(null);
  const [editingRule, setEditingRule] = useState<AfterSalesQualityRule | null>(null);
  const [savingReason, setSavingReason] = useState(false);
  const [savingRule, setSavingRule] = useState(false);
  const [deletingReason, setDeletingReason] = useState<string | null>(null);
  const [deletingRule, setDeletingRule] = useState<string | null>(null);

  const shopQuery = useMemo<SalesShopConfigListQuery>(
    () => ({
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 10),
      limit: pagination.pageSize ?? 10
    }),
    [pagination.current, pagination.pageSize]
  );

  const shopQueryResult = useQuery({
    queryKey: ['sales-shop-configs', tenantId, shopQuery],
    queryFn: () => listSalesShopConfigs(tenantId, shopQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const returnInboundSetting = useQuery({
    queryKey: ['settings', tenantId, 'service.enable_return_inbound_link'],
    queryFn: () =>
      listSettings(tenantId, {
        scope_type: 'TENANT',
        key: 'service.enable_return_inbound_link'
      }),
    enabled: tenantReady
  });

  const returnInboundLink = useMemo(() => {
    const setting = returnInboundSetting.data?.[0];
    return parseSettingBool(setting?.value_json) ?? true;
  }, [returnInboundSetting.data]);

  const reasonQuery = useMemo<AfterSalesReasonListQuery>(
    () => ({
      offset: ((reasonPagination.current ?? 1) - 1) * (reasonPagination.pageSize ?? 20),
      limit: reasonPagination.pageSize ?? 20
    }),
    [reasonPagination.current, reasonPagination.pageSize]
  );

  const reasonQueryResult = useQuery({
    queryKey: ['after-sales-reasons', tenantId, reasonQuery],
    queryFn: () => listAfterSalesReasons(tenantId, reasonQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const ruleQuery = useMemo<AfterSalesQualityRuleListQuery>(
    () => ({
      offset: ((rulePagination.current ?? 1) - 1) * (rulePagination.pageSize ?? 20),
      limit: rulePagination.pageSize ?? 20
    }),
    [rulePagination.current, rulePagination.pageSize]
  );

  const ruleQueryResult = useQuery({
    queryKey: ['after-sales-quality-rules', tenantId, ruleQuery],
    queryFn: () => listAfterSalesQualityRules(tenantId, ruleQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const handleShopToggle = async (record: SalesShopConfig, checked: boolean) => {
    if (!canWrite) {
      message.error('暂无权限修改网店配置');
      return;
    }
    setUpdatingShopId(record.shop_id);
    try {
      const payload: SalesShopConfigRequest = {
        auto_download_orders: record.auto_download_orders,
        auto_to_review: record.auto_to_review,
        download_interval_minutes: record.download_interval_minutes,
        auto_merge_non_shipping_time: record.auto_merge_non_shipping_time,
        non_shipping_time_start: record.non_shipping_time_start ?? null,
        non_shipping_time_end: record.non_shipping_time_end ?? null,
        enable_stock_sync: record.enable_stock_sync,
        enable_after_sales_download: checked,
        enable_logistics_upload: record.enable_logistics_upload
      };
      await updateSalesShopConfig(tenantId, record.shop_id, payload);
      message.success('售后下载设置已更新');
      await queryClient.invalidateQueries({ queryKey: ['sales-shop-configs', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新售后下载设置失败');
    } finally {
      setUpdatingShopId(null);
    }
  };

  const handleReturnInboundToggle = async (checked: boolean) => {
    if (!canWriteSettings) {
      message.error('暂无权限更新售后联动设置');
      return;
    }
    setUpdatingLink(true);
    try {
      await upsertSetting(tenantId, {
        scope_type: 'TENANT',
        scope_id: null,
        key: 'service.enable_return_inbound_link',
        value: checked
      });
      message.success(checked ? '已开启售后入库联动' : '已关闭售后入库联动');
      await returnInboundSetting.refetch();
    } catch (err) {
      message.error(err instanceof Error ? err.message : '更新设置失败');
    } finally {
      setUpdatingLink(false);
    }
  };

  const openReasonDrawer = (record?: AfterSalesReason) => {
    if (record) {
      setEditingReason(record);
      reasonForm.setFieldsValue({
        reason_code: record.reason_code,
        label: record.label,
        category: record.category ?? undefined,
        status: record.status,
        sort_order: record.sort_order
      });
    } else {
      setEditingReason(null);
      reasonForm.resetFields();
      reasonForm.setFieldsValue({
        status: 'ACTIVE',
        sort_order: 0
      });
    }
    setReasonDrawerOpen(true);
  };

  const openRuleDrawer = (record?: AfterSalesQualityRule) => {
    if (record) {
      setEditingRule(record);
      ruleForm.setFieldsValue({
        after_sales_type: record.after_sales_type,
        result_code: record.result_code,
        status: record.status,
        sort_order: record.sort_order
      });
    } else {
      setEditingRule(null);
      ruleForm.resetFields();
      ruleForm.setFieldsValue({
        status: 'ACTIVE',
        sort_order: 0
      });
    }
    setRuleDrawerOpen(true);
  };

  const handleSaveReason = async () => {
    try {
      const values = await reasonForm.validateFields();
      const payload: AfterSalesReasonPayload = {
        reason_code: values.reason_code?.trim(),
        label: values.label.trim(),
        category: values.category?.trim() || undefined,
        status: values.status,
        sort_order: values.sort_order
      };
      setSavingReason(true);
      if (editingReason) {
        await updateAfterSalesReason(tenantId, editingReason.reason_code, payload);
        message.success('售后原因已更新');
      } else {
        await createAfterSalesReason(tenantId, payload);
        message.success('售后原因已创建');
      }
      setReasonDrawerOpen(false);
      setEditingReason(null);
      await queryClient.invalidateQueries({ queryKey: ['after-sales-reasons', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSavingReason(false);
    }
  };

  const handleDeleteReason = async (reasonCode: string) => {
    setDeletingReason(reasonCode);
    try {
      await deleteAfterSalesReason(tenantId, reasonCode);
      message.success('售后原因已删除');
      await queryClient.invalidateQueries({ queryKey: ['after-sales-reasons', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除售后原因失败');
    } finally {
      setDeletingReason(null);
    }
  };

  const handleSaveRule = async () => {
    try {
      const values = await ruleForm.validateFields();
      const payload: AfterSalesQualityRulePayload = {
        after_sales_type: values.after_sales_type,
        result_code: values.result_code,
        status: values.status,
        sort_order: values.sort_order
      };
      setSavingRule(true);
      if (editingRule) {
        await updateAfterSalesQualityRule(tenantId, editingRule.rule_id, payload);
        message.success('质检规则已更新');
      } else {
        await createAfterSalesQualityRule(tenantId, payload);
        message.success('质检规则已创建');
      }
      setRuleDrawerOpen(false);
      setEditingRule(null);
      await queryClient.invalidateQueries({ queryKey: ['after-sales-quality-rules', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSavingRule(false);
    }
  };

  const handleDeleteRule = async (ruleId: string) => {
    setDeletingRule(ruleId);
    try {
      await deleteAfterSalesQualityRule(tenantId, ruleId);
      message.success('质检规则已删除');
      await queryClient.invalidateQueries({ queryKey: ['after-sales-quality-rules', tenantId] });
    } catch (err) {
      message.error(err instanceof Error ? err.message : '删除质检规则失败');
    } finally {
      setDeletingRule(null);
    }
  };

  const columns = useMemo<ColumnsType<SalesShopConfig>>(
    () => [
      { title: '店铺', dataIndex: 'name' },
      { title: '平台', dataIndex: 'platform' },
      { title: 'Shop ID', dataIndex: 'shop_id' },
      {
        title: '售后自动下载',
        dataIndex: 'enable_after_sales_download',
        render: (_, record) => (
          <Switch
            checked={record.enable_after_sales_download}
            loading={updatingShopId === record.shop_id}
            onChange={(checked) => handleShopToggle(record, checked)}
          />
        )
      }
    ],
    [handleShopToggle, updatingShopId]
  );

  const reasonColumns = useMemo<ColumnsType<AfterSalesReason>>(
    () => [
      { title: '原因编码', dataIndex: 'reason_code' },
      { title: '原因名称', dataIndex: 'label' },
      {
        title: '分类',
        dataIndex: 'category',
        render: (value?: string | null) => value || '-'
      },
      {
        title: '排序',
        dataIndex: 'sort_order'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'DISABLED' ? <Tag color="red">停用</Tag> : <Tag color="green">启用</Tag>
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={!canWriteAfterSales}
              onClick={() => openReasonDrawer(record)}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该原因？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => handleDeleteReason(record.reason_code)}
              disabled={!canWriteAfterSales}
            >
              <Button
                size="small"
                danger
                disabled={!canWriteAfterSales}
                loading={deletingReason === record.reason_code}
              >
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWriteAfterSales, deletingReason]
  );

  const ruleColumns = useMemo<ColumnsType<AfterSalesQualityRule>>(
    () => [
      {
        title: '售后类型',
        dataIndex: 'after_sales_type',
        render: (value) =>
          afterSalesTypeOptions.find((opt) => opt.value === value)?.label || value
      },
      {
        title: '质检结论',
        dataIndex: 'result_code',
        render: (value) =>
          qualityResultOptions.find((opt) => opt.value === value)?.label || value
      },
      {
        title: '排序',
        dataIndex: 'sort_order'
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) =>
          value === 'DISABLED' ? <Tag color="red">停用</Tag> : <Tag color="green">启用</Tag>
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button
              size="small"
              disabled={!canWriteAfterSales}
              onClick={() => openRuleDrawer(record)}
            >
              编辑
            </Button>
            <Popconfirm
              title="确认删除该规则？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => handleDeleteRule(record.rule_id)}
              disabled={!canWriteAfterSales}
            >
              <Button
                size="small"
                danger
                disabled={!canWriteAfterSales}
                loading={deletingRule === record.rule_id}
              >
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWriteAfterSales, deletingRule]
  );

  return (
    <>
      <UiModule07SettingsLayout
        title={<span data-testid="after-sales-settings-title">售后设置</span>}
      >
        <UiModule07SettingsSection title="售后自动下载">
          <ServerTable<SalesShopConfig>
            rowKey="shop_id"
            columns={columns}
            data={shopQueryResult.data?.items ?? []}
            loading={shopQueryResult.isFetching}
            error={shopQueryResult.error}
            onRetry={() => shopQueryResult.refetch()}
            errorTitle="网店配置加载失败"
            emptyText="暂无网店配置"
            total={shopQueryResult.data?.total ?? 0}
            pagination={pagination}
            onPaginationChange={setPagination}
            columnSettingsKey="after-sales.settings.shop.columns"
          />
        </UiModule07SettingsSection>
        <UiModule07SettingsSection title="退货入库联动">
          <Space align="center" size={12}>
            <Switch
              checked={returnInboundLink}
              loading={updatingLink || returnInboundSetting.isFetching}
              onChange={handleReturnInboundToggle}
              disabled={!canWriteSettings}
            />
            <Typography.Text>
              {returnInboundLink ? '已开启退货入库联动' : '已关闭退货入库联动'}
            </Typography.Text>
          </Space>
          <Typography.Text type="secondary">
            开启后，售后收货会自动生成退货入库单并同步库存。
          </Typography.Text>
        </UiModule07SettingsSection>
        <UiModule07SettingsSection
          title="售后原因字典"
          actions={
            <Button
              type="primary"
              disabled={!canWriteAfterSales}
              onClick={() => openReasonDrawer()}
            >
              新增原因
            </Button>
          }
        >
          <ServerTable<AfterSalesReason>
            rowKey="reason_code"
            columns={reasonColumns}
            data={reasonQueryResult.data?.items ?? []}
            loading={reasonQueryResult.isFetching}
            error={reasonQueryResult.error}
            onRetry={() => reasonQueryResult.refetch()}
            errorTitle="售后原因加载失败"
            emptyText="暂无售后原因"
            total={reasonQueryResult.data?.total ?? 0}
            pagination={reasonPagination}
            onPaginationChange={setReasonPagination}
            columnSettingsKey="after-sales.settings.reasons.columns"
          />
        </UiModule07SettingsSection>
        <UiModule07SettingsSection
          title="质检规则"
          actions={
            <Button
              type="primary"
              disabled={!canWriteAfterSales}
              onClick={() => openRuleDrawer()}
            >
              新增规则
            </Button>
          }
        >
          <ServerTable<AfterSalesQualityRule>
            rowKey="rule_id"
            columns={ruleColumns}
            data={ruleQueryResult.data?.items ?? []}
            loading={ruleQueryResult.isFetching}
            error={ruleQueryResult.error}
            onRetry={() => ruleQueryResult.refetch()}
            errorTitle="质检规则加载失败"
            emptyText="暂无质检规则"
            total={ruleQueryResult.data?.total ?? 0}
            pagination={rulePagination}
            onPaginationChange={setRulePagination}
            columnSettingsKey="after-sales.settings.quality.columns"
          />
        </UiModule07SettingsSection>
      </UiModule07SettingsLayout>
      <Drawer
        title={editingReason ? '编辑售后原因' : '新增售后原因'}
        open={reasonDrawerOpen}
        onClose={() => setReasonDrawerOpen(false)}
        width={420}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setReasonDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={savingReason} onClick={handleSaveReason}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={reasonForm} layout="vertical">
          <Form.Item
            name="reason_code"
            label="原因编码"
            rules={[{ required: !editingReason, message: '请输入原因编码' }]}
          >
            <Input disabled={Boolean(editingReason)} placeholder="例如：DEFECT" />
          </Form.Item>
          <Form.Item
            name="label"
            label="原因名称"
            rules={[{ required: true, message: '请输入原因名称' }]}
          >
            <Input placeholder="例如：商品破损" />
          </Form.Item>
          <Form.Item name="category" label="分类">
            <Input placeholder="例如：质量问题" />
          </Form.Item>
          <Form.Item name="sort_order" label="排序">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
        </Form>
      </Drawer>
      <Drawer
        title={editingRule ? '编辑质检规则' : '新增质检规则'}
        open={ruleDrawerOpen}
        onClose={() => setRuleDrawerOpen(false)}
        width={420}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setRuleDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={savingRule} onClick={handleSaveRule}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={ruleForm} layout="vertical">
          <Form.Item
            name="after_sales_type"
            label="售后类型"
            rules={[{ required: true, message: '请选择售后类型' }]}
          >
            <Select options={afterSalesTypeOptions} />
          </Form.Item>
          <Form.Item
            name="result_code"
            label="质检结论"
            rules={[{ required: true, message: '请选择质检结论' }]}
          >
            <Select options={qualityResultOptions} />
          </Form.Item>
          <Form.Item name="sort_order" label="排序">
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态" initialValue="ACTIVE">
            <Select options={statusOptions} />
          </Form.Item>
        </Form>
      </Drawer>
    </>
  );
};
