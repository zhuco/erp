import { Button, Drawer, Form, Space, message } from 'antd';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';

import { getProduct, updateProduct } from '@/api/products';

import { defaultGoodsProductFormValues } from '../shared/goodsFields';
import {
  mapProductFormToUpdatePayload,
  mapProductToFormValues,
  type GoodsProductFormValues,
} from '../shared/goodsFormUtils';
import { useGoodsMasterData } from '../shared/useGoodsMasterData';
import { ProductForm } from './ProductForm';

type GoodsProductEditorDialogProps = {
  open: boolean;
  productId?: string | null;
  onClose: () => void;
  onSaved?: () => void;
};

export const GoodsProductEditorDialog = ({
  open,
  productId,
  onClose,
  onSaved,
}: GoodsProductEditorDialogProps) => {
  const queryClient = useQueryClient();
  const [form] = Form.useForm<GoodsProductFormValues>();
  const [submitting, setSubmitting] = useState(false);
  const {
    tenantId,
    brandOptions,
    categoryOptions,
    unitOptions,
    supplierOptions,
    warehouseOptions,
  } = useGoodsMasterData();

  const productQuery = useQuery({
    queryKey: ['goods-product-detail', tenantId, productId],
    queryFn: () => getProduct(tenantId, productId ?? ''),
    enabled: open && Boolean(tenantId && productId),
  });

  useEffect(() => {
    if (!open) {
      form.resetFields();
      return;
    }
    if (!productQuery.data) {
      form.setFieldsValue(defaultGoodsProductFormValues);
      return;
    }
    form.setFieldsValue(mapProductToFormValues(productQuery.data));
  }, [form, open, productQuery.data]);

  const handleSubmit = async (values: GoodsProductFormValues) => {
    if (!tenantId || !productId) {
      message.warning('缺少货品标识，无法保存');
      return;
    }
    setSubmitting(true);
    try {
      await updateProduct(
        tenantId,
        productId,
        mapProductFormToUpdatePayload(values),
      );
      message.success('货品已更新');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['products'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-products-query'] }),
        queryClient.invalidateQueries({
          queryKey: ['goods-product-detail', tenantId, productId],
        }),
      ]);
      onSaved?.();
      onClose();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '货品更新失败');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Drawer
      title="货品修改"
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
            form="goods-product-editor-form"
            loading={submitting}
          >
            保存修改
          </Button>
        </Space>
      }
    >
      <ProductForm
        form={form}
        formId="goods-product-editor-form"
        initialValues={defaultGoodsProductFormValues}
        onFinish={handleSubmit}
        brandOptions={brandOptions}
        categoryOptions={categoryOptions}
        unitOptions={unitOptions}
        supplierOptions={supplierOptions}
        warehouseOptions={warehouseOptions}
      />
    </Drawer>
  );
};
