import { Card, Col, Form, InputNumber, Row, Select, Switch, Typography } from 'antd';
import type { ReactNode } from 'react';

import {
  apparelSizeSystemOptions,
  serialTrackingModeOptions,
  shelfLifeUnitOptions,
  type IndustryProfile,
} from '@/pages/products/productIndustry';

type ProductIndustrySectionsProps = {
  industryProfile: IndustryProfile;
  batchEnabled: boolean;
  serialEnabled: boolean;
  apparelComposer?: ReactNode;
};

export const ProductIndustrySections = ({
  industryProfile,
  batchEnabled,
  serialEnabled,
  apparelComposer,
}: ProductIndustrySectionsProps) => (
  <>
    {industryProfile === 'FOOD' ? (
      <Card
        size="small"
        title="食品效期管理"
        style={{ marginBottom: 8 }}
        data-testid="products-food-section"
      >
        <Row gutter={[8, 2]}>
          <Col span={6}>
            <Form.Item label="生产日期管理" name="production_date_enabled" valuePropName="checked">
              <Switch checkedChildren="启用" unCheckedChildren="停用" />
            </Form.Item>
          </Col>
          <Col span={6}>
            <Form.Item label="保质期单位" name="shelf_life_unit">
              <Select options={shelfLifeUnitOptions} disabled={!batchEnabled} />
            </Form.Item>
          </Col>
          <Col span={6}>
            <Form.Item label="临期预警天数" name="near_expiry_warning_days">
              <InputNumber min={0} disabled={!batchEnabled} style={{ width: '100%' }} />
            </Form.Item>
          </Col>
          <Col span={6}>
            <Typography.Text type="secondary">
              食品模板建议启用批次管理，并按生产日期驱动保质期、临期预警与禁售提前策略。
            </Typography.Text>
          </Col>
        </Row>
      </Card>
    ) : null}
    {industryProfile === 'APPAREL' ? (
      <>
        <Card
          size="small"
          title="鞋服规格矩阵"
          style={{ marginBottom: 8 }}
          data-testid="products-apparel-section"
        >
          <Row gutter={[8, 2]}>
            <Col span={6}>
              <Form.Item label="尺码体系" name="apparel_size_system">
                <Select options={apparelSizeSystemOptions} />
              </Form.Item>
            </Col>
            <Col span={6}>
              <Form.Item label="规格模式" name="variant_mode">
                <Select options={[{ value: 'APPAREL_MATRIX', label: '颜色 / 尺码矩阵' }]} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Typography.Text type="secondary" data-testid="products-apparel-spec-hint">
                鞋服规格默认使用颜色 / 尺码二维矩阵，规格名称可按颜色 / 尺码自动预览。
              </Typography.Text>
            </Col>
          </Row>
        </Card>
        {apparelComposer}
      </>
    ) : null}
    {industryProfile === 'DIGITAL' ? (
      <Card
        size="small"
        title="设备序列号管理"
        style={{ marginBottom: 8 }}
        data-testid="products-digital-section"
      >
        <Row gutter={[8, 2]}>
          <Col span={6}>
            <Form.Item label="序列号追踪模式" name="serial_tracking_mode">
              <Select options={serialTrackingModeOptions} disabled={!serialEnabled} />
            </Form.Item>
          </Col>
          <Col span={6}>
            <Form.Item
              label="出库绑定序列号"
              name="serial_outbound_required"
              valuePropName="checked"
            >
              <Switch checkedChildren="必需" unCheckedChildren="可选" disabled={!serialEnabled} />
            </Form.Item>
          </Col>
          <Col span={6}>
            <Form.Item
              label="入库唯一校验"
              name="serial_inbound_unique_required"
              valuePropName="checked"
            >
              <Switch checkedChildren="校验" unCheckedChildren="关闭" disabled={!serialEnabled} />
            </Form.Item>
          </Col>
          <Col span={6}>
            <Typography.Text type="secondary">
              数码设备建议启用唯一码管理，并在入库唯一校验、出库绑定与平台回传之间形成闭环。
            </Typography.Text>
          </Col>
        </Row>
      </Card>
    ) : null}
  </>
);
