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
  Switch,
  Table,
  Tag,
  Typography,
  message,
} from 'antd';
import type { TablePaginationConfig } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import type { ProductBarcodeListQuery } from '@/api/productBarcodes';
import {
  createProductBarcode,
  deleteProductBarcode,
  listProductBarcodes,
  updateProductBarcode,
} from '@/api/productBarcodes';
import { listSkus } from '@/api/skus';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type BarcodeStatus = 'ACTIVE' | 'INACTIVE';
type BarcodeLevel = '单品' | '内盒' | '外箱';

type BarcodeRecord = {
  id: string;
  sku_id: string;
  sku_code: string;
  sku_name: string;
  barcode: string;
  level: BarcodeLevel;
  unit: string;
  ratio: number;
  is_default: boolean;
  status: BarcodeStatus;
  print_template: string;
  updated_at: string;
};

type FilterValues = {
  keyword?: string;
  level?: BarcodeLevel;
  status?: BarcodeStatus;
};

type FormValues = {
  sku_id: string;
  name: string;
  barcode: string;
  level: BarcodeLevel;
  unit: string;
  ratio: number;
  is_default?: boolean;
  status: BarcodeStatus;
  print_template: string;
};

type PackagingSample = {
  level: BarcodeLevel;
  unit: string;
  ratio: number;
  barcode_rule: string;
  print_template: string;
};

const baseUnit = '件';

const levelOptions = [
  { label: '单品', value: '单品' },
  { label: '内盒', value: '内盒' },
  { label: '外箱', value: '外箱' },
];

const statusLabels: Record<BarcodeStatus, string> = {
  ACTIVE: '启用',
  INACTIVE: '停用',
};

const statusColors: Record<BarcodeStatus, string> = {
  ACTIVE: 'green',
  INACTIVE: 'red',
};

const templateOptions = [
  { label: '标准热敏', value: '标准热敏' },
  { label: '仓配联动模板', value: '仓配联动模板' },
  { label: '批次条码模板', value: '批次条码模板' },
];

const packagingSamples: PackagingSample[] = [
  {
    level: '单品',
    unit: '件',
    ratio: 1,
    barcode_rule: '一品多码可配置',
    print_template: '标准热敏',
  },
  {
    level: '内盒',
    unit: '盒',
    ratio: 6,
    barcode_rule: '批次条码校验',
    print_template: '仓配联动模板',
  },
  {
    level: '外箱',
    unit: '箱',
    ratio: 12,
    barcode_rule: '箱规唯一',
    print_template: '批次条码模板',
  },
];

export const ProductBarcodesPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FilterValues>();
  const [drawerForm] = Form.useForm<FormValues>();
  const [filters, setFilters] = useState<FilterValues>({});
  const [pagination, setPagination] = useState<TablePaginationConfig>({
    current: 1,
    pageSize: 20,
  });
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [editingBarcode, setEditingBarcode] = useState<BarcodeRecord | null>(null);
  const [saving, setSaving] = useState(false);

  const listQuery = useMemo<ProductBarcodeListQuery>(
    () => ({
      status: filters.status,
      level: filters.level,
      offset: 0,
      limit: 200,
    }),
    [filters.level, filters.status],
  );

  const { data, isLoading, isFetching, refetch } = useQuery({
    queryKey: ['product-barcodes', tenantId, listQuery],
    queryFn: () => listProductBarcodes(tenantId, listQuery),
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

  const records = useMemo<BarcodeRecord[]>(
    () =>
      (data?.items ?? []).map((item) => {
        const sku = skuMap.get(item.sku_id);
        return {
          id: item.barcode_id,
          sku_id: item.sku_id,
          sku_code: item.sku_code ?? sku?.sku_code ?? item.sku_id,
          sku_name: item.sku_name ?? sku?.name ?? '',
          barcode: item.barcode,
          level: item.level as BarcodeLevel,
          unit: item.unit,
          ratio: item.ratio,
          is_default: item.is_default,
          status: item.status as BarcodeStatus,
          print_template: item.print_template ?? '',
          updated_at: item.updated_at ?? item.created_at ?? '',
        };
      }),
    [data?.items, skuMap],
  );

  const filteredData = useMemo(() => {
    const keyword = filters.keyword?.trim();
    return records.filter((item) => {
      if (filters.level && item.level !== filters.level) {
        return false;
      }
      if (filters.status && item.status !== filters.status) {
        return false;
      }
      if (keyword) {
        const target = `${item.sku_code}${item.sku_name}${item.barcode}`.toLowerCase();
        return target.includes(keyword.toLowerCase());
      }
      return true;
    });
  }, [records, filters.keyword, filters.level, filters.status]);

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
    setEditingBarcode(null);
    setDrawerOpen(true);
    drawerForm.resetFields();
    drawerForm.setFieldsValue({
      sku_id: '',
      name: '',
      level: '单品',
      unit: '件',
      ratio: 1,
      status: 'ACTIVE',
      is_default: false,
      print_template: '标准热敏',
    });
  };

  const openEditDrawer = (record: BarcodeRecord) => {
    setEditingBarcode(record);
    setDrawerOpen(true);
    drawerForm.setFieldsValue({
      sku_id: record.sku_id,
      name: record.sku_name,
      barcode: record.barcode,
      level: record.level,
      unit: record.unit,
      ratio: record.ratio,
      is_default: record.is_default,
      status: record.status,
      print_template: record.print_template,
    });
  };

  const handleSave = async () => {
    try {
      const values = await drawerForm.validateFields();
      const trimmedBarcode = values.barcode.trim();
      if (
        records.some((item) => item.barcode === trimmedBarcode && item.id !== editingBarcode?.id)
      ) {
        message.error('条码已存在，请检查是否重复');
        return;
      }
      if (!tenantId) {
        message.warning('请先选择租户');
        return;
      }
      setSaving(true);
      const createPayload = {
        sku_id: values.sku_id,
        barcode: trimmedBarcode,
        level: values.level,
        unit: values.unit,
        ratio: Number(values.ratio ?? 1),
        is_default: Boolean(values.is_default),
        status: values.status,
        print_template: values.print_template,
      };
      if (editingBarcode) {
        await updateProductBarcode(tenantId, editingBarcode.id, {
          barcode: trimmedBarcode,
          level: values.level,
          unit: values.unit,
          ratio: Number(values.ratio ?? 1),
          is_default: values.is_default,
          status: values.status,
          print_template: values.print_template,
        });
        message.success('条码信息已更新');
      } else {
        await createProductBarcode(tenantId, createPayload);
        message.success('条码信息已保存');
      }
      setDrawerOpen(false);
      setEditingBarcode(null);
      await queryClient.invalidateQueries({ queryKey: ['product-barcodes', tenantId] });
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const handleSetDefault = async (record: BarcodeRecord) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    try {
      await updateProductBarcode(tenantId, record.id, { is_default: true });
      message.success('已设为默认条码');
      await queryClient.invalidateQueries({ queryKey: ['product-barcodes', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '设置默认条码失败');
    }
  };

  const handleToggleStatus = async (record: BarcodeRecord) => {
    const nextStatus: BarcodeStatus = record.status === 'ACTIVE' ? 'INACTIVE' : 'ACTIVE';
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    try {
      await updateProductBarcode(tenantId, record.id, { status: nextStatus });
      message.success(`条码已${nextStatus === 'ACTIVE' ? '启用' : '停用'}`);
      await queryClient.invalidateQueries({ queryKey: ['product-barcodes', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '更新条码状态失败');
    }
  };

  const handleDelete = async (record: BarcodeRecord) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    try {
      await deleteProductBarcode(tenantId, record.id);
      message.success('条码已删除');
      await queryClient.invalidateQueries({ queryKey: ['product-barcodes', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '删除条码失败');
    }
  };

  const columns = useMemo<ColumnsType<BarcodeRecord>>(
    () => [
      { title: 'SKU', dataIndex: 'sku_code' },
      { title: '商品名称', dataIndex: 'sku_name' },
      { title: '条码', dataIndex: 'barcode' },
      { title: '层级', dataIndex: 'level' },
      {
        title: '包装换算',
        render: (_, record) => `1${record.unit} = ${record.ratio}${baseUnit}`,
      },
      {
        title: '默认',
        dataIndex: 'is_default',
        render: (value: boolean) => (value ? <Tag color="blue">默认</Tag> : '-'),
      },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: BarcodeStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        ),
      },
      { title: '打印模板', dataIndex: 'print_template' },
      {
        title: '更新时间',
        dataIndex: 'updated_at',
        render: (value: string) => dayjs(value).format('YYYY-MM-DD HH:mm'),
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space wrap>
            <Button size="small" disabled={!canWrite} onClick={() => openEditDrawer(record)}>
              编辑
            </Button>
            <Button
              size="small"
              disabled={!canWrite || record.is_default}
              onClick={() => handleSetDefault(record)}
            >
              设为默认
            </Button>
            <Button size="small" disabled={!canWrite} onClick={() => handleToggleStatus(record)}>
              {record.status === 'ACTIVE' ? '停用' : '启用'}
            </Button>
            <Button size="small" onClick={() => message.info('已同步条码至打印模板配置')}>
              同步打印
            </Button>
            <Popconfirm
              title="确认删除该条码？"
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
        <Typography.Title level={3} style={{ margin: 0 }} data-testid="product-barcodes-title">
          条码与包装层级管理
        </Typography.Title>
        <Space>
          <Button onClick={() => message.info('批量导入模板已准备')}>批量导入</Button>
          <Button
            type="primary"
            onClick={openCreateDrawer}
            disabled={!canWrite}
            data-testid="product-barcodes-create-button"
          >
            新增条码
          </Button>
        </Space>
      </Space>
      <Card>
        <Form
          form={form}
          layout="vertical"
          onFinish={handleSearch}
          data-testid="product-barcodes-filter-form"
        >
          <Row gutter={16}>
            <Col xs={24} md={8}>
              <Form.Item name="keyword" label="关键词">
                <Input allowClear placeholder="SKU/商品/条码" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="level" label="层级">
                <Select allowClear options={levelOptions} placeholder="全部层级" />
              </Form.Item>
            </Col>
            <Col xs={24} md={6}>
              <Form.Item name="status" label="状态">
                <Select
                  allowClear
                  options={[
                    { label: '启用', value: 'ACTIVE' },
                    { label: '停用', value: 'INACTIVE' },
                  ]}
                  placeholder="全部状态"
                />
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
      <UiModule03MainTableSection<BarcodeRecord>
        dataTestId="product-barcodes-table"
        rowKey="id"
        columns={columns}
        data={filteredData}
        loading={isLoading || isFetching}
        onRetry={() => refetch()}
        errorTitle="条码加载失败"
        emptyText="暂无条码"
        total={filteredData.length}
        pagination={pagination}
        onPaginationChange={setPagination}
        columnSettingsKey="product.barcodes.table.columns"
        enablePanels={false}
      />
      <Card title="包装层级示例">
        <Table
          rowKey="level"
          size="small"
          pagination={false}
          dataSource={packagingSamples}
          data-testid="product-barcodes-samples-table"
          columns={[
            { title: '层级', dataIndex: 'level' },
            { title: '单位', dataIndex: 'unit' },
            {
              title: '换算',
              render: (_, record) => `1${record.unit} = ${record.ratio}${baseUnit}`,
            },
            { title: '条码规则', dataIndex: 'barcode_rule' },
            { title: '打印模板', dataIndex: 'print_template' },
          ]}
        />
      </Card>
      <Drawer
        title={editingBarcode ? '编辑条码' : '新增条码'}
        open={drawerOpen}
        onClose={() => setDrawerOpen(false)}
        width={520}
        destroyOnClose
        data-testid="product-barcodes-drawer"
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
              disabled={Boolean(editingBarcode)}
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
            name="barcode"
            label="条码"
            rules={[{ required: true, message: '请输入条码' }]}
          >
            <Input placeholder="条码编号" />
          </Form.Item>
          <Form.Item
            name="level"
            label="包装层级"
            rules={[{ required: true, message: '请选择包装层级' }]}
          >
            <Select options={levelOptions} />
          </Form.Item>
          <Form.Item
            name="unit"
            label="包装单位"
            rules={[{ required: true, message: '请输入包装单位' }]}
          >
            <Input placeholder="如：盒/箱" />
          </Form.Item>
          <Form.Item
            name="ratio"
            label={`换算数量(1包装= ?${baseUnit})`}
            rules={[{ required: true, message: '请输入换算数量' }]}
          >
            <InputNumber min={1} precision={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item
            name="print_template"
            label="打印模板"
            rules={[{ required: true, message: '请选择打印模板' }]}
          >
            <Select options={templateOptions} />
          </Form.Item>
          <Form.Item name="status" label="状态" rules={[{ required: true }]}>
            <Select
              options={[
                { label: '启用', value: 'ACTIVE' },
                { label: '停用', value: 'INACTIVE' },
              ]}
            />
          </Form.Item>
          <Form.Item name="is_default" label="设为默认条码" valuePropName="checked">
            <Switch />
          </Form.Item>
        </Form>
      </Drawer>
    </Space>
  );
};
