import { DeleteOutlined, PlusOutlined } from '@ant-design/icons';
import {
  Button,
  Col,
  Divider,
  Flex,
  Form,
  Input,
  InputNumber,
  Row,
  Select,
  Switch,
} from 'antd';
import type { FormInstance } from 'antd';
import type { ReactNode } from 'react';

import { SettingsPageSection } from '@/components/page-templates';
import type { GoodsOption } from '@/pages/goods/shared/useGoodsMasterData';

import { goodsStatusOptions } from '../shared/goodsFields';
import type { GoodsKitFormValues } from '../shared/goodsFormUtils';

type KitFormProps = {
  form: FormInstance<GoodsKitFormValues>;
  formId?: string;
  initialValues: GoodsKitFormValues;
  onFinish: (values: GoodsKitFormValues) => void | Promise<void>;
  brandOptions: GoodsOption[];
  categoryOptions: GoodsOption[];
  unitOptions: GoodsOption[];
  supplierOptions: GoodsOption[];
  warehouseOptions: GoodsOption[];
  componentSkuOptions: GoodsOption[];
  onSearchComponentSku: (keyword: string) => void;
  componentSkuLoading?: boolean;
  submitting?: boolean;
  footer?: ReactNode;
};

export const KitForm = ({
  form,
  formId,
  initialValues,
  onFinish,
  brandOptions,
  categoryOptions,
  unitOptions,
  supplierOptions,
  warehouseOptions,
  componentSkuOptions,
  onSearchComponentSku,
  componentSkuLoading = false,
  footer,
}: KitFormProps) => (
  <Form<GoodsKitFormValues>
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
        title="主档信息"
        description="组合装仍以货品主档为主体，维护内部识别和分类信息。"
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
              rules={[{ required: true, message: '请输入组合装名称' }]}
            >
              <Input maxLength={120} placeholder="请输入组合装名称" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="货品别名" name="alias">
              <Input maxLength={120} placeholder="请输入货品别名" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="货品状态" name="status">
              <Select options={[...goodsStatusOptions]} placeholder="请选择货品状态" />
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
        title="KIT SKU 信息"
        description="组合装创建链路会依次创建 Product、KIT SKU 并保存 BOM。"
      >
        <Row gutter={[12, 4]}>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item
              label="SKU 编码"
              name="sku_code"
              rules={[{ required: true, message: '请输入 KIT SKU 编码' }]}
            >
              <Input maxLength={64} placeholder="请输入 KIT SKU 编码" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="SKU 名称" name="sku_name">
              <Input maxLength={120} placeholder="请输入 KIT SKU 名称" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="SKU 条码" name="sku_barcode">
              <Input maxLength={64} placeholder="请输入 SKU 条码" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="SKU 状态" name="sku_status">
              <Select options={[...goodsStatusOptions]} placeholder="请选择 SKU 状态" />
            </Form.Item>
          </Col>
        </Row>

        <Row gutter={[12, 4]}>
          <Col xs={24} md={12} xl={8} xxl={6}>
            <Form.Item label="成本价（分）" name="cost_cent">
              <InputNumber min={0} precision={0} style={{ width: '100%' }} placeholder="请输入成本价（分）" />
            </Form.Item>
          </Col>
          <Col xs={24} md={12} xl={16} xxl={12}>
            <Form.Item label="SKU 标签" name="sku_tags">
              <Select
                mode="tags"
                tokenSeparators={[',', '，', ' ']}
                open={false}
                placeholder="输入后回车，可录入多个 SKU 标签"
              />
            </Form.Item>
          </Col>
        </Row>

        <Row gutter={[12, 4]}>
          <Col xs={24}>
            <Form.Item label="SKU 备注" name="sku_note">
              <Input.TextArea rows={3} placeholder="请输入 SKU 备注" />
            </Form.Item>
          </Col>
        </Row>
      </SettingsPageSection>

      <SettingsPageSection
        className="goods-form-section goods-form-section--compact"
        title="库存与批次策略"
        description="组合装沿用主档库存策略，并可定义 BOM 组件明细。"
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

      <SettingsPageSection
        className="goods-form-section goods-form-section--compact"
        title="BOM 组件"
        description="至少维护一条组件记录，保存时将直接调用 saveKitBom。"
      >
        <Form.List name="components">
          {(fields, { add, remove }) => (
            <Flex vertical gap={8} style={{ width: '100%' }} className="goods-form-bom-list">
              {fields.map((field, index) => (
                <div key={field.key} className="goods-form-bom-row">
                  <Row gutter={[12, 4]} align="middle">
                    <Col xs={24} xl={12}>
                      <Form.Item
                        label={index === 0 ? '组件 SKU' : undefined}
                        name={[field.name, 'component_sku_id']}
                        rules={[{ required: true, message: '请选择组件 SKU' }]}
                      >
                        <Select
                          allowClear
                          showSearch
                          filterOption={false}
                          options={componentSkuOptions}
                          placeholder="搜索并选择组件 SKU"
                          notFoundContent={componentSkuLoading ? '正在搜索组件 SKU...' : '暂无可选 SKU'}
                          onSearch={onSearchComponentSku}
                        />
                      </Form.Item>
                    </Col>
                    <Col xs={12} md={6} xl={4}>
                      <Form.Item
                        label={index === 0 ? '数量' : undefined}
                        name={[field.name, 'qty']}
                        rules={[{ required: true, message: '请输入数量' }]}
                      >
                        <InputNumber min={1} precision={0} style={{ width: '100%' }} placeholder="数量" />
                      </Form.Item>
                    </Col>
                    <Col xs={12} md={6} xl={6}>
                      <Form.Item
                        label={index === 0 ? '效期时间' : undefined}
                        name={[field.name, 'expire_at']}
                      >
                        <Input placeholder="可选，ISO 时间格式" />
                      </Form.Item>
                    </Col>
                    <Col xs={24} xl={2}>
                      <Button
                        danger
                        type="text"
                        icon={<DeleteOutlined />}
                        onClick={() => remove(field.name)}
                      >
                        删除
                      </Button>
                    </Col>
                  </Row>
                  {index < fields.length - 1 ? <Divider style={{ margin: '4px 0' }} /> : null}
                </div>
              ))}

              <Button
                type="dashed"
                icon={<PlusOutlined />}
                onClick={() => add({ component_sku_id: undefined, qty: 1, expire_at: null })}
              >
                新增组件
              </Button>
            </Flex>
          )}
        </Form.List>
      </SettingsPageSection>

      {footer ? <div>{footer}</div> : null}
    </Flex>
  </Form>
);
