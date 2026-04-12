import {
  Button,
  Card,
  Col,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Switch,
  Tabs,
  Tag,
  Typography,
  message
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import type { Key } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import {
  generateSkuSerials,
  listSkuSerials,
  printSkuSerials,
  type SkuSerial,
  type SkuSerialGenerateRequest,
  type SkuSerialPrintRequest
} from '@/api/skuSerials';
import {
  createSkuSerialRule,
  deleteSkuSerialRule,
  listSkuSerialRules,
  updateSkuSerialRule,
  type SkuSerialRule,
  type SkuSerialRulePayload
} from '@/api/skuSerialRules';
import { searchSkus, type SkuSearchItem } from '@/api/skus';
import {
  listPrintTemplates,
  type PrintTemplate
} from '@/api/printTemplates';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FilterValues = {
  sku_id?: string;
  status?: string;
  serial_no?: string;
};

type GenerateFormValues = {
  sku_id?: string;
  prefix?: string;
  rule_id?: string;
  count: number;
};

type RuleFilterValues = {
  sku_id?: string;
  enabled?: boolean;
};

type RuleFormValues = {
  sku_id?: string;
  name: string;
  prefix: string;
  seq_length?: number;
  increment_step?: number;
  enabled?: boolean;
};

type PrintFormValues = {
  template_id?: string;
  device_fingerprint?: string;
};

const statusOptions = [
  { value: 'IN_STOCK', label: 'IN_STOCK' },
  { value: 'ALLOCATED', label: 'ALLOCATED' },
  { value: 'SHIPPED', label: 'SHIPPED' },
  { value: 'RETURNED', label: 'RETURNED' },
  { value: 'SCRAPPED', label: 'SCRAPPED' }
];

const statusColors: Record<string, string> = {
  IN_STOCK: 'green',
  ALLOCATED: 'blue',
  SHIPPED: 'purple',
  RETURNED: 'orange',
  SCRAPPED: 'red'
};

const formatDateTime = (value?: string | null) =>
  value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-';

const buildSkuLabel = (sku: SkuSearchItem) => {
  const name = sku.name ? ` ${sku.name}` : '';
  const barcode = sku.barcode ? ` (${sku.barcode})` : '';
  return `${sku.sku_code}${name}${barcode}`;
};

const SkuSerialRulesPanel = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<RuleFilterValues>();
  const [drawerForm] = Form.useForm<RuleFormValues>();
  const [filters, setFilters] = useState<RuleFilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<SkuSerialRule | null>(null);
  const [saving, setSaving] = useState(false);
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuLoading, setSkuLoading] = useState(false);

  const query = useMemo(
    () => ({
      sku_id: filters.sku_id,
      enabled: filters.enabled,
      offset: ((pagination.current ?? 1) - 1) * (pagination.pageSize ?? 20),
      limit: pagination.pageSize ?? 20
    }),
    [filters.enabled, filters.sku_id, pagination.current, pagination.pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sku-serial-rules', tenantId, query],
    queryFn: () => listSkuSerialRules(tenantId, query),
    enabled: Boolean(tenantId)
  });

  const loadSkuOptions = async (keyword?: string) => {
    setSkuLoading(true);
    try {
      const response = await searchSkus(tenantId, keyword ?? '');
      setSkuOptions(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'SKU 搜索失败');
    } finally {
      setSkuLoading(false);
    }
  };

  const columns = useMemo<ColumnsType<SkuSerialRule>>(
    () => [
      {
        title: '规则名称',
        dataIndex: 'name'
      },
      {
        title: 'SKU',
        dataIndex: 'sku_id',
        render: (value) => value ?? '通用'
      },
      {
        title: '前缀',
        dataIndex: 'prefix'
      },
      {
        title: '序列长度',
        dataIndex: 'seq_length'
      },
      {
        title: '递增步长',
        dataIndex: 'increment_step'
      },
      {
        title: '启用',
        dataIndex: 'enabled',
        render: (value) => (value ? '启用' : '停用')
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
                setEditingRule(record);
                setDrawerOpen(true);
                drawerForm.setFieldsValue({
                  sku_id: record.sku_id ?? undefined,
                  name: record.name,
                  prefix: record.prefix,
                  seq_length: record.seq_length,
                  increment_step: record.increment_step,
                  enabled: record.enabled
                });
              }}
            >
              编辑
            </Button>
            <Button
              size="small"
              danger
              disabled={!canWrite}
              onClick={async () => {
                try {
                  await deleteSkuSerialRule(tenantId, record.rule_id);
                  message.success('规则已删除');
                  await queryClient.invalidateQueries({ queryKey: ['sku-serial-rules', tenantId] });
                } catch (err) {
                  message.error(err instanceof Error ? err.message : '删除失败');
                }
              }}
            >
              删除
            </Button>
          </Space>
        )
      }
    ],
    [canWrite, drawerForm, queryClient, tenantId]
  );

  const handleSearch = (values: RuleFilterValues) => {
    setFilters(values);
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openCreateDrawer = () => {
    setEditingRule(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({ enabled: true, seq_length: 6, increment_step: 1 });
    void loadSkuOptions();
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const seqLength = Number(values.seq_length ?? 6);
      const incrementStep = Number(values.increment_step ?? 1);
      const payload: SkuSerialRulePayload = {
        sku_id: values.sku_id ?? null,
        name: values.name.trim(),
        prefix: values.prefix.trim(),
        seq_length: Number.isNaN(seqLength) ? 6 : seqLength,
        increment_step: Number.isNaN(incrementStep) ? 1 : incrementStep,
        enabled: values.enabled ?? true
      };
      setSaving(true);
      if (editingRule) {
        await updateSkuSerialRule(tenantId, editingRule.rule_id, payload);
        message.success('规则已更新');
      } else {
        await createSkuSerialRule(tenantId, payload);
        message.success('规则已创建');
      }
      setDrawerOpen(false);
      setEditingRule(null);
      await queryClient.invalidateQueries({ queryKey: ['sku-serial-rules', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={4} style={{ margin: 0 }}>
          序列号规则
        </Typography.Title>
        <Button type="primary" onClick={openCreateDrawer} disabled={!canWrite}>
          新建规则
        </Button>
      </Space>
      <Card>
        <Form form={form} layout="vertical" onFinish={handleSearch}>
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="sku_id" label="SKU">
                <Select
                  showSearch
                  allowClear
                  placeholder="Search SKU"
                  options={skuOptions.map((sku) => ({
                    value: sku.sku_id,
                    label: buildSkuLabel(sku)
                  }))}
                  filterOption={false}
                  loading={skuLoading}
                  onSearch={(value) => loadSkuOptions(value)}
                  onFocus={() => loadSkuOptions()}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="enabled" label="状态">
                <Select
                  allowClear
                  options={[
                    { value: true, label: '启用' },
                    { value: false, label: '停用' }
                  ]}
                />
              </Form.Item>
            </Col>
            <Col xs={24} md={6} style={{ display: 'flex', alignItems: 'end' }}>
              <Space>
                <Button type="primary" htmlType="submit">
                  Search
                </Button>
                <Button onClick={handleReset}>Reset</Button>
              </Space>
            </Col>
          </Row>
        </Form>
      </Card>
      <UiModule03MainTableSection<SkuSerialRule>
        rowKey="rule_id"
        columns={columns}
        data={data?.items ?? []}
        loading={isLoading || isFetching}
        error={error}
        onRetry={() => refetch()}
        errorTitle="序列号规则加载失败"
        emptyText="暂无规则"
        total={data?.total ?? 0}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="sku-serial-rules.table.columns"
        enablePanels={false}
      />
      <Drawer
        title={editingRule ? '编辑规则' : '新建规则'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setDrawerOpen(false)}>Cancel</Button>
            <Button type="primary" loading={saving} onClick={handleSave}>
              Save
            </Button>
          </Space>
        }
      >
        <Form form={drawerForm} layout="vertical" initialValues={{ enabled: true }}>
          <Form.Item name="name" label="规则名称" rules={[{ required: true, message: '请输入名称' }]}>
            <Input placeholder="规则名称" />
          </Form.Item>
          <Form.Item name="sku_id" label="SKU">
            <Select
              showSearch
              allowClear
              placeholder="Search SKU"
              options={skuOptions.map((sku) => ({
                value: sku.sku_id,
                label: buildSkuLabel(sku)
              }))}
              filterOption={false}
              loading={skuLoading}
              onSearch={(value) => loadSkuOptions(value)}
              onFocus={() => loadSkuOptions()}
            />
          </Form.Item>
          <Form.Item
            name="prefix"
            label="前缀"
            rules={[{ required: true, message: '请输入前缀' }]}
          >
            <Input placeholder="SN前缀" />
          </Form.Item>
          <Form.Item name="seq_length" label="序列长度">
            <Input type="number" min={1} />
          </Form.Item>
          <Form.Item name="increment_step" label="递增步长">
            <Input type="number" min={1} />
          </Form.Item>
          <Form.Item name="enabled" label="启用" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Drawer>
    </Space>
  );
};

export const SkuSerialsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const canPrint = useCan('print_jobs.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [generateForm] = Form.useForm<GenerateFormValues>();
  const [printForm] = Form.useForm<PrintFormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20
  });
  const [cursorMap, setCursorMap] = useState<Record<number, string | undefined>>({});
  const [generateOpen, setGenerateOpen] = useState(false);
  const [printOpen, setPrintOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuLoading, setSkuLoading] = useState(false);
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);
  const [templateOptions, setTemplateOptions] = useState<PrintTemplate[]>([]);
  const [templateLoading, setTemplateLoading] = useState(false);
  const [ruleOptions, setRuleOptions] = useState<SkuSerialRule[]>([]);

  const currentPage = pagination.current ?? 1;
  const pageSize = pagination.pageSize ?? 20;
  const cursor = cursorMap[currentPage];

  const query = useMemo(
    () => ({
      sku_id: filters.sku_id,
      status: filters.status,
      serial_no: filters.serial_no?.trim() || undefined,
      cursor,
      limit: pageSize
    }),
    [cursor, filters.serial_no, filters.sku_id, filters.status, pageSize]
  );

  const { data, isLoading, isFetching, error, refetch } = useQuery({
    queryKey: ['sku-serials', tenantId, query],
    queryFn: () => listSkuSerials(tenantId, query),
    keepPreviousData: true,
    enabled: Boolean(tenantId)
  });

  useEffect(() => {
    if (!data?.next_cursor) {
      return;
    }
    const nextPage = currentPage + 1;
    setCursorMap((prev) => {
      if (prev[nextPage] === data.next_cursor) {
        return prev;
      }
      return { ...prev, [nextPage]: data.next_cursor ?? undefined };
    });
  }, [currentPage, data?.next_cursor]);

  const total = useMemo(() => {
    if (!data) {
      return 0;
    }
    if (data.next_cursor) {
      return currentPage * pageSize + pageSize;
    }
    return (currentPage - 1) * pageSize + data.data.length;
  }, [currentPage, data, pageSize]);

  const skuMap = useMemo(() => {
    const map = new Map<string, SkuSearchItem>();
    skuOptions.forEach((item) => map.set(item.sku_id, item));
    data?.data.forEach((serial) => {
      if (!map.has(serial.sku_id)) {
        map.set(serial.sku_id, {
          sku_id: serial.sku_id,
          sku_code: serial.sku_id,
          name: null,
          barcode: null,
          sku_kind: null
        });
      }
    });
    return map;
  }, [data?.data, skuOptions]);

  const loadSkuOptions = async (keyword?: string) => {
    setSkuLoading(true);
    try {
      const response = await searchSkus(tenantId, keyword ?? '');
      setSkuOptions(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : 'SKU 搜索失败');
    } finally {
      setSkuLoading(false);
    }
  };

  const loadRuleOptions = async () => {
    try {
      const response = await listSkuSerialRules(tenantId, { offset: 0, limit: 200, enabled: true });
      setRuleOptions(response.items);
    } catch (err) {
      message.error(err instanceof Error ? err.message : '序列号规则加载失败');
    }
  };

  const loadTemplateOptions = async () => {
    setTemplateLoading(true);
    try {
      const response = await listPrintTemplates(tenantId, { limit: 50, offset: 0 });
      setTemplateOptions(response.items ?? []);
      if (!printForm.getFieldValue('template_id') && response.items?.length) {
        printForm.setFieldsValue({ template_id: response.items[0].template_id });
      }
    } catch (err) {
      message.error(err instanceof Error ? err.message : '打印模板加载失败');
    } finally {
      setTemplateLoading(false);
    }
  };

  const columns = useMemo<ColumnsType<SkuSerial>>(
    () => [
      {
        title: 'Serial No',
        dataIndex: 'serial_no'
      },
      {
        title: 'SKU',
        dataIndex: 'sku_id',
        render: (_, record) => {
          const sku = skuMap.get(record.sku_id);
          return sku ? buildSkuLabel(sku) : record.sku_id;
        }
      },
      {
        title: 'Status',
        dataIndex: 'status',
        render: (_, record) => (
          <Tag color={statusColors[record.status] ?? 'default'}>{record.status}</Tag>
        )
      },
      {
        title: 'Ref Doc',
        dataIndex: 'ref_doc_id',
        render: (_, record) => record.ref_doc_id ?? '-'
      },
      {
        title: 'Updated At',
        dataIndex: 'updated_at',
        render: (_, record) => formatDateTime(record.updated_at)
      }
    ],
    [skuMap]
  );

  const handleSearch = (values: FilterValues) => {
    setFilters(values);
    setCursorMap({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const handleReset = () => {
    form.resetFields();
    setFilters({});
    setCursorMap({});
    setPagination((prev) => ({ ...prev, current: 1 }));
  };

  const openGenerateDrawer = () => {
    setGenerateOpen(true);
    generateForm.resetFields();
    generateForm.setFieldsValue({ count: 10 });
    void loadSkuOptions();
    void loadRuleOptions();
  };

  const handleGenerate = async () => {
    try {
      const values = await generateForm.validateFields();
      if (!values.sku_id) {
        message.error('请选择 SKU');
        return;
      }
      if (!values.prefix?.trim() && !values.rule_id) {
        message.error('请输入前缀或选择规则');
        return;
      }
      const payload: SkuSerialGenerateRequest = {
        sku_id: values.sku_id,
        prefix: values.prefix?.trim() || undefined,
        rule_id: values.rule_id,
        count: values.count
      };
      setSaving(true);
      await generateSkuSerials(tenantId, payload);
      message.success('唯一码已生成');
      setGenerateOpen(false);
      await queryClient.invalidateQueries({ queryKey: ['sku-serials', tenantId] });
    } catch (err) {
      if (err instanceof Error && err.message) {
        message.error(err.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const openPrintModal = () => {
    setPrintOpen(true);
    printForm.resetFields();
    void loadTemplateOptions();
  };

  const handlePrint = async () => {
    try {
      const values = await printForm.validateFields();
      const serialIds = selectedRowKeys.map((key) => String(key));
      if (!serialIds.length) {
        message.error('请选择要打印的唯一码');
        return;
      }
      if (!values.template_id) {
        message.error('请选择打印模板');
        return;
      }
      const payload: SkuSerialPrintRequest = {
        template_id: values.template_id,
        serial_ids: serialIds,
        device_fingerprint: values.device_fingerprint?.trim() || undefined
      };
      setSaving(true);
      await printSkuSerials(tenantId, payload);
      message.success('打印任务已创建');
      setPrintOpen(false);
      setSelectedRowKeys([]);
      await queryClient.invalidateQueries({ queryKey: ['print-jobs', tenantId] });
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
      <Tabs
      defaultActiveKey="serials"
      items={[
        {
          key: 'serials',
          label: '序列号列表',
          children: (
            <Space direction="vertical" size={16} style={{ width: '100%' }}>
              <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
                <Typography.Title level={3} style={{ margin: 0 }}>
                  SKU Serials
                </Typography.Title>
                <Space>
                  <Button onClick={openGenerateDrawer} disabled={!canWrite}>
                    Generate Serials
                  </Button>
                  <Button
                    type="primary"
                    onClick={openPrintModal}
                    disabled={!canPrint || selectedRowKeys.length === 0}
                  >
                    Print Selected
                  </Button>
                </Space>
              </Space>
              <Card>
                <Form form={form} layout="vertical" onFinish={handleSearch}>
                  <Row gutter={16}>
                    <Col xs={24} md={8}>
                      <Form.Item name="sku_id" label="SKU">
                        <Select
                          showSearch
                          allowClear
                          placeholder="Search SKU"
                          options={skuOptions.map((sku) => ({
                            value: sku.sku_id,
                            label: buildSkuLabel(sku)
                          }))}
                          filterOption={false}
                          loading={skuLoading}
                          onSearch={(value) => loadSkuOptions(value)}
                          onFocus={() => loadSkuOptions()}
                        />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={6}>
                      <Form.Item name="status" label="Status">
                        <Select allowClear options={statusOptions} placeholder="All" />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={6}>
                      <Form.Item name="serial_no" label="Serial No">
                        <Input allowClear placeholder="Serial number" />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
                      <Space>
                        <Button type="primary" htmlType="submit">
                          Search
                        </Button>
                        <Button onClick={handleReset}>Reset</Button>
                      </Space>
                    </Col>
                  </Row>
                </Form>
              </Card>
              <UiModule03MainTableSection<SkuSerial>
                rowKey="serial_id"
                columns={columns}
                data={data?.data ?? []}
                loading={isLoading || isFetching}
                error={error}
                onRetry={() => refetch()}
                errorTitle="唯一码加载失败"
                emptyText="暂无唯一码"
                total={total}
                pagination={pagination}
                onPaginationChange={(next) => {
                  setPagination(next);
                  setSelectedRowKeys([]);
                }}
                columnSettingsKey="sku-serials.table.columns"
                rowSelection={{
                  selectedRowKeys,
                  onChange: (keys) => setSelectedRowKeys(keys)
                }}
                enablePanels={false}
              />
            </Space>
          )
        },
        {
          key: 'rules',
          label: '序列号规则',
          children: <SkuSerialRulesPanel />
        }
      ]}
    />
      <Drawer
        title="Generate Serials"
        open={generateOpen}
        onClose={() => setGenerateOpen(false)}
        width={520}
        destroyOnClose
        footer={
          <Space>
            <Button onClick={() => setGenerateOpen(false)}>Cancel</Button>
            <Button type="primary" loading={saving} onClick={handleGenerate}>
              Generate
            </Button>
          </Space>
        }
      >
        <Form form={generateForm} layout="vertical" initialValues={{ count: 10 }}>
          <Form.Item
            name="sku_id"
            label="SKU"
            rules={[{ required: true, message: '请选择 SKU' }]}
          >
            <Select
              showSearch
              allowClear
              placeholder="Search SKU"
              options={skuOptions.map((sku) => ({
                value: sku.sku_id,
                label: buildSkuLabel(sku)
              }))}
              filterOption={false}
              loading={skuLoading}
              onSearch={(value) => loadSkuOptions(value)}
              onFocus={() => loadSkuOptions()}
            />
          </Form.Item>
          <Form.Item name="rule_id" label="规则">
            <Select
              allowClear
              placeholder="选择规则"
              options={ruleOptions.map((rule) => ({
                value: rule.rule_id,
                label: `${rule.name} (${rule.prefix})`
              }))}
            />
          </Form.Item>
          <Form.Item name="prefix" label="Prefix">
            <Input placeholder="SN prefix" />
          </Form.Item>
          <Form.Item
            name="count"
            label="Count"
            rules={[{ required: true, message: '输入数量' }]}
          >
            <InputNumber min={1} precision={0} style={{ width: '100%' }} />
          </Form.Item>
        </Form>
      </Drawer>
      <Modal
        title="Print Serials"
        open={printOpen}
        onCancel={() => setPrintOpen(false)}
        onOk={handlePrint}
        confirmLoading={saving}
        okText="Print"
      >
        <Form form={printForm} layout="vertical">
          <Form.Item
            name="template_id"
            label="Template"
            rules={[{ required: true, message: '选择模板' }]}
          >
            <Select
              showSearch
              allowClear
              placeholder="Select template"
              loading={templateLoading}
              options={templateOptions.map((template) => ({
                value: template.template_id,
                label: `${template.template_type} / ${template.template_id}`
              }))}
            />
          </Form.Item>
          <Form.Item name="device_fingerprint" label="Device Fingerprint">
            <Input placeholder="Optional device fingerprint" />
          </Form.Item>
          <Typography.Text type="secondary">
            已选择 {selectedRowKeys.length} 个唯一码
          </Typography.Text>
        </Form>
      </Modal>
    </>
  );
};
