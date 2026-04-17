import {
  Button,
  Card,
  DatePicker,
  Drawer,
  Form,
  Input,
  InputNumber,
  Popconfirm,
  Select,
  Space,
  Tabs,
  Tag,
  Typography,
  message
} from 'antd';
import type { ColumnsType, TablePaginationConfig } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type { InvoiceAdjustment, TaxRule, TaxRuleCreatePayload } from '@/api/finance';
import {
  createInvoiceAdjustment,
  createTaxRule,
  deleteTaxRule,
  listInvoiceAdjustments,
  listTaxRules,
  markInvoiceAdjustmentApplied,
  updateTaxRule
} from '@/api/finance';
import { UiModule03MainTableSection, UiModule07SettingsLayout } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type TaxFilterValues = {
  tax_code?: string;
  status?: string;
};

type TaxFormValues = TaxRuleCreatePayload;

type AdjustmentFilterValues = {
  status?: string;
  invoice_id?: string;
};

type AdjustmentFormValues = {
  invoice_id: string;
  adjustment_type: string;
  reason?: string;
  status?: string;
};

const statusOptions = [
  { label: '启用', value: 'ACTIVE' },
  { label: '停用', value: 'DISABLED' }
];

const adjustmentStatusOptions = [
  { label: '待处理', value: 'PENDING' },
  { label: '已处理', value: 'APPLIED' }
];

const formatDate = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD') : '-');

export const TaxManagementPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('finance.write');
  const queryClient = useQueryClient();
  const [taxFilters, setTaxFilters] = useState<TaxFilterValues>({});
  const [taxPagination, setTaxPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [adjustFilters, setAdjustFilters] = useState<AdjustmentFilterValues>({});
  const [adjustPagination, setAdjustPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [taxForm] = Form.useForm<TaxFilterValues>();
  const [adjustForm] = Form.useForm<AdjustmentFilterValues>();
  const [taxDrawerForm] = Form.useForm<TaxFormValues>();
  const [adjustDrawerForm] = Form.useForm<AdjustmentFormValues>();
  const [taxDrawerOpen, setTaxDrawerOpen] = useState(false);
  const [adjustDrawerOpen, setAdjustDrawerOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<TaxRule | null>(null);

  const taxQuery = useMemo(
    () => ({
      tax_code: taxFilters.tax_code?.trim() || undefined,
      status: taxFilters.status || undefined,
      offset: ((taxPagination.current ?? 1) - 1) * (taxPagination.pageSize ?? 20),
      limit: taxPagination.pageSize ?? 20
    }),
    [taxFilters, taxPagination.current, taxPagination.pageSize]
  );

  const adjustQuery = useMemo(
    () => ({
      status: adjustFilters.status || undefined,
      invoice_id: adjustFilters.invoice_id?.trim() || undefined,
      offset: ((adjustPagination.current ?? 1) - 1) * (adjustPagination.pageSize ?? 20),
      limit: adjustPagination.pageSize ?? 20
    }),
    [adjustFilters, adjustPagination.current, adjustPagination.pageSize]
  );

  const taxQueryResult = useQuery({
    queryKey: ['tax-rules', tenantId, taxQuery],
    queryFn: () => listTaxRules(tenantId, taxQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const adjustQueryResult = useQuery({
    queryKey: ['invoice-adjustments', tenantId, adjustQuery],
    queryFn: () => listInvoiceAdjustments(tenantId, adjustQuery),
    enabled: tenantReady,
    keepPreviousData: true
  });

  const saveTaxMutation = useMutation({
    mutationFn: async () => {
      const values = await taxDrawerForm.validateFields();
      const payload: TaxRuleCreatePayload = {
        tax_code: values.tax_code.trim(),
        tax_rate: values.tax_rate,
        effective_from: values.effective_from,
        effective_to: values.effective_to,
        status: values.status
      };
      if (editingRule) {
        return updateTaxRule(tenantId, editingRule.rule_id, payload);
      }
      return createTaxRule(tenantId, payload);
    },
    onSuccess: async () => {
      message.success(editingRule ? '税率已更新' : '税率已创建');
      setTaxDrawerOpen(false);
      setEditingRule(null);
      await queryClient.invalidateQueries({ queryKey: ['tax-rules', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '保存税率失败');
    }
  });

  const deleteTaxMutation = useMutation({
    mutationFn: (ruleId: string) => deleteTaxRule(tenantId, ruleId),
    onSuccess: async () => {
      message.success('税率已删除');
      await queryClient.invalidateQueries({ queryKey: ['tax-rules', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '删除税率失败');
    }
  });

  const saveAdjustmentMutation = useMutation({
    mutationFn: async () => {
      const values = await adjustDrawerForm.validateFields();
      return createInvoiceAdjustment(tenantId, {
        invoice_id: values.invoice_id.trim(),
        adjustment_type: values.adjustment_type.trim(),
        reason: values.reason?.trim(),
        status: values.status
      });
    },
    onSuccess: async () => {
      message.success('发票调整已创建');
      setAdjustDrawerOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['invoice-adjustments', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '创建发票调整失败');
    }
  });

  const applyAdjustmentMutation = useMutation({
    mutationFn: (adjustmentId: string) => markInvoiceAdjustmentApplied(tenantId, adjustmentId),
    onSuccess: async () => {
      message.success('发票调整已处理');
      await queryClient.invalidateQueries({ queryKey: ['invoice-adjustments', tenantId] });
    },
    onError: (err) => {
      message.error(err instanceof Error ? err.message : '发票调整失败');
    }
  });

  const handleTaxSearch = (values: TaxFilterValues) => {
    setTaxFilters(values);
    setTaxPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleAdjustSearch = (values: AdjustmentFilterValues) => {
    setAdjustFilters(values);
    setAdjustPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleTaxReset = () => {
    taxForm.resetFields();
    setTaxFilters({});
    setTaxPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleAdjustReset = () => {
    adjustForm.resetFields();
    setAdjustFilters({});
    setAdjustPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openTaxCreate = () => {
    setEditingRule(null);
    taxDrawerForm.resetFields();
    taxDrawerForm.setFieldsValue({
      status: 'ACTIVE'
    });
    setTaxDrawerOpen(true);
  };

  const openTaxEdit = (record: TaxRule) => {
    setEditingRule(record);
    taxDrawerForm.setFieldsValue({
      tax_code: record.tax_code,
      tax_rate: record.tax_rate,
      effective_from: record.effective_from ?? undefined,
      effective_to: record.effective_to ?? undefined,
      status: record.status
    });
    setTaxDrawerOpen(true);
  };

  const openAdjustCreate = () => {
    adjustDrawerForm.resetFields();
    adjustDrawerForm.setFieldsValue({ status: 'PENDING' });
    setAdjustDrawerOpen(true);
  };

  const taxColumns = useMemo<ColumnsType<TaxRule>>(
    () => [
      { title: '税码', dataIndex: 'tax_code' },
      { title: '税率(%)', dataIndex: 'tax_rate' },
      { title: '生效', dataIndex: 'effective_from', render: formatDate },
      { title: '失效', dataIndex: 'effective_to', render: formatDate },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) => (value === 'ACTIVE' ? <Tag color="green">启用</Tag> : <Tag>停用</Tag>)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => openTaxEdit(record)}>
              编辑
            </Button>
            <Popconfirm
              title="确认删除税率？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => deleteTaxMutation.mutate(record.rule_id)}
              disabled={!canWrite}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        )
      }
    ],
    [canWrite, deleteTaxMutation]
  );

  const adjustmentColumns = useMemo<ColumnsType<InvoiceAdjustment>>(
    () => [
      { title: '发票ID', dataIndex: 'invoice_id' },
      { title: '类型', dataIndex: 'adjustment_type' },
      { title: '原因', dataIndex: 'reason', render: (value) => value ?? '-' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value) => (value === 'APPLIED' ? <Tag color="green">已处理</Tag> : <Tag>待处理</Tag>)
      },
      { title: '处理时间', dataIndex: 'applied_at', render: formatDate },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Popconfirm
            title="确认处理调整？"
            okText="确认"
            cancelText="取消"
            onConfirm={() => applyAdjustmentMutation.mutate(record.adjustment_id)}
            disabled={!canWrite || record.status === 'APPLIED'}
          >
            <Button size="small" disabled={!canWrite || record.status === 'APPLIED'}>
              标记完成
            </Button>
          </Popconfirm>
        )
      }
    ],
    [applyAdjustmentMutation, canWrite]
  );

  return (
    <UiModule07SettingsLayout>
      <Typography.Title level={3} style={{ margin: 0 }}>
        税务与发票管理
      </Typography.Title>
      <Tabs
        items={[
          {
            key: 'tax',
            label: '税率规则',
            children: (
              <Space direction="vertical" size={16} style={{ width: '100%' }}>
                <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
                  <Card style={{ flex: 1 }}>
                    <Form form={taxForm} layout="inline" onFinish={handleTaxSearch}>
                      <Form.Item name="tax_code" label="税码">
                        <Input allowClear placeholder="税码" style={{ width: 160 }} />
                      </Form.Item>
                      <Form.Item name="status" label="状态">
                        <Select allowClear options={statusOptions} placeholder="全部" style={{ width: 140 }} />
                      </Form.Item>
                      <Form.Item>
                        <Space>
                          <Button type="primary" htmlType="submit">
                            查询
                          </Button>
                          <Button onClick={handleTaxReset}>重置</Button>
                        </Space>
                      </Form.Item>
                    </Form>
                  </Card>
                  <Button type="primary" disabled={!canWrite} onClick={openTaxCreate}>
                    新建税率
                  </Button>
                </Space>
                <UiModule03MainTableSection<TaxRule>
                  rowKey="rule_id"
                  columns={taxColumns}
                  data={taxQueryResult.data?.items ?? []}
                  loading={taxQueryResult.isFetching}
                  total={taxQueryResult.data?.total ?? 0}
                  pagination={taxPagination}
                  onPaginationChange={setTaxPagination}
                  error={taxQueryResult.error}
                  onRetry={() => taxQueryResult.refetch()}
                  errorTitle="税率规则加载失败"
                  emptyText="暂无税率规则"
                  columnSettingsKey="finance.tax.table.columns"
                />
              </Space>
            )
          },
          {
            key: 'adjustments',
            label: '发票调整',
            children: (
              <Space direction="vertical" size={16} style={{ width: '100%' }}>
                <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
                  <Card style={{ flex: 1 }}>
                    <Form form={adjustForm} layout="inline" onFinish={handleAdjustSearch}>
                      <Form.Item name="invoice_id" label="发票ID">
                        <Input allowClear placeholder="发票ID" style={{ width: 200 }} />
                      </Form.Item>
                      <Form.Item name="status" label="状态">
                        <Select allowClear options={adjustmentStatusOptions} placeholder="全部" style={{ width: 140 }} />
                      </Form.Item>
                      <Form.Item>
                        <Space>
                          <Button type="primary" htmlType="submit">
                            查询
                          </Button>
                          <Button onClick={handleAdjustReset}>重置</Button>
                        </Space>
                      </Form.Item>
                    </Form>
                  </Card>
                  <Button type="primary" disabled={!canWrite} onClick={openAdjustCreate}>
                    新建调整
                  </Button>
                </Space>
                <UiModule03MainTableSection<InvoiceAdjustment>
                  rowKey="adjustment_id"
                  columns={adjustmentColumns}
                  data={adjustQueryResult.data?.items ?? []}
                  loading={adjustQueryResult.isFetching}
                  total={adjustQueryResult.data?.total ?? 0}
                  pagination={adjustPagination}
                  onPaginationChange={setAdjustPagination}
                  error={adjustQueryResult.error}
                  onRetry={() => adjustQueryResult.refetch()}
                  errorTitle="发票调整加载失败"
                  emptyText="暂无调整记录"
                  columnSettingsKey="finance.invoice.adjustments.table.columns"
                />
              </Space>
            )
          }
        ]}
      />
      <Drawer
        title={editingRule ? '编辑税率' : '新建税率'}
        open={taxDrawerOpen}
        onClose={() => setTaxDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setTaxDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saveTaxMutation.isPending} onClick={() => saveTaxMutation.mutate()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={taxDrawerForm} layout="vertical">
          <Form.Item name="tax_code" label="税码" rules={[{ required: true, message: '请输入税码' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="tax_rate" label="税率(%)" rules={[{ required: true, message: '请输入税率' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="effective_from" label="生效日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="effective_to" label="失效日期">
            <DatePicker style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={statusOptions} />
          </Form.Item>
        </Form>
      </Drawer>
      <Drawer
        title="新建发票调整"
        open={adjustDrawerOpen}
        onClose={() => setAdjustDrawerOpen(false)}
        width={520}
        destroyOnHidden
        footer={
          <Space>
            <Button onClick={() => setAdjustDrawerOpen(false)}>取消</Button>
            <Button type="primary" loading={saveAdjustmentMutation.isPending} onClick={() => saveAdjustmentMutation.mutate()}>
              保存
            </Button>
          </Space>
        }
      >
        <Form form={adjustDrawerForm} layout="vertical">
          <Form.Item name="invoice_id" label="发票ID" rules={[{ required: true, message: '请输入发票ID' }]}>
            <Input />
          </Form.Item>
          <Form.Item name="adjustment_type" label="调整类型" rules={[{ required: true, message: '请输入类型' }]}>
            <Input placeholder="RED/REISSUE" />
          </Form.Item>
          <Form.Item name="status" label="状态">
            <Select options={adjustmentStatusOptions} />
          </Form.Item>
          <Form.Item name="reason" label="原因">
            <Input.TextArea rows={3} />
          </Form.Item>
        </Form>
      </Drawer>
    </UiModule07SettingsLayout>
  );
};
