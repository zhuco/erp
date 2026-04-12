import {
  Anchor,
  Button,
  Col,
  Form,
  Input,
  InputNumber,
  Radio,
  Row,
  Select,
  Space,
  Switch,
  Typography,
  message
} from 'antd';

import { UiModule07SettingsLayout, UiModule07SettingsSection } from '@/components/ui-modules';

import './salesSettingsPage.css';

const auditModeOptions = [
  { label: '自动审核', value: 'auto' },
  { label: '人工审核', value: 'manual' },
  { label: '按金额审核', value: 'amount' }
];

const currencyOptions = [
  { label: '人民币', value: 'CNY' },
  { label: '美元', value: 'USD' },
  { label: '港币', value: 'HKD' }
];

const mergeModeOptions = [
  { label: '同收货人合并', value: 'consignee' },
  { label: '同收货地址合并', value: 'address' },
  { label: '不自动合并', value: 'none' }
];

const exceptionTagOptions = [
  { label: '待人工处理', value: 'manual_review' },
  { label: '异常订单', value: 'exception' },
  { label: '需补录信息', value: 'info_required' }
];

const interceptReasonOptions = [
  { label: '地址异常', value: 'address_issue' },
  { label: '库存不足', value: 'stock_shortage' },
  { label: '付款异常', value: 'payment_issue' }
];

const addressStatusOptions = [
  { label: '待审核', value: 'pending_review' },
  { label: '待发货', value: 'pending_ship' },
  { label: '拦截中', value: 'intercepted' }
];

const addressConflictOptions = [
  { label: '提示运营处理', value: 'notify' },
  { label: '自动拆分订单', value: 'split' },
  { label: '保留原地址', value: 'keep' }
];

const remarkStrategyOptions = [
  { label: '追加到末尾', value: 'append' },
  { label: '覆盖原备注', value: 'replace' },
  { label: '只同步客服备注', value: 'service_only' }
];

const orderSourceOptions = [
  { label: '平台订单', value: 'platform' },
  { label: '手工录入', value: 'manual' },
  { label: '接口导入', value: 'api' }
];

const orderTagOptions = [
  { label: '渠道标签', value: 'channel' },
  { label: '优先级标签', value: 'priority' },
  { label: '客户等级标签', value: 'customer_level' }
];

const anchorItems = [
  { key: 'base', href: '#sales-settings-base', title: '基础设置' },
  { key: 'processing', href: '#sales-settings-processing', title: '订单处理' },
  { key: 'intercept', href: '#sales-settings-intercept', title: '平台订单拦截' },
  { key: 'address', href: '#sales-settings-address', title: '地址自动变更' },
  { key: 'remark', href: '#sales-settings-remark', title: '备注自动变更' },
  { key: 'source', href: '#sales-settings-source', title: '订单来源与标记' }
];

const initialValues = {
  autoCodeEnabled: true,
  auditMode: 'auto',
  auditThreshold: 5000,
  defaultCurrency: 'CNY',
  pricePrecision: 2,
  requiredRemark: false,
  autoReview: true,
  mergeMode: 'consignee',
  mergeWindow: 30,
  splitThreshold: 5,
  enableExceptionIntercept: true,
  exceptionTag: 'manual_review',
  platformIntercept: true,
  interceptReason: 'address_issue',
  interceptDuration: 24,
  interceptAutoResume: false,
  interceptNotePrefix: '平台拦截',
  addressAutoUpdate: true,
  addressStatus: 'pending_ship',
  addressConflictPolicy: 'notify',
  addressNotify: false,
  addressRemarkTemplate: '地址变更：{旧地址} → {新地址}',
  remarkAutoSync: true,
  remarkStrategy: 'append',
  remarkTag: 'priority',
  remarkReaudit: false,
  remarkTemplate: '客户留言：{remark}',
  orderSourceDefault: 'platform',
  orderTagMode: ['channel'],
  orderChannelSync: true,
  orderNotePrefix: '渠道：{channel}'
};

export const SalesSettingsPage = () => {
  const [form] = Form.useForm();
  const auditMode = Form.useWatch('auditMode', form);
  const getAnchorContainer = () =>
    (document.querySelector('.workbench-main-top') as HTMLElement) ?? document.body;

  const handleSave = () => {
    message.success('销售设置已保存（演示）');
  };

  return (
    <UiModule07SettingsLayout>
      <div className="sales-settings-wrapper">
        <Row gutter={12} className="sales-settings-page">
          <Col xs={24} xl={5} className="sales-settings-anchor">
            <div className="sales-settings-anchor-card">
              <Typography.Text type="secondary">设置目录</Typography.Text>
              <Anchor
                items={anchorItems}
                affix={false}
                className="sales-settings-anchor-list"
                getContainer={getAnchorContainer}
                targetOffset={88}
              />
            </div>
          </Col>
          <Col xs={24} xl={19} className="sales-settings-content">
            <Form form={form} layout="vertical" initialValues={initialValues} onFinish={handleSave}>
              <Space direction="vertical" size={12} style={{ width: '100%' }}>
              <div id="sales-settings-base" className="sales-settings-section">
                <UiModule07SettingsSection
                  title="基础设置"
                  description="配置销售单据的通用规则与展示方式。"
                  className="sales-settings-section-card"
                >
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="销售单自动编号"
                        name="autoCodeEnabled"
                        valuePropName="checked"
                        extra="关闭后可手工录入编号。"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="默认币种" name="defaultCurrency">
                        <Select options={currencyOptions} />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="价格保留小数" name="pricePrecision">
                        <InputNumber min={0} max={4} precision={0} style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Row gutter={16}>
                    <Col xs={24} md={12}>
                      <Form.Item label="销售单审核方式" name="auditMode">
                        <Radio.Group options={auditModeOptions} />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12}>
                      <Form.Item label="金额审核阈值（元）" name="auditThreshold">
                        <InputNumber
                          min={0}
                          precision={2}
                          style={{ width: '100%' }}
                          disabled={auditMode !== 'amount'}
                        />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Form.Item
                    label="强制填写备注字段"
                    name="requiredRemark"
                    valuePropName="checked"
                  >
                    <Switch />
                  </Form.Item>
                </UiModule07SettingsSection>
              </div>

              <div id="sales-settings-processing" className="sales-settings-section">
                <UiModule07SettingsSection
                  title="订单处理"
                  description="设置订单自动审核、合并与拆分策略。"
                  className="sales-settings-section-card"
                >
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="自动审核" name="autoReview" valuePropName="checked">
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="合并规则" name="mergeMode">
                        <Select options={mergeModeOptions} />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="合并窗口（分钟）" name="mergeWindow">
                        <InputNumber min={0} precision={0} style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="拆分阈值（件）" name="splitThreshold">
                        <InputNumber min={1} precision={0} style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="启用异常拦截"
                        name="enableExceptionIntercept"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="异常标签" name="exceptionTag">
                        <Select options={exceptionTagOptions} />
                      </Form.Item>
                    </Col>
                  </Row>
                </UiModule07SettingsSection>
              </div>

              <div id="sales-settings-intercept" className="sales-settings-section">
                <UiModule07SettingsSection
                  title="平台订单拦截"
                  description="控制平台异常订单的自动拦截与释放策略。"
                  className="sales-settings-section-card"
                >
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="启用平台订单拦截"
                        name="platformIntercept"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="拦截原因" name="interceptReason">
                        <Select options={interceptReasonOptions} />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="拦截时长（小时）" name="interceptDuration">
                        <InputNumber min={1} precision={0} style={{ width: '100%' }} />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="自动释放后继续审核"
                        name="interceptAutoResume"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={16}>
                      <Form.Item label="拦截备注前缀" name="interceptNotePrefix">
                        <Input placeholder="例如：平台拦截" />
                      </Form.Item>
                    </Col>
                  </Row>
                </UiModule07SettingsSection>
              </div>

              <div id="sales-settings-address" className="sales-settings-section">
                <UiModule07SettingsSection
                  title="地址自动变更"
                  description="当平台更新地址时的同步与冲突处理方式。"
                  className="sales-settings-section-card"
                >
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="地址变更自动同步"
                        name="addressAutoUpdate"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="同步后状态" name="addressStatus">
                        <Select options={addressStatusOptions} />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="冲突处理方式" name="addressConflictPolicy">
                        <Select options={addressConflictOptions} />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="触发短信提醒"
                        name="addressNotify"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={16}>
                      <Form.Item label="地址变更备注模板" name="addressRemarkTemplate">
                        <Input />
                      </Form.Item>
                    </Col>
                  </Row>
                </UiModule07SettingsSection>
              </div>

              <div id="sales-settings-remark" className="sales-settings-section">
                <UiModule07SettingsSection
                  title="备注自动变更"
                  description="管理平台备注同步与人工备注的冲突策略。"
                  className="sales-settings-section-card"
                >
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="启用备注同步"
                        name="remarkAutoSync"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="备注同步策略" name="remarkStrategy">
                        <Select options={remarkStrategyOptions} />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="备注标记" name="remarkTag">
                        <Select options={orderTagOptions} />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="备注更新后重新审核"
                        name="remarkReaudit"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={16}>
                      <Form.Item label="备注模板" name="remarkTemplate">
                        <Input.TextArea rows={2} />
                      </Form.Item>
                    </Col>
                  </Row>
                </UiModule07SettingsSection>
              </div>

              <div id="sales-settings-source" className="sales-settings-section">
                <UiModule07SettingsSection
                  title="订单来源与标记"
                  description="统一订单来源展示，并自动追加业务标签。"
                  className="sales-settings-section-card"
                >
                  <Row gutter={16}>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="默认订单来源" name="orderSourceDefault">
                        <Select options={orderSourceOptions} />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item
                        label="同步渠道标识"
                        name="orderChannelSync"
                        valuePropName="checked"
                      >
                        <Switch />
                      </Form.Item>
                    </Col>
                    <Col xs={24} md={12} xl={8}>
                      <Form.Item label="自动追加标签" name="orderTagMode">
                        <Select mode="multiple" options={orderTagOptions} />
                      </Form.Item>
                    </Col>
                  </Row>
                  <Form.Item label="订单备注前缀" name="orderNotePrefix">
                    <Input />
                  </Form.Item>
                </UiModule07SettingsSection>
              </div>
            </Space>

              <div className="sales-settings-actions">
                <Button type="primary" htmlType="submit" data-testid="sales-settings-save">
                  保存
                </Button>
                <Button onClick={() => form.resetFields()}>恢复默认</Button>
              </div>
            </Form>
          </Col>
        </Row>
      </div>
    </UiModule07SettingsLayout>
  );
};
