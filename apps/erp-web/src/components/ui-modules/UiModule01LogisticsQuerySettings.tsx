import { Button, Card, Col, Form, Input, Row, Space, Typography } from 'antd';
import type { FormInstance } from 'antd';
import type { ReactNode } from 'react';

export type UiModule01LogisticsQueryValues = {
  tracking_no?: string;
};

type UiModule01LogisticsQuerySettingsProps = {
  form: FormInstance<UiModule01LogisticsQueryValues>;
  onSearch: (values: UiModule01LogisticsQueryValues) => void;
  onReset: () => void;
  title?: ReactNode;
  trackingNoLabel?: ReactNode;
  trackingNoPlaceholder?: string;
  searchLabel?: ReactNode;
  resetLabel?: ReactNode;
};

export const UiModule01LogisticsQuerySettings = ({
  form,
  onSearch,
  onReset,
  title = '物流查询',
  trackingNoLabel = '运单号',
  trackingNoPlaceholder = '输入运单号',
  searchLabel = '查询',
  resetLabel = '重置'
}: UiModule01LogisticsQuerySettingsProps) => (
  <Space direction="vertical" size={16} style={{ width: '100%' }}>
    <Typography.Title level={3}>{title}</Typography.Title>
    <Card>
      <Form form={form} layout="vertical" onFinish={onSearch}>
        <Row gutter={16}>
          <Col xs={24} md={8}>
            <Form.Item name="tracking_no" label={trackingNoLabel}>
              <Input allowClear placeholder={trackingNoPlaceholder} />
            </Form.Item>
          </Col>
          <Col xs={24} md={16} style={{ display: 'flex', alignItems: 'end' }}>
            <Space>
              <Button type="primary" htmlType="submit">
                {searchLabel}
              </Button>
              <Button onClick={onReset}>{resetLabel}</Button>
            </Space>
          </Col>
        </Row>
      </Form>
    </Card>
  </Space>
);
