import { Button, Form, InputNumber, Select, Space, Tag, Typography, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import {
  listProductPackagingSpecs,
  type ProductPackagingSpec,
  upsertDefaultProductPackagingSpec
} from '@/api/productPackagingSpecs';
import { searchSkus } from '@/api/skus';
import { UiModule03MainTableSection } from '@/components/ui-modules';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type FormValues = {
  level_code: string;
  unit_name: string;
  units_per_level: number;
  length_mm?: number;
  width_mm?: number;
  height_mm?: number;
  weight_gram?: number;
};

const columns: ColumnsType<ProductPackagingSpec> = [
  { title: '层级', dataIndex: 'level_code' },
  { title: '单位', dataIndex: 'unit_name' },
  { title: '每层数量', dataIndex: 'units_per_level' },
  { title: '长(mm)', dataIndex: 'length_mm' },
  { title: '宽(mm)', dataIndex: 'width_mm' },
  { title: '高(mm)', dataIndex: 'height_mm' },
  { title: '重量(g)', dataIndex: 'weight_gram' },
  {
    title: '默认规格',
    dataIndex: 'is_default',
    render: (value: boolean) => (value ? <Tag color="green">默认</Tag> : <Tag>-</Tag>)
  }
];

export const ProductPackagingSpecsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('skus.write');
  const queryClient = useQueryClient();
  const [form] = Form.useForm<FormValues>();
  const [skuKeyword, setSkuKeyword] = useState('');
  const [selectedSkuId, setSelectedSkuId] = useState<string | undefined>(undefined);
  const [saving, setSaving] = useState(false);

  const skuSearchQuery = useQuery({
    queryKey: ['packaging-specs', 'sku-search', tenantId, skuKeyword],
    queryFn: () => searchSkus(tenantId, skuKeyword),
    enabled: tenantReady && skuKeyword.trim().length >= 2,
    placeholderData: (previousData) => previousData
  });

  const skuOptions = useMemo(
    () =>
      (skuSearchQuery.data?.items ?? []).map((item) => ({
        value: item.sku_id,
        label: `${item.sku_code}${item.name ? ` | ${item.name}` : ''}`
      })),
    [skuSearchQuery.data?.items]
  );

  const selectedSkuLabel = useMemo(
    () => skuOptions.find((item) => item.value === selectedSkuId)?.label ?? selectedSkuId,
    [selectedSkuId, skuOptions]
  );

  const specsQuery = useQuery({
    queryKey: ['product-packaging-specs', tenantId, selectedSkuId],
    queryFn: () => listProductPackagingSpecs(tenantId, selectedSkuId ?? ''),
    enabled: tenantReady && Boolean(selectedSkuId),
    placeholderData: (previousData) => previousData
  });

  const handleSave = async () => {
    if (!tenantId || !selectedSkuId) {
      message.warning('请先选择一个 SKU');
      return;
    }
    if (!canWrite) {
      message.warning('当前账号无权维护包装规格');
      return;
    }
    try {
      const values = await form.validateFields();
      setSaving(true);
      await upsertDefaultProductPackagingSpec(tenantId, selectedSkuId, {
        level_code: values.level_code.trim(),
        unit_name: values.unit_name.trim(),
        units_per_level: Number(values.units_per_level),
        length_mm: values.length_mm ?? null,
        width_mm: values.width_mm ?? null,
        height_mm: values.height_mm ?? null,
        weight_gram: values.weight_gram ?? null
      });
      message.success('包装规格已保存');
      await queryClient.invalidateQueries({
        queryKey: ['product-packaging-specs', tenantId, selectedSkuId]
      });
    } catch (error) {
      if (error instanceof Error && error.message) {
        message.error(error.message);
      }
    } finally {
      setSaving(false);
    }
  };

  const tableData = specsQuery.data?.items ?? [];

  return (
    <Space
      direction="vertical"
      size={16}
      style={{ width: '100%' }}
      data-testid="product-packaging-specs-page"
    >
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          <Select
            data-testid="packaging-specs-sku-select"
            showSearch
            placeholder="按编码或名称搜索 SKU（至少输入 2 个字符）"
            value={selectedSkuId}
            onSearch={setSkuKeyword}
            onChange={setSelectedSkuId}
            options={skuOptions}
            filterOption={false}
            style={{ width: 420, maxWidth: '100%' }}
          />
          <Form
            form={form}
            layout="inline"
            data-testid="packaging-specs-form"
            initialValues={{
              level_code: 'CASE',
              unit_name: 'Box',
              units_per_level: 1
            }}
          >
            <Form.Item
              label="包装层级"
              name="level_code"
              rules={[{ required: true, message: '必填' }]}
            >
              <Select
                style={{ width: 132 }}
                options={[
                  { label: '单件（EA）', value: 'EA' },
                  { label: '内包装（INNER）', value: 'INNER' },
                  { label: '箱（CASE）', value: 'CASE' },
                  { label: '托盘（PALLET）', value: 'PALLET' }
                ]}
              />
            </Form.Item>
            <Form.Item
              label="单位名称"
              name="unit_name"
              rules={[{ required: true, message: '必填' }]}
            >
              <Select
                style={{ width: 120 }}
                options={[
                  { label: '单件', value: 'Each' },
                  { label: '内包装', value: 'Inner' },
                  { label: '箱', value: 'Box' },
                  { label: '托盘', value: 'Pallet' }
                ]}
              />
            </Form.Item>
            <Form.Item
              label="每层数量"
              name="units_per_level"
              rules={[{ required: true, message: '必填' }]}
            >
              <InputNumber min={1} />
            </Form.Item>
            <Form.Item label="长(mm)" name="length_mm">
              <InputNumber min={0} />
            </Form.Item>
            <Form.Item label="宽(mm)" name="width_mm">
              <InputNumber min={0} />
            </Form.Item>
            <Form.Item label="高(mm)" name="height_mm">
              <InputNumber min={0} />
            </Form.Item>
            <Form.Item label="重量(g)" name="weight_gram">
              <InputNumber min={0} />
            </Form.Item>
            <Form.Item>
              <Button
                type="primary"
                onClick={handleSave}
                loading={saving}
                disabled={!canWrite}
                data-testid="packaging-specs-save-button"
              >
                保存为默认规格
              </Button>
            </Form.Item>
          </Form>
        </Space>

      <UiModule03MainTableSection<ProductPackagingSpec>
        toolbarLeft={
          <Typography.Title level={3} data-testid="product-packaging-specs-title" style={{ margin: 0 }}>
            包装规格主表
          </Typography.Title>
        }
        toolbarRight={
          <Space size={8}>
            <Tag color={selectedSkuId ? 'blue' : 'default'}>
              {selectedSkuId ? `已选 SKU：${selectedSkuLabel}` : '未选择 SKU'}
            </Tag>
            <Button
              size="small"
              data-testid="product-packaging-specs-refresh-button"
              disabled={!selectedSkuId}
              loading={specsQuery.isFetching}
              onClick={() => specsQuery.refetch()}
            >
              刷新
            </Button>
          </Space>
        }
        rowKey="spec_id"
        columns={columns}
        data={tableData}
        loading={specsQuery.isLoading || specsQuery.isFetching}
        error={specsQuery.error}
        onRetry={() => specsQuery.refetch()}
        errorTitle="包装规格加载失败"
        emptyText="暂无包装规格"
        total={tableData.length}
        pagination={{ current: 1, pageSize: 50, total: tableData.length, hideOnSinglePage: true }}
        onPaginationChange={() => undefined}
        dataTestId="product-packaging-specs-main-grid"
        columnSettingsKey="products.packaging-specs.table.columns"
        gridPreset="industrial-review-compact"
        selectionCellMode="custom"
        selectionHeaderMode="allInvert"
        showColumnSettings
      />
    </Space>
  );
};
