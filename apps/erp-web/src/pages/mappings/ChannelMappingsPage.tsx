import { Button, Card, Form, Input, Select, Space, Typography, message } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';

import { useSkusList } from '@/api/hooks';
import { createMapping } from '@/api/mappings';
import { type PlatformCode } from '@/features/platform';
import { useCan } from '@/features/rbac';
import { useTenantId } from '@/features/tenant';

type FormValues = {
  platform?: string;
  shop_id?: string;
  platform_sku_id?: string;
  outer_sku_id?: string;
  sku_id?: string;
};

export const ChannelMappingsPage = () => {
  const tenantId = useTenantId();
  const canWrite = useCan('channel_sku_mappings.write');
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [form] = Form.useForm<FormValues>();
  const [saving, setSaving] = useState(false);

  const defaults = useMemo(
    () => ({
      platform: searchParams.get('platform') ?? '',
      shop_id: searchParams.get('shop_id') ?? '',
      platform_sku_id: searchParams.get('platform_sku_id') ?? '',
      outer_sku_id: searchParams.get('outer_sku_id') ?? ''
    }),
    [searchParams]
  );

  useEffect(() => {
    form.setFieldsValue(defaults);
  }, [defaults, form]);

  const returnTo = searchParams.get('return_to') ?? '/orders';

  const skuQuery = useSkusList({ pageSize: 200, enabled: Boolean(tenantId) });
  const skuOptions = useMemo(
    () =>
      (skuQuery.data?.data ?? []).map((sku) => ({
        label: `${sku.sku_code}${sku.name ? ` ${sku.name}` : ''}`,
        value: sku.sku_id
      })),
    [skuQuery.data?.data]
  );
  const selectedSkuId = Form.useWatch('sku_id', form);
  const selectedSku = useMemo(
    () => (skuQuery.data?.data ?? []).find((sku) => sku.sku_id === selectedSkuId),
    [selectedSkuId, skuQuery.data?.data]
  );

  const handleSave = async () => {
    if (!tenantId) {
      message.error('请先选择商户');
      return;
    }
    if (!canWrite) {
      message.warning('暂无渠道映射权限');
      return;
    }
    try {
      const values = await form.validateFields();
      if (!values.platform || !values.shop_id || !values.platform_sku_id) {
        message.warning('请补全渠道与 SKU 信息');
        return;
      }
      if (!selectedSkuId) {
        message.warning('请选择 ERP SKU');
        return;
      }
      setSaving(true);
      await createMapping(tenantId, {
        platform: values.platform as PlatformCode,
        shop_id: values.shop_id,
        platform_sku_id: values.platform_sku_id,
        outer_sku_id: values.outer_sku_id?.trim() || null,
        sku_id: selectedSkuId,
        sku_code: selectedSku?.sku_code ?? null,
        barcode: selectedSku?.barcode ?? null
      });
      message.success('渠道映射已保存');
      navigate(returnTo);
    } catch (error) {
      message.error(error instanceof Error ? error.message : '保存渠道映射失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Typography.Title level={3} style={{ margin: 0 }}>
        渠道映射
      </Typography.Title>
      <Card>
        <Form form={form} layout="vertical">
          <Form.Item label="渠道平台" name="platform" rules={[{ required: true }]}>
            <Input placeholder="平台代码" />
          </Form.Item>
          <Form.Item label="店铺 ID" name="shop_id" rules={[{ required: true }]}>
            <Input data-testid="mapping-shop-id" placeholder="店铺 ID" />
          </Form.Item>
          <Form.Item label="平台 SKU" name="platform_sku_id" rules={[{ required: true }]}>
            <Input data-testid="mapping-platform-sku" placeholder="平台 SKU" />
          </Form.Item>
          <Form.Item label="外部 SKU" name="outer_sku_id">
            <Input data-testid="mapping-outer-sku" placeholder="外部 SKU" />
          </Form.Item>
          <Form.Item label="ERP SKU" name="sku_id" rules={[{ required: true }]}>
            <Select
              showSearch
              placeholder="选择 ERP SKU"
              options={skuOptions}
              loading={skuQuery.isLoading}
              optionFilterProp="label"
              data-testid="mapping-sku-select"
            />
          </Form.Item>
          <Space>
            <Button onClick={() => navigate(returnTo)}>返回</Button>
            <Button
              type="primary"
              loading={saving}
              onClick={handleSave}
              data-testid="mapping-save-button"
            >
              保存映射
            </Button>
          </Space>
        </Form>
      </Card>
    </Space>
  );
};
