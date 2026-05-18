import { Descriptions, Form, Tag, message } from 'antd';
import type { TabsProps } from 'antd';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Form as AntdForm } from 'antd';
import { useMemo, useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';

import { saveKitBom } from '@/api/kits';
import { createProduct } from '@/api/products';
import { createSku, searchSkus } from '@/api/skus';
import { UiModule04BottomTable } from '@/components/ui-modules';

import { defaultGoodsKitFormValues } from '../shared/goodsFields';
import {
  mapKitFormToBomPayload,
  mapKitFormToProductPayload,
  mapKitFormToSkuPayload,
  type GoodsKitFormValues,
} from '../shared/goodsFormUtils';
import { GoodsCreateWorkbench } from '../shared/GoodsCreateWorkbench';
import {
  buildGoodsKitEditPath,
  buildGoodsQueryPath,
} from '../shared/goodsRoutes';
import { useGoodsMasterData } from '../shared/useGoodsMasterData';
import { KitForm } from './KitForm';
import { IndustrialTable } from '@/components/industrial';

export const GoodsKitCreatePage = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
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
  const watchedValues = AntdForm.useWatch([], form);
  const values = {
    ...defaultGoodsKitFormValues,
    ...(watchedValues ?? {}),
  };
  const componentCount = values.components.length;
  const validComponentCount = values.components.filter(
    (item) => String(item.component_sku_id ?? '').trim() && Number(item.qty ?? 0) > 0,
  ).length;
  const requiredFieldCount = 3;
  const completedRequiredFieldCount = [
    values.name,
    values.sku_code,
    values.status,
  ].filter((value) => String(value ?? '').trim()).length;
  const returnQueryPath = buildGoodsQueryPath('/goods/kits/query', searchParams.toString());

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

  const handleSubmit = async (values: GoodsKitFormValues) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    setSubmitting(true);
    try {
      const product = await createProduct(
        tenantId,
        mapKitFormToProductPayload(values),
      );
      const sku = await createSku(
        tenantId,
        mapKitFormToSkuPayload(product.product_id, values),
      );
      await saveKitBom(tenantId, mapKitFormToBomPayload(sku.sku_id, values));
      message.success('组合装已创建');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['kits'] }),
        queryClient.invalidateQueries({ queryKey: ['products'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-kits-query'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-products-query'] }),
      ]);
      navigate(buildGoodsKitEditPath(sku.sku_id, searchParams.toString()));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '组合装创建失败');
    } finally {
      setSubmitting(false);
    }
  };

  const detailPanels = [
    {
      key: 'summary',
      label: '组合装摘要',
      children: (
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
            依次创建 Product、KIT SKU 与 BOM
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'governance',
      label: '建档策略',
      children: (
        <div className="goods-create-workbench-note-list">
          <div className="goods-create-workbench-note-item">
            <div className="goods-create-workbench-note-title">建档顺序</div>
            <div className="goods-create-workbench-note-text">
              组合装先落货品主档，再创建 KIT SKU，最后写入 BOM 组件关系。
            </div>
          </div>
          <div className="goods-create-workbench-note-item">
            <div className="goods-create-workbench-note-title">BOM 最小要求</div>
            <div className="goods-create-workbench-note-text">
              至少保留 1 条有效组件，并确保数量大于 0。
            </div>
          </div>
        </div>
      ),
    },
  ];

  const bottomItems: TabsProps['items'] = [
    {
      key: 'checklist',
      label: '建档校验',
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
        <UiModule04BottomTable syncKey="goods-kit-create-api-flow">
          <IndustrialTable
            size="small"
            rowKey="step"
            pagination={false}
            scroll={{ x: 'max-content' }}
            dataSource={[
              { step: '1', action: 'createProduct', result: '创建组合装对应货品主档' },
              { step: '2', action: 'createSku', result: '创建 KIT SKU 记录' },
              { step: '3', action: 'saveKitBom', result: '保存 BOM 组件关系' },
              { step: '4', action: 'navigate', result: '跳转到组合装查询页编辑态' },
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
      pageTestId="goods-kit-create-page"
      title="新建组合装"
      description="创建时将直接调用 createProduct、createSku 与 saveKitBom，完成可实际使用的组合装建档。"
      statusTag={<Tag color="purple">KIT 组合装</Tag>}
      onBack={() => navigate(returnQueryPath)}
      saveLabel="保存组合装"
      formId="goods-kit-create-form"
      submitting={submitting}
      rightPanels={detailPanels}
      bottomItems={bottomItems}
    >
      <KitForm
        form={form}
        formId="goods-kit-create-form"
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
