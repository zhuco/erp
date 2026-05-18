import { Button, Drawer, Form, Space, message } from 'antd';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useMemo, useState } from 'react';

import { getKitDetail, saveKitBom } from '@/api/kits';
import { updateProduct } from '@/api/products';
import { getSkuDetail, searchSkus, updateSku } from '@/api/skus';

import { defaultGoodsKitFormValues } from '../shared/goodsFields';
import {
  mapKitDetailToFormValues,
  mapKitFormToBomPayload,
  mapKitFormToSkuUpdatePayload,
  mapProductFormToUpdatePayload,
  type GoodsKitFormValues,
} from '../shared/goodsFormUtils';
import { useGoodsMasterData } from '../shared/useGoodsMasterData';
import { KitForm } from './KitForm';

type GoodsKitEditorDialogProps = {
  open: boolean;
  kitSkuId?: string | null;
  onClose: () => void;
  onSaved?: () => void;
};

export const GoodsKitEditorDialog = ({
  open,
  kitSkuId,
  onClose,
  onSaved,
}: GoodsKitEditorDialogProps) => {
  const queryClient = useQueryClient();
  const [form] = Form.useForm<GoodsKitFormValues>();
  const [submitting, setSubmitting] = useState(false);
  const [componentKeyword, setComponentKeyword] = useState('');
  const {
    tenantId,
    brandOptions,
    categoryOptions,
    unitOptions,
    supplierOptions,
    warehouseOptions,
  } = useGoodsMasterData();

  const kitDetailQuery = useQuery({
    queryKey: ['goods-kit-detail', tenantId, kitSkuId],
    queryFn: () => getKitDetail(tenantId, kitSkuId ?? ''),
    enabled: open && Boolean(tenantId && kitSkuId),
  });

  const skuDetailQuery = useQuery({
    queryKey: ['goods-kit-sku-detail', tenantId, kitSkuId],
    queryFn: () => getSkuDetail(tenantId, kitSkuId ?? ''),
    enabled: open && Boolean(tenantId && kitSkuId),
  });

  const componentSkuQuery = useQuery({
    queryKey: ['goods-kit-component-search', tenantId, componentKeyword],
    queryFn: () => searchSkus(tenantId, componentKeyword),
    enabled: Boolean(tenantId && componentKeyword.trim()),
    staleTime: 30_000,
  });

  const componentSkuOptions = useMemo(
    () =>
      (componentSkuQuery.data?.items ?? []).map((item) => ({
        label: `${item.sku_code}${item.name ? ` ${item.name}` : ''}`,
        value: item.sku_id,
      })),
    [componentSkuQuery.data?.items],
  );

  useEffect(() => {
    if (!open) {
      form.resetFields();
      return;
    }
    if (!kitDetailQuery.data) {
      form.setFieldsValue(defaultGoodsKitFormValues);
      return;
    }
    form.setFieldsValue(
      mapKitDetailToFormValues(kitDetailQuery.data, skuDetailQuery.data),
    );
  }, [form, kitDetailQuery.data, open, skuDetailQuery.data]);

  const handleSubmit = async (values: GoodsKitFormValues) => {
    const productId = skuDetailQuery.data?.product?.product_id;
    if (!tenantId || !kitSkuId || !productId) {
      message.warning('缺少组合装主档信息，无法保存');
      return;
    }
    setSubmitting(true);
    try {
      await updateProduct(
        tenantId,
        productId,
        mapProductFormToUpdatePayload(values),
      );
      await updateSku(
        tenantId,
        kitSkuId,
        mapKitFormToSkuUpdatePayload(values),
      );
      await saveKitBom(tenantId, mapKitFormToBomPayload(kitSkuId, values));
      message.success('组合装已更新');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['kits'] }),
        queryClient.invalidateQueries({ queryKey: ['products'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-kits-query'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-products-query'] }),
        queryClient.invalidateQueries({
          queryKey: ['goods-kit-detail-panel'],
        }),
        queryClient.invalidateQueries({
          queryKey: ['goods-kit-detail', tenantId, kitSkuId],
        }),
        queryClient.invalidateQueries({
          queryKey: ['goods-kit-sku-detail', tenantId, kitSkuId],
        }),
      ]);
      onSaved?.();
      onClose();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '组合装更新失败');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Drawer
      title="组合装修改"
      open={open}
      onClose={onClose}
      size="large"
      forceRender
      destroyOnHidden={false}
      extra={
        <Space>
          <Button onClick={onClose}>关闭</Button>
          <Button
            type="primary"
            htmlType="submit"
            form="goods-kit-editor-form"
            loading={submitting}
          >
            保存修改
          </Button>
        </Space>
      }
    >
      <KitForm
        form={form}
        formId="goods-kit-editor-form"
        initialValues={defaultGoodsKitFormValues}
        onFinish={handleSubmit}
        brandOptions={brandOptions}
        categoryOptions={categoryOptions}
        unitOptions={unitOptions}
        supplierOptions={supplierOptions}
        warehouseOptions={warehouseOptions}
        componentSkuOptions={componentSkuOptions}
        componentSkuLoading={componentSkuQuery.isFetching}
        onSearchComponentSku={(keyword) => setComponentKeyword(keyword)}
      />
    </Drawer>
  );
};
