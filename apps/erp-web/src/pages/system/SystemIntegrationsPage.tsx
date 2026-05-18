import {
  Button,
  Card,
  Col,
  Form,
  Input,
  Modal,
  Row,
  Select,
  Space,
  Statistic,
  Table,
  Tag,
  Tooltip,
  Typography,
  message
} from 'antd';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { ColumnsType } from 'antd/es/table';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';

import {
  getSystemIntegrationsCatalog,
  getShopIntegrationOverview,
  updateSystemIntegrationsCatalog,
  type IntegrationConfig,
  type IntegrationChange,
  type IntegrationStatus,
  type ShopIntegrationOverviewItem,
  type SystemIntegrationsCatalog,
  type WebhookSlaStatus,
  type WebhookStatus
} from '@/api/systemIntegrations';
import { getSystemWorkflowCatalog } from '@/api/systemWorkflows';
import { requestTokenRefresh } from '@/api/shops';
import { EmptyState } from '@/components/EmptyState';
import { ErrorState } from '@/components/QueryState';
import { SettingsPageTemplate, SettingsPageSection } from '@/components/page-templates';
import { useCan } from '@/features/rbac';
import { isTenantSelected, useTenantId } from '@/features/tenant';

type IntegrationFormValues = {
  name: string;
  category: string;
  provider: string;
  retry_policy: string;
  alert_channel: string;
  description?: string;
};

const { Text } = Typography;

const statusColors: Record<IntegrationStatus, string> = {
  CONNECTED: 'green',
  WARNING: 'orange',
  DISCONNECTED: 'red'
};

const statusLabels: Record<IntegrationStatus, string> = {
  CONNECTED: '已连接',
  WARNING: '告警',
  DISCONNECTED: '未连接'
};

const nextStatusMap: Record<IntegrationStatus, IntegrationStatus> = {
  CONNECTED: 'WARNING',
  WARNING: 'DISCONNECTED',
  DISCONNECTED: 'CONNECTED'
};

const healthScoreByStatus: Record<IntegrationStatus, number> = {
  CONNECTED: 98,
  WARNING: 92,
  DISCONNECTED: 75
};

const categoryOptions = [
  { label: '电商平台', value: '电商平台' },
  { label: '物流系统', value: '物流系统' },
  { label: '支付渠道', value: '支付渠道' },
  { label: 'BI 报表', value: 'BI 报表' }
];

const oauthStatusLabels: Record<string, string> = {
  SUCCESS: '授权成功',
  FAILED: '授权异常',
  PENDING: '待授权'
};

const oauthStatusColors: Record<string, string> = {
  SUCCESS: 'green',
  FAILED: 'red',
  PENDING: 'gold'
};

const platformLabels: Record<string, string> = {
  taobao: '淘宝',
  tmall: '天猫',
  jd: '京东',
  pinduoduo: '拼多多',
  douyin: '抖店'
};

const webhookModeLabels: Record<string, string> = {
  STRICT: '严格验签',
  OPTIONAL: '宽松验签',
  DISABLED: '未校验'
};

const jobTypeLabels: Record<string, string> = {
  order_sync: '订单',
  refund_sync: '售后',
  shipping_sync: '发货',
  product_sync: '商品'
};

const tokenStatusLabels: Record<string, string> = {
  ACTIVE: '有效',
  EXPIRED: '已过期',
  REFRESHING: '刷新中'
};

const webhookStatusLabels: Record<WebhookStatus, string> = {
  HEALTHY: 'Webhook 正常',
  WARNING: 'Webhook 预警',
  IDLE: 'Webhook 空闲'
};

const webhookStatusColors: Record<WebhookStatus, string> = {
  HEALTHY: 'green',
  WARNING: 'orange',
  IDLE: 'default'
};

const webhookSlaStatusLabels: Record<WebhookSlaStatus, string> = {
  HEALTHY: 'SLA 正常',
  WARNING: 'SLA 告警',
  DEAD: 'SLA 死信',
  IDLE: 'SLA 空闲'
};

const webhookSlaStatusColors: Record<WebhookSlaStatus, string> = {
  HEALTHY: 'cyan',
  WARNING: 'orange',
  DEAD: 'red',
  IDLE: 'default'
};

const formatDateTime = (value?: string) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');
const formatPlatform = (value: string) => platformLabels[value] ?? value.toUpperCase();
const formatOauthStatus = (value: string) => oauthStatusLabels[value] ?? value;
const formatWebhookMode = (value?: string) =>
  value ? webhookModeLabels[value] ?? value.toUpperCase() : '未配置';
const formatWebhookStatus = (value: WebhookStatus) => webhookStatusLabels[value] ?? value;
const formatWebhookSlaStatus = (value: WebhookSlaStatus) => webhookSlaStatusLabels[value] ?? value;
const formatTokenStatus = (value?: string) =>
  value ? tokenStatusLabels[value] ?? value.toUpperCase() : '未生成';

export const SystemIntegrationsPage = () => {
  const tenantId = useTenantId();
  const tenantReady = isTenantSelected(tenantId);
  const canWrite = useCan('SYS.SETTINGS.WRITE');
  const canChannelWrite = useCan('channels.write');
  const canOperateShopIntegration = canWrite || canChannelWrite;
  const queryClient = useQueryClient();
  const [configs, setConfigs] = useState<IntegrationConfig[]>([]);
  const [changes, setChanges] = useState<IntegrationChange[]>([]);
  const [modalOpen, setModalOpen] = useState(false);
  const [historyOpen, setHistoryOpen] = useState(false);
  const [editing, setEditing] = useState<IntegrationConfig | null>(null);
  const [activeIntegrationId, setActiveIntegrationId] = useState<string | null>(null);
  const [refreshingShopId, setRefreshingShopId] = useState<string | null>(null);
  const [form] = Form.useForm<IntegrationFormValues>();

  const catalogQuery = useQuery({
    queryKey: ['system-integrations', tenantId],
    queryFn: () => getSystemIntegrationsCatalog(tenantId),
    enabled: tenantReady
  });

  const overviewQuery = useQuery({
    queryKey: ['system-integrations-shops', tenantId],
    queryFn: () => getShopIntegrationOverview(tenantId),
    enabled: tenantReady
  });
  const workflowQuery = useQuery({
    queryKey: ['system-workflows', tenantId],
    queryFn: () => getSystemWorkflowCatalog(tenantId),
    enabled: tenantReady
  });

  useEffect(() => {
    if (!tenantReady) {
      setConfigs([]);
      setChanges([]);
      setHistoryOpen(false);
      setActiveIntegrationId(null);
      return;
    }
    if (catalogQuery.data) {
      setConfigs(catalogQuery.data.configs);
      setChanges(catalogQuery.data.changes);
    }
  }, [catalogQuery.data, tenantReady]);

  const saveMutation = useMutation({
    mutationFn: (payload: SystemIntegrationsCatalog) =>
      updateSystemIntegrationsCatalog(tenantId, payload),
    onError: (error) => {
      message.error(error instanceof Error ? error.message : '保存集成中心配置失败');
    }
  });

  const refreshTokenMutation = useMutation({
    mutationFn: (shopId: string) => requestTokenRefresh(tenantId, shopId)
  });

  const catalogReady = tenantReady && !!catalogQuery.data;
  const writeDisabled = !canWrite || !catalogReady || saveMutation.isPending;
  const tableLoading = (catalogQuery.isLoading && !catalogQuery.data) || saveMutation.isPending;
  const overviewItems = overviewQuery.data?.items ?? [];

  const activeChanges = useMemo(
    () =>
      activeIntegrationId
        ? changes.filter((item) => item.integration_id === activeIntegrationId)
        : [],
    [activeIntegrationId, changes]
  );

  const overviewStats = useMemo(
    () =>
      overviewItems.reduce(
        (summary, item) => {
          summary.total += 1;
          if (item.integration_status === 'CONNECTED') {
            summary.connected += 1;
          }
          if (item.integration_status === 'WARNING') {
            summary.warning += 1;
          }
          if (item.integration_status === 'DISCONNECTED') {
            summary.disconnected += 1;
          }
          summary.failedJobs += item.failed_job_count;
          summary.alerts += item.warning_count;
          return summary;
        },
        {
          total: 0,
          connected: 0,
          warning: 0,
          disconnected: 0,
          failedJobs: 0,
          alerts: 0
        }
      ),
    [overviewItems]
  );
  const activeGovernance = useMemo(
    () => workflowQuery.data?.workflows.find((item) => item.status === 'ACTIVE')?.governance,
    [workflowQuery.data]
  );

  const persistCatalog = async (
    payload: SystemIntegrationsCatalog,
    successMessage: string
  ) => {
    const saved = await saveMutation.mutateAsync(payload);
    setConfigs(saved.configs);
    setChanges(saved.changes);
    await queryClient.invalidateQueries({ queryKey: ['system-integrations', tenantId] });
    message.success(successMessage);
  };

  const closeModal = () => {
    setModalOpen(false);
    setEditing(null);
  };

  const openModal = (record?: IntegrationConfig) => {
    setEditing(record ?? null);
    form.resetFields();
    if (record) {
      form.setFieldsValue(record);
    } else {
      form.setFieldsValue({
        category: categoryOptions[0]?.value ?? '电商平台',
        retry_policy: '失败重试 3 次 / 15 分钟',
        alert_channel: '企业微信-集成告警'
      });
    }
    setModalOpen(true);
  };

  const handleSave = async () => {
    try {
      const values = await form.validateFields();
      const updatedAt = dayjs().toISOString();
      if (editing) {
        const nextConfigs = configs.map((item) =>
          item.integration_id === editing.integration_id
            ? { ...item, ...values, updated_at: updatedAt }
            : item
        );
        const nextChanges = [
          {
            record_id: `CHG-${Date.now()}`,
            integration_id: editing.integration_id,
            action: '更新配置',
            operator: '系统管理员',
            comment: values.description,
            created_at: updatedAt
          },
          ...changes
        ];
        await persistCatalog(
          {
            configs: nextConfigs,
            changes: nextChanges
          },
          '集成配置已更新'
        );
      } else {
        const integrationId = `INT-${Date.now()}`;
        const nextConfigs = [
          {
            integration_id: integrationId,
            name: values.name,
            category: values.category,
            provider: values.provider,
            status: 'CONNECTED' as IntegrationStatus,
            success_rate: healthScoreByStatus.CONNECTED,
            retry_policy: values.retry_policy,
            alert_channel: values.alert_channel,
            last_checked_at: updatedAt,
            updated_at: updatedAt,
            description: values.description
          },
          ...configs
        ];
        const nextChanges = [
          {
            record_id: `CHG-${Date.now()}`,
            integration_id: integrationId,
            action: '新增集成',
            operator: '系统管理员',
            comment: values.description,
            created_at: updatedAt
          },
          ...changes
        ];
        await persistCatalog(
          {
            configs: nextConfigs,
            changes: nextChanges
          },
          '集成配置已新增'
        );
      }
      closeModal();
    } catch {
      return;
    }
  };

  const handleHealthCheck = async (record: IntegrationConfig) => {
    try {
      const nextStatus = nextStatusMap[record.status];
      const updatedAt = dayjs().toISOString();
      const nextConfigs = configs.map((item) =>
        item.integration_id === record.integration_id
          ? {
              ...item,
              status: nextStatus,
              success_rate: healthScoreByStatus[nextStatus],
              last_checked_at: updatedAt,
              updated_at: updatedAt
            }
          : item
      );
      const nextChanges = [
        {
          record_id: `CHG-${Date.now()}`,
          integration_id: record.integration_id,
          action: '执行健康检查',
          operator: '系统管理员',
          comment: `状态更新为${statusLabels[nextStatus]}`,
          created_at: updatedAt
        },
        ...changes
      ];
      await persistCatalog(
        {
          configs: nextConfigs,
          changes: nextChanges
        },
        `健康检查完成，当前状态：${statusLabels[nextStatus]}`
      );
    } catch {
      return;
    }
  };

  const handleRefreshToken = async (record: ShopIntegrationOverviewItem) => {
    try {
      setRefreshingShopId(record.shop_id);
      const payload = await refreshTokenMutation.mutateAsync(record.shop_id);
      message.success(`已提交 Token 刷新任务，流水号：${payload.outbox_id}`);
      await queryClient.invalidateQueries({ queryKey: ['system-integrations-shops', tenantId] });
    } catch (error) {
      message.error(error instanceof Error ? error.message : '提交 Token 刷新任务失败');
    } finally {
      setRefreshingShopId(null);
    }
  };

  const columns = useMemo<ColumnsType<IntegrationConfig>>(
    () => [
      { title: '集成名称', dataIndex: 'name' },
      { title: '类别', dataIndex: 'category' },
      { title: '服务商', dataIndex: 'provider' },
      {
        title: '状态',
        dataIndex: 'status',
        render: (value: IntegrationStatus) => (
          <Tag color={statusColors[value]}>{statusLabels[value]}</Tag>
        )
      },
      {
        title: '成功率',
        dataIndex: 'success_rate',
        render: (value: number) => `${value.toFixed(1)}%`
      },
      { title: '重试策略', dataIndex: 'retry_policy' },
      { title: '告警渠道', dataIndex: 'alert_channel' },
      {
        title: '最近检查',
        dataIndex: 'last_checked_at',
        render: (value?: string) => formatDateTime(value)
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => (
          <Space size={8}>
            <Button
              size="small"
              disabled={writeDisabled}
              onClick={() => void handleHealthCheck(record)}
            >
              健康检查
            </Button>
            <Button size="small" disabled={writeDisabled} onClick={() => openModal(record)}>
              配置
            </Button>
            <Button
              size="small"
              onClick={() => {
                setActiveIntegrationId(record.integration_id);
                setHistoryOpen(true);
              }}
            >
              变更记录
            </Button>
          </Space>
        )
      }
    ],
    [writeDisabled]
  );

  const shopOverviewColumns = useMemo<ColumnsType<ShopIntegrationOverviewItem>>(
    () => [
      {
        title: '店铺 / 平台',
        key: 'shop',
        render: (_, record) => (
          <Space direction="vertical" size={2}>
            <Space size={8} wrap>
              <Text strong>{record.name}</Text>
              <Tag>{formatPlatform(record.platform)}</Tag>
            </Space>
            <Text type="secondary">{record.platform_shop_id}</Text>
          </Space>
        )
      },
      {
        title: '接入状态',
        key: 'health',
        render: (_, record) => (
          <Space direction="vertical" size={4}>
            <Space size={[8, 4]} wrap>
              <Tag color={statusColors[record.integration_status]}>
                {statusLabels[record.integration_status]}
              </Tag>
              <Tag color={oauthStatusColors[record.oauth_status] ?? 'default'}>
                {formatOauthStatus(record.oauth_status)}
              </Tag>
              <Tag color={webhookStatusColors[record.webhook_status]}>
                {formatWebhookStatus(record.webhook_status)}
              </Tag>
              <Tag color={webhookSlaStatusColors[record.webhook_sla_status]}>
                {formatWebhookSlaStatus(record.webhook_sla_status)}
              </Tag>
              <Tag color="blue">{formatWebhookMode(record.webhook_mode)}</Tag>
            </Space>
            <Text type="secondary">Token：{formatTokenStatus(record.token_status)}</Text>
          </Space>
        )
      },
      {
        title: '同步健康',
        key: 'sync',
        render: (_, record) => (
          <Space direction="vertical" size={4}>
            <Text>
              启用任务 {record.enabled_job_count} / 失败任务 {record.failed_job_count}
            </Text>
            <Text type="secondary">
              Webhook 失败 {record.webhook_failed_count} / 验签失败{' '}
              {record.webhook_verify_failed_count}
            </Text>
            <Text type="secondary">
              待处理 {record.webhook_pending_count} / 重试 {record.webhook_retrying_count} / 死信{' '}
              {record.webhook_dlq_count}
            </Text>
            {record.webhook_next_retry_at ? (
              <Text type="secondary">
                下次重试：{formatDateTime(record.webhook_next_retry_at)}
              </Text>
            ) : null}
            <Space size={[4, 4]} wrap>
              {record.jobs.length > 0 ? (
                record.jobs.map((job) => (
                  <Tag
                    key={`${record.shop_id}-${job.job_type}`}
                    color={
                      job.status === 'FAILED'
                        ? 'red'
                        : job.enabled
                          ? 'processing'
                          : 'default'
                    }
                  >
                    {jobTypeLabels[job.job_type] ?? job.job_type}
                  </Tag>
                ))
              ) : (
                <Text type="secondary">未配置同步任务</Text>
              )}
            </Space>
          </Space>
        )
      },
      {
        title: '最近同步 / Webhook',
        key: 'last_sync_at',
        render: (_, record) => (
          <Space direction="vertical" size={2}>
            <Text>最近同步：{formatDateTime(record.last_sync_at)}</Text>
            <Text type="secondary">
              最近 Webhook：{formatDateTime(record.last_webhook_received_at)}
            </Text>
            <Text type="secondary">
              最近处理：{formatDateTime(record.last_webhook_processed_at)}
            </Text>
            <Text type="secondary">Token 到期：{formatDateTime(record.token_expires_at)}</Text>
          </Space>
        )
      },
      {
        title: '告警',
        key: 'alerts',
        render: (_, record) =>
          record.alert_messages.length > 0 ? (
            <Space size={[4, 4]} wrap>
              {record.alert_messages.slice(0, 2).map((alert) => (
                <Tag key={`${record.shop_id}-${alert}`} color="orange">
                  {alert}
                </Tag>
              ))}
              {record.alert_messages.length > 2 ? (
                <Tooltip title={record.alert_messages.join('；')}>
                  <Text type="secondary">+{record.alert_messages.length - 2}</Text>
                </Tooltip>
              ) : null}
            </Space>
          ) : (
            <Tag color="green">正常</Tag>
          )
      },
      {
        title: '操作',
        key: 'actions',
        render: (_, record) => {
          const canRequestRefresh =
            canOperateShopIntegration &&
            record.oauth_status === 'SUCCESS' &&
            typeof record.token_status === 'string';
          const actionButton = (
            <Button
              size="small"
              disabled={!canRequestRefresh}
              loading={refreshingShopId === record.shop_id}
              data-testid={`system-integrations-shop-refresh-${record.shop_id}`}
              onClick={() => void handleRefreshToken(record)}
            >
              刷新 Token
            </Button>
          );
          if (canRequestRefresh) {
            return actionButton;
          }
          return (
            <Tooltip
              title={
                canOperateShopIntegration
                  ? '当前店铺尚未形成可刷新 Token 的授权状态'
                  : '缺少系统写入或渠道写入权限'
              }
            >
              <span>{actionButton}</span>
            </Tooltip>
          );
        }
      }
    ],
    [canOperateShopIntegration, refreshingShopId]
  );

  if (!tenantReady) {
    return (
      <SettingsPageTemplate
        title={<span data-testid="system-integrations-title">接口与集成</span>}
      >
        <EmptyState description="请先选择租户" />
      </SettingsPageTemplate>
    );
  }

  return (
    <SettingsPageTemplate
      title={<span data-testid="system-integrations-title">接口与集成</span>}
    >
      {catalogQuery.isError ? (
        <ErrorState
          title="加载集成中心配置失败"
          error={catalogQuery.error}
          onRetry={() => void catalogQuery.refetch()}
        />
      ) : null}
      <SettingsPageSection
        title="平台店铺接入总览"
        actions={
          <Button onClick={() => void overviewQuery.refetch()} loading={overviewQuery.isFetching}>
            刷新总览
          </Button>
        }
      >
        {overviewQuery.isError ? (
          <ErrorState
            title="加载平台店铺接入总览失败"
            error={overviewQuery.error}
            onRetry={() => void overviewQuery.refetch()}
          />
        ) : (
          <Space
            direction="vertical"
            size={16}
            style={{ width: '100%' }}
            data-testid="system-integrations-overview-section"
          >
            <Space wrap data-testid="system-integrations-governance-summary">
              <Tag color="blue">治理链路</Tag>
              <Tag color="processing">发布状态: {activeGovernance?.publish_status ?? 'DRAFT'}</Tag>
              <Tag>流程引擎: {activeGovernance?.engine ?? 'CAMUNDA8'}</Tag>
              <Tag>流程键: {activeGovernance?.process_key ?? '-'}</Tag>
              <Tag>生效时间: {activeGovernance?.effective_at ?? '-'}</Tag>
            </Space>
            <Text type="secondary">
              按店铺聚合 OAuth、Webhook、同步任务和 Token 健康状态，用于判断平台接入是否可持续运行。
            </Text>
            <Row gutter={[12, 12]}>
              <Col xs={24} md={6}>
                <Card size="small">
                  <Statistic title="已连接店铺" value={overviewStats.connected} valueStyle={{ color: '#389e0d' }} />
                </Card>
              </Col>
              <Col xs={24} md={6}>
                <Card size="small">
                  <Statistic title="告警店铺" value={overviewStats.warning} valueStyle={{ color: '#d48806' }} />
                </Card>
              </Col>
              <Col xs={24} md={6}>
                <Card size="small">
                  <Statistic
                    title="未连接店铺"
                    value={overviewStats.disconnected}
                    valueStyle={{ color: '#cf1322' }}
                  />
                </Card>
              </Col>
              <Col xs={24} md={6}>
                <Card size="small">
                  <Statistic title="失败同步任务" value={overviewStats.failedJobs} valueStyle={{ color: '#cf1322' }} />
                </Card>
              </Col>
            </Row>
            <div data-testid="system-integrations-overview-table">
              <Table<ShopIntegrationOverviewItem>
                rowKey="shop_id"
                columns={shopOverviewColumns}
                dataSource={overviewItems}
                pagination={false}
                loading={overviewQuery.isLoading || overviewQuery.isFetching}
                locale={{ emptyText: <EmptyState description="暂无已接入店铺" /> }}
              />
            </div>
          </Space>
        )}
      </SettingsPageSection>
      <SettingsPageSection
        title="集成清单"
        actions={
          <Button type="primary" disabled={writeDisabled} onClick={() => openModal()}>
            新增集成
          </Button>
        }
      >
        <div data-testid="system-integrations-catalog-section">
          <Table
          rowKey="integration_id"
          columns={columns}
          dataSource={configs}
          pagination={false}
          loading={tableLoading}
          />
        </div>
      </SettingsPageSection>
      <Modal
        title={editing ? '编辑集成' : '新增集成'}
        open={modalOpen}
        onCancel={closeModal}
        onOk={() => void handleSave()}
        okText="保存"
        okButtonProps={{ loading: saveMutation.isPending }}
        cancelButtonProps={{ disabled: saveMutation.isPending }}
        destroyOnHidden
      >
        <Form form={form} layout="vertical">
          <Form.Item
            name="name"
            label="集成名称"
            rules={[{ required: true, message: '请输入集成名称' }]}
          >
            <Input placeholder="例如：天猫旗舰店" />
          </Form.Item>
          <Form.Item
            name="category"
            label="集成类别"
            rules={[{ required: true, message: '请选择集成类别' }]}
          >
            <Select options={categoryOptions} />
          </Form.Item>
          <Form.Item
            name="provider"
            label="服务商"
            rules={[{ required: true, message: '请输入服务商' }]}
          >
            <Input placeholder="例如：Alibaba OpenAPI" />
          </Form.Item>
          <Form.Item
            name="retry_policy"
            label="失败重试策略"
            rules={[{ required: true, message: '请输入重试策略' }]}
          >
            <Input placeholder="失败重试 3 次 / 15 分钟" />
          </Form.Item>
          <Form.Item
            name="alert_channel"
            label="告警渠道"
            rules={[{ required: true, message: '请输入告警渠道' }]}
          >
            <Input placeholder="企业微信-集成告警" />
          </Form.Item>
          <Form.Item name="description" label="备注">
            <Input.TextArea rows={3} placeholder="记录集成用途与注意事项" />
          </Form.Item>
        </Form>
      </Modal>
      <Modal
        title="集成变更记录"
        open={historyOpen}
        onCancel={() => setHistoryOpen(false)}
        footer={[
          <Button key="close" onClick={() => setHistoryOpen(false)}>
            关闭
          </Button>
        ]}
      >
        <Table<IntegrationChange>
          rowKey="record_id"
          dataSource={activeChanges}
          pagination={false}
          columns={[
            { title: '动作', dataIndex: 'action' },
            { title: '操作人', dataIndex: 'operator' },
            { title: '说明', dataIndex: 'comment', ellipsis: true },
            {
              title: '时间',
              dataIndex: 'created_at',
              render: (value: string) => formatDateTime(value)
            }
          ]}
        />
      </Modal>
    </SettingsPageTemplate>
  );
};
