import {
  Anchor,
  Button,
  Col,
  Divider,
  Form,
  Input,
  InputNumber,
  Radio,
  Row,
  Select,
  Space,
  Switch,
  Tag,
  Typography,
  message
} from 'antd';
import {
  AppstoreOutlined,
  ControlOutlined,
  FlagOutlined,
  SafetyCertificateOutlined,
  SettingOutlined,
  SyncOutlined
} from '@ant-design/icons';
import { useEffect, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import {
  loadSalesStrategies,
  salesStrategyUpdatedEventName,
  type SalesStrategy
} from '@/features/salesStrategies';
import { SettingsPageSection, SettingsPageTemplate } from '@/components/page-templates';

import { SalesStrategySettingsPage } from './SalesStrategySettingsPage';
import {
  buildSalesSettingsSectionStatuses,
  buildStrategyCategoryCards,
  type SalesSettingsFormValues
} from './salesSettingsWorkbenchModel';

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

const initialValues: SalesSettingsFormValues = {
  autoCodeEnabled: true,
  auditMode: 'auto',
  auditThreshold: 5000,
  defaultCurrency: 'CNY',
  pricePrecision: 2,
  requiredRemark: false,
  autoReview: true,
  enableOrderRecheck: true,
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

const sectionIdMap = {
  base: 'sales-settings-base',
  processing: 'sales-settings-processing',
  intercept: 'sales-settings-intercept',
  sync: 'sales-settings-sync',
  source: 'sales-settings-source',
  'strategy-center': 'sales-settings-strategy-center'
} as const;

const scrollToSection = (sectionId: string) => {
  const target = document.getElementById(sectionId);
  if (!target) {
    return;
  }
  target.scrollIntoView({ behavior: 'smooth', block: 'start' });
};

const iconMap = {
  base: <SettingOutlined />,
  processing: <ControlOutlined />,
  intercept: <SafetyCertificateOutlined />,
  sync: <SyncOutlined />,
  source: <FlagOutlined />,
  'strategy-center': <AppstoreOutlined />
};

type SalesSettingsPageProps = {
  initialSection?: keyof typeof sectionIdMap;
};

export const SalesSettingsPage = ({
  initialSection = 'base'
}: SalesSettingsPageProps) => {
  const [form] = Form.useForm<SalesSettingsFormValues>();
  const auditMode = Form.useWatch('auditMode', form);
  const watchedValues = Form.useWatch([], form);
  const values = { ...initialValues, ...(watchedValues ?? {}) };
  const [strategies, setStrategies] = useState<SalesStrategy[]>(() => loadSalesStrategies());
  const [searchParams, setSearchParams] = useSearchParams();
  const strategyCards = buildStrategyCategoryCards(strategies);
  const sectionStatuses = buildSalesSettingsSectionStatuses(values, strategies);
  const anchorItems = sectionStatuses.map((section) => ({
    key: section.key,
    href: `#${sectionIdMap[section.key]}`,
    title: (
      <div className="sales-settings-anchor-item">
        <span className="sales-settings-anchor-item-title">{section.title}</span>
        <span className="sales-settings-anchor-item-status">{section.status}</span>
        <span className="sales-settings-anchor-item-detail">{section.detail}</span>
      </div>
    )
  }));

  useEffect(() => {
    const syncStrategies = () => {
      setStrategies(loadSalesStrategies());
    };
    syncStrategies();
    window.addEventListener(salesStrategyUpdatedEventName, syncStrategies);
    return () => window.removeEventListener(salesStrategyUpdatedEventName, syncStrategies);
  }, []);

  useEffect(() => {
    const hash = window.location.hash?.replace('#', '');
    const targetId = hash || sectionIdMap[initialSection];
    const timer = window.setTimeout(() => {
      scrollToSection(targetId);
    }, 80);
    return () => window.clearTimeout(timer);
  }, [initialSection]);

  const handleSave = () => {
    message.success('销售设置工作台配置已保存（演示）');
  };

  const handleRestore = () => {
    form.resetFields();
    message.success('已恢复为默认展示配置');
  };

  const handleStrategyCardClick = (key: string) => {
    const nextSearchParams = new URLSearchParams(searchParams);
    nextSearchParams.set('tab', key);
    setSearchParams(nextSearchParams);
    scrollToSection(sectionIdMap['strategy-center']);
  };

  const getAnchorContainer = () =>
    (document.querySelector('.workbench-main-top') as HTMLElement) ?? document.body;

  return (
    <SettingsPageTemplate>
      <div className="sales-settings-workbench">
        <Form form={form} layout="vertical" initialValues={initialValues} onFinish={handleSave}>
          <div className="sales-settings-page-header">
            <div className="sales-settings-page-header-main">
              <Typography.Title level={2} className="sales-settings-page-title">
                销售设置工作台
              </Typography.Title>
              <Typography.Paragraph className="sales-settings-page-description">
                统一管理订单治理、平台风险、同步规则与策略执行。
              </Typography.Paragraph>
            </div>
            <div className="sales-settings-page-actions">
              <Button type="primary" htmlType="submit" data-testid="sales-settings-save">
                保存变更
              </Button>
              <Button onClick={handleRestore}>恢复默认</Button>
            </div>
          </div>

          <div className="sales-settings-body">
            <aside className="sales-settings-sidebar">
              <div className="sales-settings-anchor-card">
                <Typography.Text className="sales-settings-sidebar-title">
                  分组目录
                </Typography.Text>
                <Anchor
                  items={anchorItems}
                  affix={false}
                  className="sales-settings-anchor-list"
                  getContainer={getAnchorContainer}
                  targetOffset={88}
                />
              </div>
            </aside>

            <div className="sales-settings-content">
              <Space direction="vertical" size={16} style={{ width: '100%' }}>
                <div id={sectionIdMap.base} className="sales-settings-section">
                  <SettingsPageSection
                    title="基础设置"
                    description="定义销售单据的基础口径、审核方式和录入标准。"
                    className="sales-settings-section-card"
                    actions={<Tag color="blue">{sectionStatuses[0].status}</Tag>}
                  >
                    <div className="sales-settings-summary-strip">
                      <div className="sales-settings-summary-lead">
                        <span className="sales-settings-summary-icon">{iconMap.base}</span>
                        <div>
                          <div className="sales-settings-summary-title">基础规则摘要</div>
                          <div className="sales-settings-summary-description">
                            {sectionStatuses[0].detail}
                          </div>
                        </div>
                      </div>
                      <div className="sales-settings-summary-tags">
                        <Tag>{values.autoCodeEnabled ? '自动编号' : '手工编号'}</Tag>
                        <Tag>{values.requiredRemark ? '备注必填' : '备注选填'}</Tag>
                        <Tag>{values.defaultCurrency}</Tag>
                      </div>
                    </div>

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
                  </SettingsPageSection>
                </div>

                <div id={sectionIdMap.processing} className="sales-settings-section">
                  <SettingsPageSection
                    title="自动处理"
                    description="集中控制自动审核、合单、拆单与异常拦截的处理节奏。"
                    className="sales-settings-section-card"
                    actions={<Tag color="green">{sectionStatuses[1].status}</Tag>}
                  >
                    <div className="sales-settings-summary-strip">
                      <div className="sales-settings-summary-lead">
                        <span className="sales-settings-summary-icon">{iconMap.processing}</span>
                        <div>
                          <div className="sales-settings-summary-title">自动处理摘要</div>
                          <div className="sales-settings-summary-description">
                            {sectionStatuses[1].detail}
                          </div>
                        </div>
                      </div>
                      <div className="sales-settings-summary-tags">
                        <Tag>{values.enableOrderRecheck ? '订单复核启用' : '审核后直递 WMS'}</Tag>
                        <Tag>{values.enableExceptionIntercept ? '异常拦截启用' : '异常拦截关闭'}</Tag>
                        <Tag>{`拆分阈值 ${values.splitThreshold} 件`}</Tag>
                      </div>
                    </div>

                    <Row gutter={16}>
                      <Col xs={24} md={12} xl={8}>
                        <Form.Item label="自动审核" name="autoReview" valuePropName="checked">
                          <Switch />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12} xl={8}>
                        <Form.Item
                          label="是否启用订单复核"
                          name="enableOrderRecheck"
                          valuePropName="checked"
                        >
                          <Switch />
                        </Form.Item>
                      </Col>
                      <Col xs={24} md={12} xl={8}>
                        <Form.Item label="合并规则" name="mergeMode">
                          <Select options={mergeModeOptions} />
                        </Form.Item>
                      </Col>
                    </Row>
                    <Row gutter={16}>
                      <Col xs={24} md={12} xl={8}>
                        <Form.Item label="合并窗口（分钟）" name="mergeWindow">
                          <InputNumber min={0} precision={0} style={{ width: '100%' }} />
                        </Form.Item>
                      </Col>
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
                  </SettingsPageSection>
                </div>

                <div id={sectionIdMap.intercept} className="sales-settings-section">
                  <SettingsPageSection
                    title="平台拦截"
                    description="独立展示平台订单风控策略，突出高风险治理能力。"
                    className="sales-settings-section-card"
                    actions={<Tag color="gold">{sectionStatuses[2].status}</Tag>}
                  >
                    <div className="sales-settings-summary-strip sales-settings-summary-strip--warning">
                      <div className="sales-settings-summary-lead">
                        <span className="sales-settings-summary-icon">{iconMap.intercept}</span>
                        <div>
                          <div className="sales-settings-summary-title">平台风控摘要</div>
                          <div className="sales-settings-summary-description">
                            {sectionStatuses[2].detail}
                          </div>
                        </div>
                      </div>
                      <div className="sales-settings-summary-tags">
                        <Tag color={values.platformIntercept ? 'gold' : 'default'}>
                          {values.platformIntercept ? '风控开启' : '风控关闭'}
                        </Tag>
                        <Tag>{values.interceptAutoResume ? '自动释放后继续审核' : '释放后需人工确认'}</Tag>
                      </div>
                    </div>

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
                  </SettingsPageSection>
                </div>

                <div id={sectionIdMap.sync} className="sales-settings-section">
                  <SettingsPageSection
                    title="地址与备注同步"
                    description="统一管理平台变更同步、冲突处理和人工复核口径。"
                    className="sales-settings-section-card"
                    actions={<Tag color="cyan">{sectionStatuses[3].status}</Tag>}
                  >
                    <div className="sales-settings-summary-strip">
                      <div className="sales-settings-summary-lead">
                        <span className="sales-settings-summary-icon">{iconMap.sync}</span>
                        <div>
                          <div className="sales-settings-summary-title">同步治理摘要</div>
                          <div className="sales-settings-summary-description">
                            {sectionStatuses[3].detail}
                          </div>
                        </div>
                      </div>
                      <div className="sales-settings-summary-tags">
                        <Tag>{values.addressAutoUpdate ? '地址自动同步' : '地址人工处理'}</Tag>
                        <Tag>{values.remarkAutoSync ? '备注自动同步' : '备注人工处理'}</Tag>
                      </div>
                    </div>

                    <div className="sales-settings-subsection">
                      <div className="sales-settings-subsection-header">
                        <Typography.Text strong>地址同步规则</Typography.Text>
                        <Typography.Text type="secondary">
                          当平台地址变更时，控制订单状态与冲突处理。
                        </Typography.Text>
                      </div>
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
                    </div>

                    <Divider className="sales-settings-divider" />

                    <div className="sales-settings-subsection">
                      <div className="sales-settings-subsection-header">
                        <Typography.Text strong>备注同步规则</Typography.Text>
                        <Typography.Text type="secondary">
                          管理平台备注与人工备注的冲突优先级。
                        </Typography.Text>
                      </div>
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
                    </div>
                  </SettingsPageSection>
                </div>

                <div id={sectionIdMap.source} className="sales-settings-section">
                  <SettingsPageSection
                    title="来源与标签"
                    description="统一订单来源、渠道标识与业务标签口径。"
                    className="sales-settings-section-card"
                    actions={<Tag>{sectionStatuses[4].status}</Tag>}
                  >
                    <div className="sales-settings-summary-strip">
                      <div className="sales-settings-summary-lead">
                        <span className="sales-settings-summary-icon">{iconMap.source}</span>
                        <div>
                          <div className="sales-settings-summary-title">标识治理摘要</div>
                          <div className="sales-settings-summary-description">
                            {sectionStatuses[4].detail}
                          </div>
                        </div>
                      </div>
                      <div className="sales-settings-summary-tags">
                        <Tag>{`自动标签 ${values.orderTagMode.length} 项`}</Tag>
                        <Tag>{values.orderChannelSync ? '渠道同步开启' : '渠道同步关闭'}</Tag>
                      </div>
                    </div>

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
                  </SettingsPageSection>
                </div>

                <div id={sectionIdMap['strategy-center']} className="sales-settings-section">
                  <SettingsPageSection
                    title="策略中心"
                    description="在同一工作台内集中管理销售策略，实现概览与执行配置一体化。"
                    className="sales-settings-section-card sales-settings-section-card--strategy"
                    actions={<Tag color="purple">{sectionStatuses[5].status}</Tag>}
                  >
                    <div className="sales-settings-summary-strip sales-settings-summary-strip--strategy">
                      <div className="sales-settings-summary-lead">
                        <span className="sales-settings-summary-icon">{iconMap['strategy-center']}</span>
                        <div>
                          <div className="sales-settings-summary-title">策略覆盖摘要</div>
                          <div className="sales-settings-summary-description">
                            {sectionStatuses[5].detail}
                          </div>
                        </div>
                      </div>
                      <div className="sales-settings-summary-tags">
                        <Tag color="blue">{`${strategies.filter((item) => item.enabled).length} 条生效`}</Tag>
                        <Tag>{`${strategies.length} 条总策略`}</Tag>
                      </div>
                    </div>

                    <div className="sales-settings-strategy-card-grid">
                      {strategyCards.map((card) => (
                        <button
                          key={card.key}
                          type="button"
                          className="sales-settings-strategy-card"
                          onClick={() => handleStrategyCardClick(card.key)}
                        >
                          <span className="sales-settings-strategy-card-label">{card.label}</span>
                          <strong className="sales-settings-strategy-card-count">{card.total}</strong>
                          <span className="sales-settings-strategy-card-summary">{card.summary}</span>
                        </button>
                      ))}
                    </div>

                    <div className="sales-settings-strategy-panel-shell">
                      <SalesStrategySettingsPage />
                    </div>
                  </SettingsPageSection>
                </div>
              </Space>
            </div>
          </div>
        </Form>
      </div>
    </SettingsPageTemplate>
  );
};
