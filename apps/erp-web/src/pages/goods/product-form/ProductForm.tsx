import { Col, Flex, Form, Input, InputNumber, Row, Select, Switch } from 'antd';
import type { FormInstance } from 'antd';
import type { ReactNode } from 'react';

import { SettingsPageSection } from '@/components/page-templates';
import type { GoodsOption } from '@/pages/goods/shared/useGoodsMasterData';

import { goodsStatusOptions } from '../shared/goodsFields';
import type { GoodsProductFormValues } from '../shared/goodsFormUtils';

type ProductFormProps = {
  form: FormInstance<GoodsProductFormValues>;
  formId?: string;
  initialValues: GoodsProductFormValues;
  onFinish: (values: GoodsProductFormValues) => void | Promise<void>;
  brandOptions: GoodsOption[];
  categoryOptions: GoodsOption[];
  unitOptions: GoodsOption[];
  supplierOptions: GoodsOption[];
  warehouseOptions: GoodsOption[];
  footer?: ReactNode;
};

export const ProductForm = ({
  form,
  formId,
  initialValues,
  onFinish,
  brandOptions,
  categoryOptions,
  unitOptions,
  supplierOptions,
  warehouseOptions,
  footer,
}: ProductFormProps) => (
  <Form<GoodsProductFormValues>
    id={formId}
    form={form}
    size="small"
    className="goods-compact-form"
    layout="vertical"
    initialValues={initialValues}
    onFinish={onFinish}
  >
    <Flex vertical gap={12} style={{ width: '100%' }}>
      <SettingsPageSection
        className="goods-form-section goods-form-section--compact"
        title="基础信息"
        description="维护 ERP 内部识别使用的主档信息和基础归类。"
      >
        <Row gutter={[12, 4]}>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="货品编码" name="product_code">
              <Input maxLength={64} placeholder="请输入货品编码" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item
              label="货品名称"
              name="name"
              rules={[{ required: true, message: '请输入货品名称' }]}
            >
              <Input maxLength={120} placeholder="请输入货品名称" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="货品别名" name="alias">
              <Input maxLength={120} placeholder="请输入货品别名" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="状态" name="status">
              <Select options={[...goodsStatusOptions]} placeholder="请选择状态" />
            </Form.Item>
          </Col>
        </Row>

        <Row gutter={[12, 4]}>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="品牌" name="brand">
              <Select
                allowClear
                showSearch
                options={brandOptions}
                optionFilterProp="label"
                placeholder="请选择品牌"
              />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="分类" name="category">
              <Select
                allowClear
                showSearch
                options={categoryOptions}
                optionFilterProp="label"
                placeholder="请选择分类"
              />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="主单位" name="base_unit">
              <Select
                allowClear
                showSearch
                options={unitOptions}
                optionFilterProp="label"
                placeholder="请选择主单位"
              />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="副单位" name="second_unit">
              <Select
                allowClear
                showSearch
                options={unitOptions}
                optionFilterProp="label"
                placeholder="请选择副单位"
              />
            </Form.Item>
          </Col>
        </Row>

        <Row gutter={[12, 4]}>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="主条码" name="primary_barcode">
              <Input maxLength={64} placeholder="请输入主条码" />
            </Form.Item>
          </Col>
        </Row>
      </SettingsPageSection>

      <SettingsPageSection
        className="goods-form-section goods-form-section--compact"
        title="采购与仓储"
        description="指定默认供应商、默认仓库以及标签信息。"
      >
        <Row gutter={[12, 4]}>
          <Col xs={24} md={12} xl={8}>
            <Form.Item label="默认供应商" name="default_supplier_id">
              <Select
                allowClear
                showSearch
                options={supplierOptions}
                optionFilterProp="label"
                placeholder="请选择默认供应商"
              />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8}>
            <Form.Item label="默认仓库" name="default_warehouse_id">
              <Select
                allowClear
                showSearch
                options={warehouseOptions}
                optionFilterProp="label"
                placeholder="请选择默认仓库"
              />
            </Form.Item>
          </Col>
          <Col xs={24} xl={8}>
            <Form.Item label="货品标签" name="product_tags">
              <Select
                mode="tags"
                tokenSeparators={[',', '，', ' ']}
                open={false}
                placeholder="输入后回车，可录入多个标签"
              />
            </Form.Item>
          </Col>
        </Row>

        <Row gutter={[12, 4]}>
          <Col xs={24}>
            <Form.Item label="备注" name="remark">
              <Input.TextArea rows={3} placeholder="请输入备注信息" />
            </Form.Item>
          </Col>
        </Row>
      </SettingsPageSection>

      <SettingsPageSection
        className="goods-form-section goods-form-section--compact"
        title="库存与批次策略"
        description="设置预警库存、批次、序列号和负库存策略。"
      >
        <Row gutter={[12, 4]}>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="预警上限" name="warning_upper">
              <InputNumber min={0} precision={0} style={{ width: '100%' }} placeholder="不限制则留空" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="预警下限" name="warning_lower">
              <InputNumber min={0} precision={0} style={{ width: '100%' }} placeholder="不限制则留空" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="保质期天数" name="shelf_life_days">
              <InputNumber min={0} precision={0} style={{ width: '100%' }} placeholder="请输入保质期天数" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="禁售提前天数" name="forbid_sale_days">
              <InputNumber min={0} precision={0} style={{ width: '100%' }} placeholder="请输入禁售提前天数" />
            </Form.Item>
          </Col>
        </Row>

        <Row gutter={[12, 4]}>
          <Col xs={12} md={6}>
            <Form.Item label="启用批次管理" name="batch_enabled" valuePropName="checked">
              <Switch checkedChildren="启用" unCheckedChildren="关闭" />
            </Form.Item>
          </Col>
          <Col xs={12} md={6}>
            <Form.Item label="启用序列号管理" name="serial_enabled" valuePropName="checked">
              <Switch checkedChildren="启用" unCheckedChildren="关闭" />
            </Form.Item>
          </Col>
          <Col xs={12} md={6}>
            <Form.Item label="序列号同步平台" name="serial_push_platform" valuePropName="checked">
              <Switch checkedChildren="同步" unCheckedChildren="不同步" />
            </Form.Item>
          </Col>
          <Col xs={12} md={6}>
            <Form.Item label="允许负库存" name="allow_negative_stock" valuePropName="checked">
              <Switch checkedChildren="允许" unCheckedChildren="禁止" />
            </Form.Item>
          </Col>
        </Row>
      </SettingsPageSection>

      {footer ? <div>{footer}</div> : null}
    </Flex>
  </Form>
);
