import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type ReportStatus = 'READY' | 'GENERATING' | 'FAILED';

export type AuditReport = {
  report_id: string;
  module: string;
  period: string;
  owner: string;
  anomalies: number;
  status: ReportStatus;
  generated_at: string;
};

export type AlertRule = {
  rule_id: string;
  module: string;
  name: string;
  condition: string;
  enabled: boolean;
  updated_at: string;
};

export type SystemAuditReportsCatalog = {
  reports: AuditReport[];
  rules: AlertRule[];
};

const cloneCatalog = (catalog: SystemAuditReportsCatalog): SystemAuditReportsCatalog =>
  JSON.parse(JSON.stringify(catalog)) as SystemAuditReportsCatalog;

const buildMockCatalog = (): SystemAuditReportsCatalog => ({
  reports: [
    {
      report_id: 'REP-ORDER-01',
      module: '订单',
      period: '2025-12',
      owner: '审计专员',
      anomalies: 3,
      status: 'READY',
      generated_at: '2026-04-06T00:00:00.000Z'
    },
    {
      report_id: 'REP-INV-02',
      module: '库存',
      period: '2025-12',
      owner: '系统管理员',
      anomalies: 0,
      status: 'READY',
      generated_at: '2026-04-07T00:00:00.000Z'
    },
    {
      report_id: 'REP-RET-03',
      module: '售后',
      period: '2025-12',
      owner: '运营主管',
      anomalies: 5,
      status: 'GENERATING',
      generated_at: '2026-04-07T18:00:00.000Z'
    }
  ],
  rules: [
    {
      rule_id: 'RULE-001',
      module: '订单',
      name: '异常订单金额',
      condition: '订单金额 > 10 万元',
      enabled: true,
      updated_at: '2026-04-06T00:00:00.000Z'
    },
    {
      rule_id: 'RULE-002',
      module: '库存',
      name: '负库存告警',
      condition: '库存 < 0',
      enabled: true,
      updated_at: '2026-04-07T00:00:00.000Z'
    }
  ]
});

let mockCatalog: SystemAuditReportsCatalog | null = null;

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

export const getSystemAuditReportsCatalog = async (
  tenantId: string
): Promise<SystemAuditReportsCatalog> => {
  if (useMocks) {
    if (!mockCatalog) {
      mockCatalog = buildMockCatalog();
    }
    return cloneCatalog(mockCatalog);
  }
  return requestJson<SystemAuditReportsCatalog>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/audit-reports`,
    { headers: buildAuthHeaders() },
    '获取审计报告失败'
  );
};

export const updateSystemAuditReportsCatalog = async (
  tenantId: string,
  payload: SystemAuditReportsCatalog
): Promise<SystemAuditReportsCatalog> => {
  if (useMocks) {
    mockCatalog = cloneCatalog(payload);
    return cloneCatalog(mockCatalog);
  }
  return requestJson<SystemAuditReportsCatalog>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/audit-reports`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '保存审计报告配置失败'
  );
};
