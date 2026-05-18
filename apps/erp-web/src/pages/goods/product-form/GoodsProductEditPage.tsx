import { Descriptions, Empty, Form, Tag, message } from 'antd';
import type { TabsProps } from 'antd';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Form as AntdForm } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';

import { getProduct, updateProduct } from '@/api/products';
import { UiModule04BottomTable } from '@/components/ui-modules';

import { buildGoodsQueryPath } from '../shared/goodsRoutes';
import { defaultGoodsProductFormValues } from '../shared/goodsFields';
import {
  mapProductFormToUpdatePayload,
  mapProductToFormValues,
  type GoodsProductFormValues,
} from '../shared/goodsFormUtils';
import { GoodsCreateWorkbench } from '../shared/GoodsCreateWorkbench';
import { useGoodsMasterData } from '../shared/useGoodsMasterData';
import { ProductForm } from './ProductForm';
import { IndustrialTable } from '@/components/industrial';

export const GoodsProductEditPage = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { productId } = useParams<{ productId: string }>();
  const [searchParams] = useSearchParams();
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

  const watchedValues = AntdForm.useWatch([], form);
  const values = {
    ...defaultGoodsProductFormValues,
    ...(watchedValues ?? {}),
  };

  const returnQueryPath = useMemo(
    () => buildGoodsQueryPath('/goods/products/query', searchParams.toString()),
    [searchParams],
  );

  const productQuery = useQuery({
    queryKey: ['goods-product-detail', tenantId, productId],
    queryFn: () => getProduct(tenantId, productId ?? ''),
    enabled: Boolean(tenantId && productId),
  });

  useEffect(() => {
    if (!productId) {
      message.warning('缺少货品标识，已返回查询页');
      navigate('/goods/products/query', { replace: true });
    }
  }, [navigate, productId]);

  useEffect(() => {
    if (!productQuery.data) {
      return;
    }
    form.setFieldsValue(mapProductToFormValues(productQuery.data));
  }, [form, productQuery.data]);

  const requiredFieldCount = 2;
  const completedRequiredFieldCount = [values.name, values.status].filter(
    (value) => String(value ?? '').trim(),
  ).length;
  const hasSupplier = Boolean(String(values.default_supplier_id ?? '').trim());
  const hasWarehouse = Boolean(String(values.default_warehouse_id ?? '').trim());
  const inventoryGovernanceEnabled =
    values.batch_enabled || values.serial_enabled || values.allow_negative_stock;

  const handleBack = () => {
    navigate(returnQueryPath);
  };

  const handleSubmit = async (nextValues: GoodsProductFormValues) => {
    if (!tenantId || !productId) {
      message.warning('缺少货品标识，无法保存');
      return;
    }
    setSubmitting(true);
    try {
      await updateProduct(
        tenantId,
        productId,
        mapProductFormToUpdatePayload(nextValues),
      );
      message.success('货品已更新');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['products'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-products-query'] }),
        queryClient.invalidateQueries({
          queryKey: ['goods-product-detail', tenantId, productId],
        }),
      ]);
      handleBack();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '货品更新失败');
    } finally {
      setSubmitting(false);
    }
  };

  const detailPanels = [
    {
      key: 'summary',
      label: '主档摘要',
      children: productQuery.data ? (
        <Descriptions column={1} size="small" bordered>
          <Descriptions.Item label="货品名称">
            {values.name || '待填写'}
          </Descriptions.Item>
          <Descriptions.Item label="货品编码">
            {values.product_code || '系统允许留空'}
          </Descriptions.Item>
          <Descriptions.Item label="品牌 / 分类">
            {(values.brand || '未选择') + ' / ' + (values.category || '未选择')}
          </Descriptions.Item>
          <Descriptions.Item label="库存治理">
            {inventoryGovernanceEnabled ? (
              <Tag color="green">已配置</Tag>
            ) : (
              <Tag>基础模式</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="保存后动作">
            返回原查询上下文
          </Descriptions.Item>
        </Descriptions>
      ) : (
        <Empty description="正在加载货品主档" />
      ),
    },
  ];

  const bottomItems: TabsProps['items'] = [
    {
      key: 'checklist',
      label: '编辑校验',
      children: (
        <UiModule04BottomTable syncKey={`${values.name}-${values.status}`}>
          <IndustrialTable
            size="small"
            rowKey="field"
            pagination={false}
            scroll={{ x: 'max-content' }}
            dataSource={[
              {
                field: '必填字段',
                status: `${completedRequiredFieldCount}/${requiredFieldCount}`,
                note: '货品名称和状态为当前最小保存门槛',
              },
              {
                field: '默认供应商',
                status: hasSupplier ? '已配置' : '待补充',
                note: hasSupplier ? '采购默认值完整' : '建议完善默认供应商',
              },
              {
                field: '默认仓库',
                status: hasWarehouse ? '已配置' : '待补充',
                note: hasWarehouse ? '仓储默认值完整' : '建议完善默认仓库',
              },
              {
                field: '库存策略',
                status: inventoryGovernanceEnabled ? '已启用治理' : '基础模式',
                note: inventoryGovernanceEnabled
                  ? '批次、序列号或负库存策略已开启'
                  : '适合普通单品编辑态',
              },
            ]}
            columns={[
              { title: '检查项', dataIndex: 'field', key: 'field', width: 180 },
              { title: '当前状态', dataIndex: 'status', key: 'status', width: 160 },
              { title: '说明', dataIndex: 'note', key: 'note', width: 420 },
            ]}
          />
        </UiModule04BottomTable>
      ),
    },
    {
      key: 'api-flow',
      label: '接口链路',
      children: (
        <UiModule04BottomTable syncKey="goods-product-edit-api-flow">
          <IndustrialTable
            size="small"
            rowKey="step"
            pagination={false}
            scroll={{ x: 'max-content' }}
            dataSource={[
              { step: '1', action: 'getProduct', result: '加载货品主档详情' },
              { step: '2', action: 'updateProduct', result: '保存货品主档修改' },
              { step: '3', action: 'invalidateQueries', result: '刷新查询与详情缓存' },
              { step: '4', action: 'navigate', result: '返回原查询上下文' },
            ]}
            columns={[
              { title: '步骤', dataIndex: 'step', key: 'step', width: 80 },
              { title: '调用动作', dataIndex: 'action', key: 'action', width: 180 },
              { title: '预期结果', dataIndex: 'result', key: 'result', width: 420 },
            ]}
          />
        </UiModule04BottomTable>
      ),
    },
  ];

  return (
    <GoodsCreateWorkbench
      pageTestId="goods-product-edit-page"
      title="货品修改"
      description="当前页面直连现有货品接口，保存后返回原查询上下文。"
      statusTag={<Tag color="blue">货品主档</Tag>}
      onBack={handleBack}
      saveLabel="保存修改"
      formId="goods-product-edit-form"
      submitting={submitting}
      rightPanels={detailPanels}
      bottomItems={bottomItems}
    >
      <ProductForm
        form={form}
        formId="goods-product-edit-form"
        initialValues={defaultGoodsProductFormValues}
        onFinish={handleSubmit}
        brandOptions={brandOptions}
        categoryOptions={categoryOptions}
        unitOptions={unitOptions}
        supplierOptions={supplierOptions}
        warehouseOptions={warehouseOptions}
        footer={null}
      />
    </GoodsCreateWorkbench>
  );
};
