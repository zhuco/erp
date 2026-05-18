import { Descriptions, Form, Tag, message } from 'antd';
import type { TabsProps } from 'antd';
import { useQueryClient } from '@tanstack/react-query';
import { Form as AntdForm } from 'antd';
import { useState } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';

import { createProduct } from '@/api/products';
import { UiModule04BottomTable } from '@/components/ui-modules';

import { defaultGoodsProductFormValues } from '../shared/goodsFields';
import {
  mapProductFormToCreatePayload,
  type GoodsProductFormValues,
} from '../shared/goodsFormUtils';
import { GoodsCreateWorkbench } from '../shared/GoodsCreateWorkbench';
import {
  buildGoodsProductEditPath,
  buildGoodsQueryPath,
} from '../shared/goodsRoutes';
import { useGoodsMasterData } from '../shared/useGoodsMasterData';
import { ProductForm } from './ProductForm';
import { IndustrialTable } from '@/components/industrial';

export const GoodsProductCreatePage = () => {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const [form] = Form.useForm<GoodsProductFormValues>();
  const {
    tenantId,
    brandOptions,
    categoryOptions,
    unitOptions,
    supplierOptions,
    warehouseOptions,
  } = useGoodsMasterData();
  const [submitting, setSubmitting] = useState(false);
  const watchedValues = AntdForm.useWatch([], form);
  const values = {
    ...defaultGoodsProductFormValues,
    ...(watchedValues ?? {}),
  };
  const requiredFieldCount = 2;
  const completedRequiredFieldCount = [
    values.name,
    values.status,
  ].filter((value) => String(value ?? '').trim()).length;
  const hasSupplier = Boolean(String(values.default_supplier_id ?? '').trim());
  const hasWarehouse = Boolean(String(values.default_warehouse_id ?? '').trim());
  const inventoryGovernanceEnabled =
    values.batch_enabled || values.serial_enabled || values.allow_negative_stock;
  const returnQueryPath = buildGoodsQueryPath(
    '/goods/products/query',
    searchParams.toString(),
  );

  const handleSubmit = async (values: GoodsProductFormValues) => {
    if (!tenantId) {
      message.warning('请先选择租户');
      return;
    }
    setSubmitting(true);
    try {
      const created = await createProduct(
        tenantId,
        mapProductFormToCreatePayload(values),
      );
      message.success('货品已创建');
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['products'] }),
        queryClient.invalidateQueries({ queryKey: ['goods-products-query'] }),
      ]);
      navigate(buildGoodsProductEditPath(created.product_id, searchParams.toString()));
    } catch (error) {
      message.error(error instanceof Error ? error.message : '货品创建失败');
    } finally {
      setSubmitting(false);
    }
  };

  const detailPanels = [
    {
      key: 'summary',
      label: '主档摘要',
      children: (
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
            创建主档并跳转到货品查询编辑态
          </Descriptions.Item>
        </Descriptions>
      ),
    },
    {
      key: 'quality',
      label: '建档提示',
      children: (
        <div className="goods-create-workbench-note-list">
          <div className="goods-create-workbench-note-item">
            <div className="goods-create-workbench-note-title">识别口径</div>
            <div className="goods-create-workbench-note-text">
              货品用于 ERP 内部识别，建议先明确品牌、分类与单位。
            </div>
          </div>
          <div className="goods-create-workbench-note-item">
            <div className="goods-create-workbench-note-title">供应链配置</div>
            <div className="goods-create-workbench-note-text">
              默认供应商和默认仓库决定后续采购、入库与拣配默认落点。
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
                note: '货品名称和状态为当前最小建档门槛',
              },
              {
                field: '默认供应商',
                status: hasSupplier ? '已配置' : '待补充',
                note: hasSupplier ? '可直接承接采购默认值' : '建议配置默认供应商',
              },
              {
                field: '默认仓库',
                status: hasWarehouse ? '已配置' : '待补充',
                note: hasWarehouse ? '可直接承接仓储默认值' : '建议配置默认仓库',
              },
              {
                field: '库存策略',
                status: inventoryGovernanceEnabled ? '已启用治理' : '基础模式',
                note: inventoryGovernanceEnabled
                  ? '已启用批次/序列号/负库存中的至少一项'
                  : '适合普通单品主档',
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
        <UiModule04BottomTable syncKey="goods-product-create-api-flow">
          <IndustrialTable
            size="small"
            rowKey="step"
            pagination={false}
            scroll={{ x: 'max-content' }}
            dataSource={[
              {
                step: '1',
                action: 'createProduct',
                result: '创建 ERP 货品主档',
              },
              {
                step: '2',
                action: 'invalidateQueries',
                result: '刷新 products 与 goods-products-query 缓存',
              },
              {
                step: '3',
                action: 'navigate',
                result: '跳转到货品查询页并打开编辑态',
              },
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
      pageTestId="goods-product-create-page"
      title="货品新建"
      description="按 ERP 统一主档规范创建货品，保存后自动进入查询页编辑态。"
      statusTag={<Tag color="blue">货品主档</Tag>}
      onBack={() => navigate(returnQueryPath)}
      saveLabel="保存货品"
      formId="goods-product-create-form"
      submitting={submitting}
      rightPanels={detailPanels}
      bottomItems={bottomItems}
    >
      <ProductForm
        form={form}
        formId="goods-product-create-form"
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
