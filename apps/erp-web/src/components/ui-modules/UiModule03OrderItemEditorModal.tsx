import {
  Button,
  Checkbox,
  Col,
  Form,
  Input,
  InputNumber,
  Modal,
  Row,
  Select,
  Space,
  Table,
  Typography,
  message
} from 'antd';
import type { SelectProps } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { EllipsisOutlined } from '@ant-design/icons';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { getSkuDetail, searchSkus, type SkuSearchItem } from '@/api/skus';
import { useTenantId } from '@/features/tenant';

import { UiModule03SkuSelectorModal } from './UiModule03SkuSelectorModal';
import './UiModule03OrderItemEditorModal.css';

export type UiModule03OrderItemEditorValues = {
  sku_id?: string | null;
  sku_code?: string | null;
  title?: string | null;
  spec?: string | null;
  qty?: number;
  unit?: string | null;
  price?: number;
  discount_rate?: number;
  tax_rate?: number;
  remark?: string | null;
  supplier?: string | null;
  salesperson?: string | null;
  shop_order_id?: string | null;
  declared_value?: number | null;
  is_gift?: boolean;
  is_bundle?: boolean;
  keep_amount?: boolean;
  linked_item_ids?: string[];
};

export type UiModule03OrderItemRelation = {
  id: string;
  outer_sku_id?: string | null;
  platform_sku_id?: string | null;
  title?: string | null;
  spec?: string | null;
  deleted?: boolean | null;
};

type UiModule03OrderItemEditorModalProps = {
  open: boolean;
  mode: 'create' | 'edit';
  initialValues?: UiModule03OrderItemEditorValues;
  relatedItems?: UiModule03OrderItemRelation[];
  orderNo?: string;
  orderNoOptions?: SelectProps['options'];
  submitting?: boolean;
  onCancel: () => void;
  onSubmit: (values: UiModule03OrderItemEditorValues) => void;
};

const formatSkuSpec = (spec?: Record<string, unknown> | null) => {
  if (!spec) {
    return '';
  }
  return Object.entries(spec)
    .map(([key, value]) => `${key}:${String(value)}`)
    .join(' / ');
};

const normalizeUnitOptions = (values: Array<string | null | undefined>) => {
  const filtered = values.filter((item): item is string => Boolean(item));
  return filtered.length ? Array.from(new Set(filtered)) : ['件'];
};

export const UiModule03OrderItemEditorModal = ({
  open,
  mode,
  initialValues,
  relatedItems,
  orderNo,
  orderNoOptions,
  submitting,
  onCancel,
  onSubmit
}: UiModule03OrderItemEditorModalProps) => {
  const tenantId = useTenantId();
  const [form] = Form.useForm<UiModule03OrderItemEditorValues>();
  const [skuOptions, setSkuOptions] = useState<SkuSearchItem[]>([]);
  const [skuLoading, setSkuLoading] = useState(false);
  const [unitOptions, setUnitOptions] = useState<string[]>(['件']);
  const [skuSelectorOpen, setSkuSelectorOpen] = useState(false);
  const [linkedItemIds, setLinkedItemIds] = useState<string[]>([]);
  const [hideRemoved, setHideRemoved] = useState(false);

  const qty = Form.useWatch('qty', form) ?? 1;
  const price = Form.useWatch('price', form) ?? 0;
  const discountRate = Form.useWatch('discount_rate', form) ?? 100;
  const taxRate = Form.useWatch('tax_rate', form) ?? 0;

  const discountPrice = useMemo(() => price * (discountRate / 100), [discountRate, price]);
  const discountAmount = useMemo(() => Math.max(price - discountPrice, 0), [price, discountPrice]);
  const amount = useMemo(() => discountPrice * qty, [discountPrice, qty]);
  const taxAmount = useMemo(() => amount * (taxRate / 100), [amount, taxRate]);

  const skuSelectOptions = useMemo(
    () =>
      skuOptions.map((sku) => ({
        value: sku.sku_id,
        label: `${sku.sku_code} ${sku.name ?? ''}`.trim()
      })),
    [skuOptions]
  );

  const ensureSkuOption = useCallback((sku: SkuSearchItem) => {
    setSkuOptions((prev) => {
      if (prev.some((item) => item.sku_id === sku.sku_id)) {
        return prev;
      }
      return [sku, ...prev];
    });
  }, []);

  const applySkuSelection = useCallback(
    async (sku: SkuSearchItem, options?: { preservePrice?: boolean }) => {
      const resolvedTitle = sku.name ?? sku.sku_code ?? '';
      form.setFieldsValue({
        sku_id: sku.sku_id,
        sku_code: sku.sku_code,
        title: resolvedTitle
      });
      ensureSkuOption(sku);
      if (!tenantId) {
        return;
      }
      try {
        const detail = await getSkuDetail(tenantId, sku.sku_id);
        const nextUnits = normalizeUnitOptions([
          detail.product?.base_unit,
          detail.product?.second_unit
        ]);
        const currentUnit = form.getFieldValue('unit') ?? undefined;
        const nextUnit = nextUnits.includes(currentUnit ?? '') ? currentUnit : nextUnits[0];
        const nextValues: UiModule03OrderItemEditorValues = {
          spec: formatSkuSpec(detail.sku.spec),
          unit: nextUnit
        };
        if (!options?.preservePrice) {
          nextValues.price = detail.sku.cost_cent ? detail.sku.cost_cent / 100 : 0;
        }
        setUnitOptions(nextUnits);
        form.setFieldsValue(nextValues);
      } catch (err) {
        message.error(err instanceof Error ? err.message : '加载货品详情失败');
      }
    },
    [ensureSkuOption, form, tenantId]
  );

  const handleSkuSearch = useCallback(
    async (value: string) => {
      if (!tenantId) {
        return;
      }
      setSkuLoading(true);
      try {
        const response = await searchSkus(tenantId, value);
        setSkuOptions(response.items ?? []);
      } catch (err) {
        message.error(err instanceof Error ? err.message : '货品搜索失败');
      } finally {
        setSkuLoading(false);
      }
    },
    [tenantId]
  );

  useEffect(() => {
    if (!open) {
      return;
    }
    const defaults: UiModule03OrderItemEditorValues = {
      qty: 1,
      price: 0,
      discount_rate: 100,
      tax_rate: 0,
      ...initialValues
    };
    form.setFieldsValue(defaults);
    setLinkedItemIds(defaults.linked_item_ids ?? []);
    setUnitOptions(normalizeUnitOptions([defaults.unit]));
    if (defaults.sku_id) {
      void applySkuSelection(
        {
          sku_id: defaults.sku_id,
          sku_code: defaults.sku_code ?? defaults.sku_id,
          name: defaults.title ?? ''
        },
        { preservePrice: defaults.price != null }
      );
    }
  }, [applySkuSelection, form, initialValues, open]);

  useEffect(() => {
    if (!open) {
      form.resetFields();
      setSkuOptions([]);
      setUnitOptions(['件']);
      setLinkedItemIds([]);
      setHideRemoved(false);
    }
  }, [form, open]);

  const relatedColumns = useMemo<ColumnsType<UiModule03OrderItemRelation>>(
    () => [
      {
        title: '网店主订单号',
        dataIndex: 'order_no',
        width: 180,
        render: () => orderNo ?? '-'
      },
      {
        title: '网店规格编号',
        dataIndex: 'outer_sku_id',
        width: 160,
        render: (_, record) => record.outer_sku_id ?? record.platform_sku_id ?? '-'
      },
      {
        title: '交易名称',
        dataIndex: 'title',
        width: 220,
        render: (value) => value ?? '-'
      },
      {
        title: '交易规格',
        dataIndex: 'spec',
        render: (value, record) =>
          value ?? record.platform_sku_id ?? record.outer_sku_id ?? '-'
      }
    ],
    [orderNo]
  );

  const filteredRelatedItems = useMemo(() => {
    if (!hideRemoved) {
      return relatedItems ?? [];
    }
    return (relatedItems ?? []).filter((item) => !item.deleted);
  }, [hideRemoved, relatedItems]);

  const handleOk = useCallback(async () => {
    try {
      const values = await form.validateFields();
      onSubmit({ ...values, linked_item_ids: linkedItemIds });
    } catch {
      return;
    }
  }, [form, linkedItemIds, onSubmit]);

  return (
    <>
      <Modal
        title={mode === 'create' ? '新增销售单货品' : '编辑销售单货品'}
        open={open}
        onCancel={onCancel}
        onOk={handleOk}
        okText="保存"
        cancelText="取消"
        width={920}
        className="order-review-item-editor-modal"
        confirmLoading={submitting}
        destroyOnClose
      >
        <Form layout="vertical" form={form} className="order-review-item-editor-form">
          <Form.Item name="sku_code" hidden>
            <Input />
          </Form.Item>
          <Row gutter={[16, 8]}>
            <Col span={12}>
              <Form.Item label="货品编号" required>
                <Space.Compact style={{ width: '100%' }}>
                  <Form.Item
                    name="sku_id"
                    noStyle
                    rules={[{ required: true, message: '请选择货品编号' }]}
                  >
                    <Select
                      showSearch
                      allowClear
                      placeholder="可直接输入/查询"
                      options={skuSelectOptions}
                      optionFilterProp="label"
                      filterOption={false}
                      loading={skuLoading}
                      onSearch={handleSkuSearch}
                      onFocus={() => {
                        if (!skuOptions.length) {
                          void handleSkuSearch('');
                        }
                      }}
                      onChange={(value) => {
                        if (!value) {
                          form.setFieldsValue({
                            sku_id: null,
                            sku_code: null,
                            title: '',
                            spec: '',
                            unit: '件',
                            price: 0
                          });
                          setUnitOptions(['件']);
                          return;
                        }
                        const selected = skuOptions.find((item) => item.sku_id === value);
                        if (selected) {
                          void applySkuSelection(selected);
                        }
                      }}
                    />
                  </Form.Item>
                  <Button
                    icon={<EllipsisOutlined />}
                    aria-label="Open SKU selector"
                    onClick={() => setSkuSelectorOpen(true)}
                  />
                </Space.Compact>
              </Form.Item>
              <Form.Item label="规格" name="spec">
                <Input disabled placeholder="自动加载规格" />
              </Form.Item>
              <Form.Item label="单价" name="price">
                <InputNumber min={0} precision={4} style={{ width: '100%' }} />
              </Form.Item>
              <Form.Item label="折后单价">
                <InputNumber
                  min={0}
                  precision={4}
                  style={{ width: '100%' }}
                  value={Number.isFinite(discountPrice) ? discountPrice : 0}
                  disabled
                />
              </Form.Item>
              <Form.Item label="税额(税率)">
                <Space.Compact style={{ width: '100%' }}>
                  <InputNumber
                    min={0}
                    precision={4}
                    style={{ width: '50%' }}
                    value={Number.isFinite(taxAmount) ? taxAmount : 0}
                    disabled
                  />
                  <Form.Item name="tax_rate" noStyle>
                    <InputNumber min={0} max={100} precision={2} style={{ width: '50%' }} />
                  </Form.Item>
                  <span className="order-review-item-editor-unit">%</span>
                </Space.Compact>
              </Form.Item>
              <Form.Item label="备注" name="remark">
                <Input placeholder="填写备注" />
              </Form.Item>
              <Form.Item label="供应商" name="supplier">
                <Input placeholder="请选择供应商" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item label="货品名称" name="title">
                <Input disabled placeholder="自动加载名称" />
              </Form.Item>
              <Form.Item label="数量/单位">
                <Space.Compact style={{ width: '100%' }}>
                  <Form.Item name="qty" noStyle>
                    <InputNumber min={1} precision={0} style={{ width: '50%' }} />
                  </Form.Item>
                  <Form.Item name="unit" noStyle>
                    <Select
                      placeholder="请选择货品单位"
                      options={unitOptions.map((unit) => ({
                        value: unit,
                        label: unit
                      }))}
                    />
                  </Form.Item>
                </Space.Compact>
              </Form.Item>
              <Form.Item label="优惠(折扣)">
                <Space.Compact style={{ width: '100%' }}>
                  <InputNumber
                    min={0}
                    precision={4}
                    style={{ width: '50%' }}
                    value={Number.isFinite(discountAmount) ? discountAmount : 0}
                    disabled
                  />
                  <Form.Item name="discount_rate" noStyle>
                    <InputNumber min={0} max={100} precision={2} style={{ width: '50%' }} />
                  </Form.Item>
                  <span className="order-review-item-editor-unit">%</span>
                </Space.Compact>
              </Form.Item>
              <Form.Item label="金额">
                <InputNumber
                  min={0}
                  precision={4}
                  style={{ width: '100%' }}
                  value={Number.isFinite(amount) ? amount : 0}
                  disabled
                />
              </Form.Item>
              <Form.Item label="业务员" name="salesperson">
                <Input placeholder="请输入姓名、工号、手机" />
              </Form.Item>
              <Form.Item label="网店主订单号" name="shop_order_id">
                <Select
                  allowClear
                  placeholder="选择货品需要关联的网店订单号"
                  options={orderNoOptions}
                />
              </Form.Item>
              <Form.Item label="申报价值" name="declared_value">
                <InputNumber min={0} precision={2} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
          <Row gutter={[16, 8]}>
            <Col span={24}>
              <Space size={24}>
                <Form.Item name="is_gift" valuePropName="checked" noStyle>
                  <Checkbox>赠品</Checkbox>
                </Form.Item>
                <Form.Item name="is_bundle" valuePropName="checked" noStyle>
                  <Checkbox>合单</Checkbox>
                </Form.Item>
                {mode === 'edit' ? (
                  <Form.Item name="keep_amount" valuePropName="checked" noStyle>
                    <Checkbox>货品金额不变</Checkbox>
                  </Form.Item>
                ) : null}
              </Space>
            </Col>
          </Row>
          <div className="order-review-item-editor-related">
            <div className="order-review-item-editor-related-header">
              <Typography.Text>选择货品需要关联的网店子订单货品</Typography.Text>
              <Checkbox checked={hideRemoved} onChange={(event) => setHideRemoved(event.target.checked)}>
                隐藏已删除的货品
              </Checkbox>
            </div>
            <Table
              rowKey="id"
              size="small"
              columns={relatedColumns}
              dataSource={filteredRelatedItems}
              pagination={false}
              rowSelection={{
                selectedRowKeys: linkedItemIds,
                onChange: (keys) => setLinkedItemIds(keys as string[])
              }}
              locale={{ emptyText: '暂无关联货品' }}
            />
          </div>
        </Form>
      </Modal>
      <UiModule03SkuSelectorModal
        open={skuSelectorOpen}
        onCancel={() => setSkuSelectorOpen(false)}
        onConfirm={(sku) => {
          setSkuSelectorOpen(false);
          void applySkuSelection(sku);
        }}
      />
    </>
  );
};
