import { Descriptions, Empty, Form, Tag, message } from 'antd';
import type { TabsProps } from 'antd';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Form as AntdForm } from 'antd';
import { useEffect, useMemo, useState } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';

import { getKitDetail, saveKitBom } from '@/api/kits';
import { updateProduct } from '@/api/products';
import { getSkuDetail, searchSkus, updateSku } from '@/api/skus';
import { UiModule04BottomTable } from '@/components/ui-modules';

import { buildGoodsQueryPath } from '../shared/goodsRoutes';
import { defaultGoodsKitFormValues } from '../shared/goodsFields';
import {
  mapKitDetailToFormValues,
  mapKitFormToBomPayload,
  mapKitFormToSkuUpdatePayload,
  mapProductFormToUpdatePayload,
  type GoodsKitFormValues,
} from '../shared/goodsFormUtils';
import { GoodsCreateWorkbench } from '../shared/GoodsCreateWorkbench';
import { useGoodsMasterData } from '../shared/useGoodsMasterData';
import { KitForm } from './KitForm';
import { IndustrialTable } from '@/components/industrial';

export const GoodsKitEditPage = () => {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { kitSkuId } = useParams<{ kitSkuId: string }>();
  const [searchParams] = useSearchParams();
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

  const watchedValues = AntdForm.useWatch([], form);
  const values = {
    ...defaultGoodsKitFormValues,
    ...(watchedValues ?? {}),
  };

  const returnQueryPath = useMemo(
    () => buildGoodsQueryPath('/goods/kits/query', searchParams.toString()),
    [searchParams],
  );

  const kitDetailQuery = useQuery({
    queryKey: ['goods-kit-detail', tenantId, kitSkuId],
    queryFn: () => getKitDetail(tenantId, kitSkuId ?? ''),
    enabled: Boolean(tenantId && kitSkuId),
  });

  const skuDetailQuery = useQuery({
    queryKey: ['goods-kit-sku-detail', tenantId, kitSkuId],
    queryFn: () => getSkuDetail(tenantId, kitSkuId ?? ''),
    enabled: Boolean(tenantId && kitSkuId),
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
    if (!kitSkuId) {
      message.warning('缺少组合装标识，已返回查询页');
      navigate('/goods/kits/query', { replace: true });
    }
  }, [kitSkuId, navigate]);

  useEffect(() => {
    if (!kitDetailQuery.data) {
      return;
    }
    form.setFieldsValue(
      mapKitDetailToFormValues(kitDetailQuery.data, skuDetailQuery.data),
    );
  }, [form, kitDetailQuery.data, skuDetailQuery.data]);

  const componentCount = values.components.length;
  const validComponentCount = values.components.filter(
    (item) => String(item.component_sku_id ?? '').trim() && Number(item.qty ?? 0) > 0,
  ).length;
  const requiredFieldCount = 3;
  const completedRequiredFieldCount = [values.name, values.sku_code, values.status].filter(
    (value) => String(value ?? '').trim(),
  ).length;

  const handleBack = () => {
    navigate(returnQueryPath);
  };

  const handleSubmit = async (nextValues: GoodsKitFormValues) => {
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
        mapProductFormToUpdatePayload(nextValues),
      );
      await updateSku(
        tenantId,
        kitSkuId,
        mapKitFormToSkuUpdatePayload(nextValues),
      );
      await saveKitBom(tenantId, mapKitFormToBomPayload(kitSkuId, nextValues));
      message.success('组合装已更新');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['kits'] }),
        queryClient.invalidateQueries({ queryKey: ['products'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-kits-query'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-products-query'] }),
        queryClient.invalidateQueries({
          queryKey: ['goods-kit-detail', tenantId, kitSkuId],
        }),
        queryClient.invalidateQueries({
          queryKey: ['goods-kit-sku-detail', tenantId, kitSkuId],
        }),
      ]);
      handleBack();
    } catch (error) {
      message.error(error instanceof Error ? error.message : '组合装更新失败');
    } finally {
      setSubmitting(false);
    }
  };

  const detailPanels = [
    {
      key: 'summary',
      label: '组合装摘要',
      children: kitDetailQuery.data ? (
        <Descriptions column={1} size="small" bordered>
          <Descriptions.Item label="组合装名称">
            {values.name || '待填写'}
          </Descriptions.Item>
          <Descriptions.Item label="KIT SKU">
            {values.sku_code || '待填写'}
          </Descriptions.Item>
          <Descriptions.Item label="组件装配">
            {validComponentCount > 0 ? (
              <Tag color="green">{`${validComponentCount} 个有效组件`}</Tag>
            ) : (
              <Tag color="gold">待补 BOM</Tag>
            )}
          </Descriptions.Item>
          <Descriptions.Item label="保存后动作">
            返回原查询上下文
          </Descriptions.Item>
        </Descriptions>
      ) : (
        <Empty description="正在加载组合装主档" />
      ),
    },
  ];

  const bottomItems: TabsProps['items'] = [
    {
      key: 'checklist',
      label: '编辑校验',
      children: (
        <UiModule04BottomTable syncKey={`${values.sku_code}-${componentCount}`}>
          <IndustrialTable
            size="small"
            rowKey="field"
            pagination={false}
            scroll={{ x: 'max-content' }}
            dataSource={[
              {
                field: '必填字段',
                status: `${completedRequiredFieldCount}/${requiredFieldCount}`,
                note: '组合装名称、KIT SKU 编码、货品状态必须明确',
              },
              {
                field: 'BOM 组件数',
                status: `${validComponentCount}/${componentCount || 0}`,
                note: validComponentCount > 0 ? '可生成有效 BOM' : '请至少维护 1 条有效组件',
              },
              {
                field: '供应链默认项',
                status: `${
                  Number(Boolean(String(values.default_supplier_id ?? '').trim())) +
                  Number(Boolean(String(values.default_warehouse_id ?? '').trim()))
                }/2`,
                note: '建议为组合装明确默认供应商与仓库',
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
        <UiModule04BottomTable syncKey="goods-kit-edit-api-flow">
          <IndustrialTable
            size="small"
            rowKey="step"
            pagination={false}
            scroll={{ x: 'max-content' }}
            dataSource={[
              { step: '1', action: 'getKitDetail', result: '加载组合装 BOM 详情' },
              { step: '2', action: 'getSkuDetail', result: '加载 KIT SKU 与货品主档详情' },
              { step: '3', action: 'updateProduct + updateSku', result: '保存主档与 SKU 修改' },
              { step: '4', action: 'saveKitBom', result: '保存 BOM 组件关系' },
              { step: '5', action: 'navigate', result: '返回原查询上下文' },
            ]}
            columns={[
              { title: '步骤', dataIndex: 'step', key: 'step', width: 80 },
              { title: '调用动作', dataIndex: 'action', key: 'action', width: 220 },
              { title: '预期结果', dataIndex: 'result', key: 'result', width: 420 },
            ]}
          />
        </UiModule04BottomTable>
      ),
    },
  ];

  return (
    <GoodsCreateWorkbench
      pageTestId="goods-kit-edit-page"
      title="组合装修改"
      description="当前页面直连组合装与 SKU 接口，保存后返回原查询上下文。"
      statusTag={<Tag color="purple">KIT 组合装</Tag>}
      onBack={handleBack}
      saveLabel="保存修改"
      formId="goods-kit-edit-form"
      submitting={submitting}
      rightPanels={detailPanels}
      bottomItems={bottomItems}
    >
      <KitForm
        form={form}
        formId="goods-kit-edit-form"
        initialValues={{
          ...defaultGoodsKitFormValues,
          components: [{ component_sku_id: '', qty: 1, expire_at: null }],
        }}
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
    </GoodsCreateWorkbench>
  );
};
