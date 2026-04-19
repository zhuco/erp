import { Button, Card, Form, InputNumber, Select, Space, Table, Tag, message } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useMemo, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import {
  listProductPackagingSpecs,
  type ProductPackagingSpec,
  upsertDefaultProductPackagingSpec
} from '@/api/productPackagingSpecs';
import { searchSkus } from '@/api/skus';
import { ErrorState } from '@/components/QueryState';
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
  { title: 'Level', dataIndex: 'level_code' },
  { title: 'Unit', dataIndex: 'unit_name' },
  { title: 'Units/Level', dataIndex: 'units_per_level' },
  { title: 'Length(mm)', dataIndex: 'length_mm' },
  { title: 'Width(mm)', dataIndex: 'width_mm' },
  { title: 'Height(mm)', dataIndex: 'height_mm' },
  { title: 'Weight(g)', dataIndex: 'weight_gram' },
  {
    title: 'Default',
    dataIndex: 'is_default',
    render: (value: boolean) => (value ? <Tag color="green">DEFAULT</Tag> : <Tag>-</Tag>)
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

  const specsQuery = useQuery({
    queryKey: ['product-packaging-specs', tenantId, selectedSkuId],
    queryFn: () => listProductPackagingSpecs(tenantId, selectedSkuId ?? ''),
    enabled: tenantReady && Boolean(selectedSkuId),
    placeholderData: (previousData) => previousData
  });

  const handleSave = async () => {
    if (!tenantId || !selectedSkuId) {
      message.warning('Please select one SKU first');
      return;
    }
    if (!canWrite) {
      message.warning('No permission to update packaging specs');
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
      message.success('Packaging spec saved');
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

  return (
    <Space
      direction="vertical"
      size={16}
      style={{ width: '100%' }}
      data-testid="product-packaging-specs-page"
    >
      <Card title="Packaging Spec Governance" size="small">
        <Space direction="vertical" style={{ width: '100%' }} size={12}>
          <Select
            data-testid="packaging-specs-sku-select"
            showSearch
            placeholder="Search SKU by code/name (at least 2 chars)"
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
              label="Level"
              name="level_code"
              rules={[{ required: true, message: 'Required' }]}
            >
              <Select
                style={{ width: 120 }}
                options={[
                  { label: 'EA', value: 'EA' },
                  { label: 'INNER', value: 'INNER' },
                  { label: 'CASE', value: 'CASE' },
                  { label: 'PALLET', value: 'PALLET' }
                ]}
              />
            </Form.Item>
            <Form.Item
              label="Unit"
              name="unit_name"
              rules={[{ required: true, message: 'Required' }]}
            >
              <Select
                style={{ width: 120 }}
                options={[
                  { label: 'Each', value: 'Each' },
                  { label: 'Inner', value: 'Inner' },
                  { label: 'Box', value: 'Box' },
                  { label: 'Pallet', value: 'Pallet' }
                ]}
              />
            </Form.Item>
            <Form.Item
              label="Units/Level"
              name="units_per_level"
              rules={[{ required: true, message: 'Required' }]}
            >
              <InputNumber min={1} />
            </Form.Item>
            <Form.Item label="L(mm)" name="length_mm">
              <InputNumber min={0} />
            </Form.Item>
            <Form.Item label="W(mm)" name="width_mm">
              <InputNumber min={0} />
            </Form.Item>
            <Form.Item label="H(mm)" name="height_mm">
              <InputNumber min={0} />
            </Form.Item>
            <Form.Item label="Weight(g)" name="weight_gram">
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
                Save As Default
              </Button>
            </Form.Item>
          </Form>
        </Space>
      </Card>

      <Card title="Packaging Specs" size="small">
        {specsQuery.error ? (
          <ErrorState
            title="Failed to load packaging specs"
            error={specsQuery.error}
            onRetry={() => specsQuery.refetch()}
          />
        ) : (
          <Table<ProductPackagingSpec>
            data-testid="packaging-specs-table"
            rowKey="spec_id"
            columns={columns}
            dataSource={specsQuery.data?.items ?? []}
            loading={specsQuery.isLoading || specsQuery.isFetching}
            pagination={false}
          />
        )}
      </Card>
    </Space>
  );
};
