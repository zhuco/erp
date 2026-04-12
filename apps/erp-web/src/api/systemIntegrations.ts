import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type IntegrationStatus = 'CONNECTED' | 'WARNING' | 'DISCONNECTED';
export type WebhookStatus = 'HEALTHY' | 'WARNING' | 'IDLE';
export type WebhookSlaStatus = 'HEALTHY' | 'WARNING' | 'DEAD' | 'IDLE';

export type IntegrationConfig = {
  integration_id: string;
  name: string;
  category: string;
  provider: string;
  status: IntegrationStatus;
  success_rate: number;
  retry_policy: string;
  alert_channel: string;
  last_checked_at?: string;
  updated_at: string;
  description?: string;
};

export type IntegrationChange = {
  record_id: string;
  integration_id: string;
  action: string;
  operator: string;
  comment?: string;
  created_at: string;
};

export type SystemIntegrationsCatalog = {
  configs: IntegrationConfig[];
  changes: IntegrationChange[];
};

export type ShopIntegrationOverviewJob = {
  job_type: string;
  status: string;
  enabled: boolean;
  last_error?: string;
  last_success_at?: string;
};

export type ShopIntegrationOverviewItem = {
  shop_id: string;
  platform: string;
  platform_shop_id: string;
  name: string;
  shop_status: string;
  oauth_status: string;
  integration_status: IntegrationStatus;
  webhook_mode?: string;
  webhook_status: WebhookStatus;
  webhook_sla_status: WebhookSlaStatus;
  last_webhook_received_at?: string;
  last_webhook_processed_at?: string;
  webhook_failed_count: number;
  webhook_verify_failed_count: number;
  webhook_pending_count: number;
  webhook_retrying_count: number;
  webhook_dlq_count: number;
  webhook_next_retry_at?: string;
  token_status?: string;
  token_expires_at?: string;
  last_sync_at?: string;
  enabled_job_count: number;
  failed_job_count: number;
  warning_count: number;
  alert_messages: string[];
  jobs: ShopIntegrationOverviewJob[];
};

export type ShopIntegrationOverviewResponse = {
  items: ShopIntegrationOverviewItem[];
  total: number;
};

const cloneCatalog = (catalog: SystemIntegrationsCatalog): SystemIntegrationsCatalog =>
  JSON.parse(JSON.stringify(catalog)) as SystemIntegrationsCatalog;

const cloneShopOverview = (
  overview: ShopIntegrationOverviewResponse
): ShopIntegrationOverviewResponse =>
  JSON.parse(JSON.stringify(overview)) as ShopIntegrationOverviewResponse;

const buildMockCatalog = (): SystemIntegrationsCatalog => ({
  configs: [
    {
      integration_id: 'INT-EC-01',
      name: '天猫旗舰店',
      category: '电商平台',
      provider: 'Alibaba OpenAPI',
      status: 'CONNECTED',
      success_rate: 98.7,
      retry_policy: '失败重试 3 次 / 15 分钟',
      alert_channel: '企业微信-集成告警',
      last_checked_at: '2026-04-08T08:00:00.000Z',
      updated_at: '2026-04-07T00:00:00.000Z',
      description: '用于订单与库存同步'
    },
    {
      integration_id: 'INT-PAY-02',
      name: '微信支付',
      category: '支付渠道',
      provider: 'WeChat Pay',
      status: 'WARNING',
      success_rate: 92.4,
      retry_policy: '失败重试 5 次 / 10 分钟',
      alert_channel: '钉钉-财务告警',
      last_checked_at: '2026-04-08T04:00:00.000Z',
      updated_at: '2026-04-06T00:00:00.000Z',
      description: '支付对账接口'
    }
  ],
  changes: [
    {
      record_id: 'CHG-1001',
      integration_id: 'INT-EC-01',
      action: '更新重试策略',
      operator: '系统管理员',
      comment: '高峰期调整重试次数',
      created_at: '2026-04-05T00:00:00.000Z'
    },
    {
      record_id: 'CHG-1002',
      integration_id: 'INT-PAY-02',
      action: '告警渠道调整',
      operator: '财务主管',
      comment: '新增钉钉告警群',
      created_at: '2026-04-07T00:00:00.000Z'
    }
  ]
});

const buildMockShopOverview = (): ShopIntegrationOverviewResponse => ({
  items: [
    {
      shop_id: 'SHOP-TB-001',
      platform: 'taobao',
      platform_shop_id: 'TB-HEALTHY',
      name: '天猫旗舰店',
      shop_status: 'AUTHORIZED',
      oauth_status: 'SUCCESS',
      integration_status: 'CONNECTED',
      webhook_mode: 'STRICT',
      webhook_status: 'HEALTHY',
      webhook_sla_status: 'HEALTHY',
      last_webhook_received_at: '2026-04-08T08:12:00.000Z',
      last_webhook_processed_at: '2026-04-08T08:12:20.000Z',
      webhook_failed_count: 0,
      webhook_verify_failed_count: 0,
      webhook_pending_count: 0,
      webhook_retrying_count: 0,
      webhook_dlq_count: 0,
      token_status: 'ACTIVE',
      token_expires_at: '2026-04-08T18:00:00.000Z',
      last_sync_at: '2026-04-08T08:10:00.000Z',
      enabled_job_count: 3,
      failed_job_count: 0,
      warning_count: 0,
      alert_messages: [],
      jobs: [
        {
          job_type: 'order_sync',
          status: 'PENDING',
          enabled: true,
          last_success_at: '2026-04-08T08:10:00.000Z'
        },
        {
          job_type: 'refund_sync',
          status: 'PENDING',
          enabled: true,
          last_success_at: '2026-04-08T08:05:00.000Z'
        },
        {
          job_type: 'shipping_sync',
          status: 'PENDING',
          enabled: true,
          last_success_at: '2026-04-08T07:58:00.000Z'
        }
      ]
    },
    {
      shop_id: 'SHOP-JD-002',
      platform: 'jd',
      platform_shop_id: 'JD-WARNING',
      name: '京东自营店',
      shop_status: 'EXPIRED',
      oauth_status: 'FAILED',
      integration_status: 'DISCONNECTED',
      webhook_mode: 'STRICT',
      webhook_status: 'IDLE',
      webhook_sla_status: 'IDLE',
      webhook_failed_count: 0,
      webhook_verify_failed_count: 0,
      webhook_pending_count: 0,
      webhook_retrying_count: 0,
      webhook_dlq_count: 0,
      token_status: 'EXPIRED',
      token_expires_at: '2026-04-08T03:00:00.000Z',
      last_sync_at: '2026-04-08T02:30:00.000Z',
      enabled_job_count: 2,
      failed_job_count: 1,
      warning_count: 3,
      alert_messages: ['授权状态异常', 'Token 已过期', 'order_sync 同步失败'],
      jobs: [
        {
          job_type: 'order_sync',
          status: 'FAILED',
          enabled: true,
          last_error: 'platform timeout',
          last_success_at: '2026-04-08T02:30:00.000Z'
        },
        {
          job_type: 'shipping_sync',
          status: 'PENDING',
          enabled: true,
          last_success_at: '2026-04-08T02:10:00.000Z'
        }
      ]
    },
    {
      shop_id: 'SHOP-DY-003',
      platform: 'douyin',
      platform_shop_id: 'DY-WEBHOOK-WARN',
      name: '抖店 webhook 预警店',
      shop_status: 'AUTHORIZED',
      oauth_status: 'SUCCESS',
      integration_status: 'WARNING',
      webhook_mode: 'STRICT',
      webhook_status: 'WARNING',
      webhook_sla_status: 'WARNING',
      last_webhook_received_at: '2026-04-08T07:52:00.000Z',
      webhook_failed_count: 1,
      webhook_verify_failed_count: 1,
      webhook_pending_count: 1,
      webhook_retrying_count: 1,
      webhook_dlq_count: 0,
      webhook_next_retry_at: '2026-04-08T08:04:00.000Z',
      token_status: 'ACTIVE',
      token_expires_at: '2026-04-08T17:00:00.000Z',
      last_sync_at: '2026-04-08T07:54:00.000Z',
      enabled_job_count: 1,
      failed_job_count: 0,
      warning_count: 2,
      alert_messages: ['Webhook 验签失败', 'Webhook 自动重试中'],
      jobs: [
        {
          job_type: 'order_sync',
          status: 'PENDING',
          enabled: true,
          last_success_at: '2026-04-08T07:54:00.000Z'
        }
      ]
    },
    {
      shop_id: 'SHOP-PDD-004',
      platform: 'pinduoduo',
      platform_shop_id: 'PDD-WEBHOOK-DEAD',
      name: '拼多多死信店铺',
      shop_status: 'AUTHORIZED',
      oauth_status: 'SUCCESS',
      integration_status: 'WARNING',
      webhook_mode: 'STRICT',
      webhook_status: 'WARNING',
      webhook_sla_status: 'DEAD',
      last_webhook_received_at: '2026-04-08T07:45:00.000Z',
      webhook_failed_count: 1,
      webhook_verify_failed_count: 0,
      webhook_pending_count: 0,
      webhook_retrying_count: 0,
      webhook_dlq_count: 1,
      token_status: 'ACTIVE',
      token_expires_at: '2026-04-08T18:30:00.000Z',
      last_sync_at: '2026-04-08T07:40:00.000Z',
      enabled_job_count: 1,
      failed_job_count: 0,
      warning_count: 2,
      alert_messages: ['Webhook 处理失败', 'Webhook 死信待处理'],
      jobs: [
        {
          job_type: 'order_sync',
          status: 'PENDING',
          enabled: true,
          last_success_at: '2026-04-08T07:40:00.000Z'
        }
      ]
    }
  ],
  total: 4
});

let mockCatalog: SystemIntegrationsCatalog | null = null;
let mockShopOverview: ShopIntegrationOverviewResponse | null = null;

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const response = await fetchWithAuth(input, init);
  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError(message, response.status, payload);
  }
  return payload as T;
};

export const getSystemIntegrationsCatalog = async (
  tenantId: string
): Promise<SystemIntegrationsCatalog> => {
  if (useMocks) {
    if (!mockCatalog) {
      mockCatalog = buildMockCatalog();
    }
    return cloneCatalog(mockCatalog);
  }
  return requestJson<SystemIntegrationsCatalog>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/integrations`,
    { headers: buildAuthHeaders() },
    '获取集成中心配置失败'
  );
};

export const updateSystemIntegrationsCatalog = async (
  tenantId: string,
  payload: SystemIntegrationsCatalog
): Promise<SystemIntegrationsCatalog> => {
  if (useMocks) {
    mockCatalog = cloneCatalog(payload);
    return cloneCatalog(mockCatalog);
  }
  return requestJson<SystemIntegrationsCatalog>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/integrations`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '保存集成中心配置失败'
  );
};

export const getShopIntegrationOverview = async (
  tenantId: string
): Promise<ShopIntegrationOverviewResponse> => {
  if (useMocks) {
    if (!mockShopOverview) {
      mockShopOverview = buildMockShopOverview();
    }
    return cloneShopOverview(mockShopOverview);
  }
  return requestJson<ShopIntegrationOverviewResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/integrations/shops`,
    { headers: buildAuthHeaders() },
    '获取平台店铺接入总览失败'
  );
};
