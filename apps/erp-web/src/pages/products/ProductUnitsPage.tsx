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
  message,
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { ProductUnitConversionListQuery } from '@/api/productUnitConversions';
import {
  createProductUnitConversion,
  deleteProductUnitConversion,
  listProductUnitConversions,
  updateProductUnitConversion,
} from '@/api/productUnitConversions';
import { listSkus } from '@/api/skus';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type ConversionStatus = 'ACTIVE' | 'INACTIVE';

type UnitConversion = {
  id: string;
  sku_id: string;
  sku_code: string;
  sku_name: string;
  base_unit: string;
  aux_unit: string;
  ratio: number;
  rounding: string;
  scenario: string;
  status: ConversionStatus;
  updated_at: string;
};

type FilterValues = {
  keyword?: string;
  scenario?: string;
  status?: ConversionStatus;
  unit?: string;
};

type FormValues = {
  sku_id: string;
  name: string;
  base_unit: string;
  aux_unit: string;
  ratio: number;
  rounding: string;
  scenario: string;
  status: ConversionStatus;
};

type RoundingRules = {
  purchase: string;
  inbound: string;
  outbound: string;
  precision: number;
};

const roundingOptions = [
  { label: '四舍五入', value: 'ROUND' },
  { label: '向上取整', value: 'CEIL' },
  { label: '向下取整', value: 'FLOOR' },
];

const scenarioOptions = [
  { label: '采购', value: '采购' },
  { label: '仓储', value: '仓储' },
  { label: '销售', value: '销售' },
];

const statusLabels: Record<ConversionStatus, string> = {
  ACTIVE: '启用',
  INACTIVE: '停用',
};

const statusColors: Record<ConversionStatus, string> = {
  ACTIVE: 'green',
  INACTIVE: 'red',
};

const unitOptions = ['件', '盒', '箱', '袋', '套'].map((unit) => ({
  label: unit,
  value: unit,
}));

const defaultRules: RoundingRules = {
  purchase: 'ROUND',
  inbound: 'CEIL',
  outbound: 'FLOOR',
  precision: 0,
};

export const ProductUnitsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [ruleForm] = Form.useForm<RoundingRules>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingConversion, setEditingConversion] = useState<UnitConversion | null>(null);
  const [saving, setSaving] = useState(false);
  const [rules, setRules] = useState<RoundingRules>(defaultRules);

  const listQuery = useMemo<ProductUnitConversionListQuery>(
    () => ({
      scenario: filters.scenario,
      status: filters.status,
      offset: 0,
      limit: 200,
    }),
    [filters.scenario, filters.status],
  );

  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['product-unit-conversions', tenantId, listQuery],
    queryFn: () => listProductUnitConversions(tenantId, listQuery),
    enabled: Boolean(tenantId),
  });

  const skuQuery = useQuery({
    queryKey: ['skus', tenantId],
    queryFn: () => listSkus(tenantId, { limit: 200 }),
    enabled: Boolean(tenantId),
  });

  const skuMap = useMemo(
    () => new Map((skuQuery.data?.data ?? []).map((sku) => [sku.sku_id, sku])),
    [skuQuery.data?.data],
  );

  const skuOptions = useMemo(
    () =>
      (skuQuery.data?.data ?? []).map((sku) => ({
        value: sku.sku_id,
        label: `${sku.sku_code}${sku.name ? ` | ${sku.name}` : ''}`,
      })),
    [skuQuery.data?.data],
  );

  const records = useMemo<UnitConversion[]>(
    () =>
      (data?.items ?? []).map((item) => {
        const sku = skuMap.get(item.sku_id);
        return {
          id: item.conversion_id,
          sku_id: item.sku_id,
          sku_code: item.sku_code ?? sku?.sku_code ?? item.sku_id,
          sku_name: item.sku_name ?? sku?.name ?? '',
          base_unit: item.base_unit,
          aux_unit: item.aux_unit,
          ratio: item.ratio,
          rounding: item.rounding,
          scenario: item.scenario,
          status: item.status as ConversionStatus,
          updated_at: item.updated_at ?? item.created_at ?? '',
        };
      }),
    [data?.items, skuMap],
  );

  const filteredData = useMemo(() => {
    const keyword = filters.keyword?.trim();
    return records.filter((item) => {
      if (filters.scenario && item.scenario !== filters.scenario) {
        return false;
      }
      if (filters.status && item.status !== filters.status) {
        return false;
      }
      if (filters.unit && item.aux_unit !== filters.unit) {
        return false;
      }
      if (keyword) {
        const target = `${item.sku_code}${item.sku_name}`.toLowerCase();
        return target.includes(keyword.toLowerCase());
      }
      return true;
    });
  }, [filters.keyword, filters.scenario, filters.status, filters.unit, records]);

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
    setEditingConversion(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      sku_id: '',
      name: '',
      base_unit: '件',
      status: 'ACTIVE',
      rounding: 'ROUND',
      scenario: '采购',
      ratio: 1,
    });
  };

  const openEditDrawer = (record: UnitConversion) => {
    setEditingConversion(record);
    setDrawerOpen(true);
    drawerForm.setFieldsValue({
      sku_id: record.sku_id,
      name: record.sku_name,
      base_unit: record.base_unit,
      aux_unit: record.aux_unit,
      ratio: record.ratio,
      rounding: record.rounding,
      scenario: record.scenario,
      status: record.status,
    });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      if (!tenantId) {
        message.warning('请先选择租户');
        return;
      }
      setSaving(true);
      const payload = {
        base_unit: values.base_unit,
        aux_unit: values.aux_unit,
        ratio: Number(values.ratio ?? 1),
        rounding: values.rounding,
        scenario: values.scenario,
        status: values.status,
      };
      if (editingConversion) {
        await updateProductUnitConversion(tenantId, editingConversion.id, payload);
        message.success('换算规则已更新');
      } else {
        await createProductUnitConversion(tenantId, {
          sku_id: values.sku_id,
          ...payload,
        });
        message.success('换算规则已保存');
      }
      setDrawerOpen(false);
      setEditingConversion(null);
      await queryClient.invalidateQueries({ queryKey: ['product-unit-conversions', tenantId] });
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleToggleStatus = (record: UnitConversion) => {
    const nextStatus: ConversionStatus = record.status === 'ACTIVE' ? 'INACTIVE' : 'ACTIVE';
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    updateProductUnitConversion(tenantId, record.id, { status: nextStatus })
      .then(async () => {
        message.success(`已${nextStatus === 'ACTIVE' ? '启用' : '停用'}该换算规则`);
        await queryClient.invalidateQueries({ queryKey: ['product-unit-conversions', tenantId] });
      })
      .catch((error) => {
        message.error(error instanceof Error ? error.message : '更新换算状态失败');
      });
  };

  const handleDelete = async (record: UnitConversion) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    try {
      await deleteProductUnitConversion(tenantId, record.id);
      message.success('换算规则已删除');
      await queryClient.invalidateQueries({ queryKey: ['product-unit-conversions', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除换算规则失败');
    }
  };

  const handleSaveRules = async () => {
    const values = await ruleForm.validateFields();
    setRules(values);
    message.success('取整规则已更新');
  };

  const columns = useMemo<ColumnsType<UnitConversion>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_code' },
      { title: '商品名称', dataIndex: 'sku_name' },
      {
        title: '单位换算',
        render: (_, record) => `1${record.aux_unit} = ${record.ratio}${record.base_unit}`,
      },
      { title: '场景', dataIndex: 'scenario' },
      {
        title: '取整规则',
        dataIndex: 'rounding',
        render: (value: string) =>
          roundingOptions.find((item) => item.value === value)?.label ?? value,
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: ConversionStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        ),
      },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value: string) => dayjs(value).format('YYYY-MM-DD HH:mm'),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space>
            <Button size="small" disabled={!canWrite} onClick={() => openEditDrawer(record)}>
              编辑
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => handleToggleStatus(record)}>
              {record.status === 'ACTIVE' ? '停用' : '启用'}
            </Button>
            <Popconfirm
              title="确认删除该换算规则？"
              okText="删除"
              cancelText="取消"
              onConfirm={() => handleDelete(record)}
              disabled={!canWrite}
            >
              <Button size="small" danger disabled={!canWrite}>
                删除
              </Button>
            </Popconfirm>
          </Space>
        ),
      },
    ],
    [canWrite],
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Space align="center" style={{ width: '100%', justifyContent: 'space-between' }}>
        <Typography.Title level={3} style={{ margin: 0 }} data-testid="product-units-title">
          多单位与计量换算
        </Typography.Title>
        <Button
          type="primary"
          onClick={openCreateDrawer}
          disabled={!canWrite}
          data-testid="product-units-create-button"
        >
          新增换算
        </Button>
      </Space>
      <Card title="取整规则配置">
        <Form
          form={ruleForm}
          layout="inline"
          initialValues={rules}
          data-testid="product-units-rules-form"
        >
          <Form.Item
            name="purchase"
            label="采购"
            rules={[{ required: true, message: '请选择采购取整规则' }]}
          >
            <Select style={{ width: 160 }} options={roundingOptions} />
          </Form.Item>
          <Form.Item
            name="inbound"
            label="入库"
            rules={[{ required: true, message: '请选择入库取整规则' }]}
          >
            <Select style={{ width: 160 }} options={roundingOptions} />
          </Form.Item>
          <Form.Item
            name="outbound"
            label="出库"
            rules={[{ required: true, message: '请选择出库取整规则' }]}
          >
            <Select style={{ width: 160 }} options={roundingOptions} />
          </Form.Item>
          <Form.Item
            name="precision"
            label="精度"
            rules={[{ required: true, message: '请输入精度' }]}
          >
            <InputNumber min={0} max={4} precision={0} />
          </Form.Item>
          <Form.Item>
            <Button type="primary" onClick={handleSaveRules} disabled={!canWrite}>
              保存规则
            </Button>
          </Form.Item>
        </Form>
      </Card>
      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSearch}
          data-testid="product-units-filter-form"
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="SKU/商品名称" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="scenario" label="场景">
                <Select allowClear options={scenarioOptions} placeholder="全部场景" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="unit" label="辅助单位">
                <Select allowClear options={unitOptions} placeholder="全部单位" />
              </Form.Item>
            </Col>
            <Col xs={24} md={4} style={{ display: 'flex', alignItems: 'end' }}>
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
      <Card title="换算清单">
        <UiModule03MainTableSection<UnitConversion>
          dataTestId="product-units-table"
          rowKey="id"
          columns={columns}
          data={filteredData}
          loading={isLoading || isFetching}
          onRetry={() => refetch()}
          errorTitle="单位换算加载失败"
          emptyText="暂无换算规则"
          total={filteredData.length}
          pagination={pagination}
          onPaginationChange={setPagination}
          columnSettingsKey="product.units.table.columns"
          enablePanels={false}
        />
      </Card>
      <Drawer
        title={editingConversion ? '编辑换算' : '新增换算'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        data-testid="product-units-drawer"
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
          <Form.Item name="sku_id" label="SKU" rules={[{ required: true, message: '请选择SKU' }]}>
            <Select
              showSearch
              options={skuOptions}
              placeholder="选择SKU"
              loading={skuQuery.isLoading}
              disabled={Boolean(editingConversion)}
              filterOption={(input, option) =>
                (option?.label ?? '').toString().toLowerCase().includes(input.toLowerCase())
              }
              onChange={(value) => {
                const sku = skuMap.get(value);
                drawerForm.setFieldsValue({ name: sku?.name ?? '' });
              }}
            />
          </Form.Item>
          <Form.Item
            name="name"
            label="商品名称"
            rules={[{ required: true, message: '请输入商品名称' }]}
          >
            <Input placeholder="商品名称" disabled />
          </Form.Item>
          <Form.Item
            name="base_unit"
            label="基本单位"
            rules={[{ required: true, message: '请选择基本单位' }]}
          >
            <Select options={unitOptions} />
          </Form.Item>
          <Form.Item
            name="aux_unit"
            label="辅助单位"
            rules={[{ required: true, message: '请选择辅助单位' }]}
          >
            <Select options={unitOptions} />
          </Form.Item>
          <Form.Item
            name="ratio"
            label="换算比例"
            rules={[{ required: true, message: '请输入换算比例' }]}
          >
            <InputNumber min={1} precision={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="scenario"
            label="适用场景"
            rules={[{ required: true, message: '请选择适用场景' }]}
          >
            <Select options={scenarioOptions} />
          </Form.Item>
          <Form.Item
            name="rounding"
            label="取整规则"
            rules={[{ required: true, message: '请选择取整规则' }]}
          >
            <Select options={roundingOptions} />
          </Form.Item>
          <Form.Item name="status" label="状态" rules={[{ required: true }]}>
            <Select
              options={[
                { label: '启用', value: 'ACTIVE' },
                { label: '停用', value: 'INACTIVE' },
              ]}
            />
          </Form.Item>
        </Form>
      </Drawer>
    </Space>
  );
};
