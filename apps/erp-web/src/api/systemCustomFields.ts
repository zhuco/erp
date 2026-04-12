import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type FieldStatus = 'ACTIVE' | 'DISABLED';
export type FieldType = 'TEXT' | 'NUMBER' | 'DATE' | 'SELECT';

export type CustomField = {
  field_id: string;
  entity: string;
  label: string;
  field_key: string;
  field_type: FieldType;
  required: boolean;
  compatible: boolean;
  status: FieldStatus;
  description?: string;
  updated_at: string;
};

export type FieldAudit = {
  record_id: string;
  field_id: string;
  action: string;
  operator: string;
  comment?: string;
  created_at: string;
};

export type SystemCustomFieldsCatalog = {
  fields: CustomField[];
  audits: FieldAudit[];
};

const cloneCatalog = (catalog: SystemCustomFieldsCatalog): SystemCustomFieldsCatalog =>
  JSON.parse(JSON.stringify(catalog)) as SystemCustomFieldsCatalog;

const buildMockCatalog = (): SystemCustomFieldsCatalog => ({
  fields: [
    {
      field_id: 'CF-ORDER-01',
      entity: '订单',
      label: '渠道备注',
      field_key: 'channel_note',
      field_type: 'TEXT',
      required: false,
      compatible: true,
      status: 'ACTIVE',
      description: '同步渠道订单备注',
      updated_at: '2026-04-06T00:00:00.000Z'
    },
    {
      field_id: 'CF-SKU-02',
      entity: '商品',
      label: '温控等级',
      field_key: 'temp_level',
      field_type: 'SELECT',
      required: true,
      compatible: true,
      status: 'ACTIVE',
      description: '生鲜商品温控要求',
      updated_at: '2026-04-02T00:00:00.000Z'
    },
    {
      field_id: 'CF-PUR-03',
      entity: '采购',
      label: '预计到货周期',
      field_key: 'lead_time',
      field_type: 'NUMBER',
      required: false,
      compatible: false,
      status: 'DISABLED',
      description: '用于供应商响应分析',
      updated_at: '2026-03-29T00:00:00.000Z'
    }
  ],
  audits: [
    {
      record_id: 'AUD-1001',
      field_id: 'CF-ORDER-01',
      action: '创建字段',
      operator: '系统管理员',
      comment: '首次启用渠道备注',
      created_at: '2026-04-03T00:00:00.000Z'
    },
    {
      record_id: 'AUD-1002',
      field_id: 'CF-SKU-02',
      action: '更新字段',
      operator: '商品主管',
      comment: '新增温控等级选项',
      created_at: '2026-04-06T00:00:00.000Z'
    }
  ]
});

let mockCatalog: SystemCustomFieldsCatalog | null = null;

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

export const getSystemCustomFieldsCatalog = async (
  tenantId: string
): Promise<SystemCustomFieldsCatalog> => {
  if (useMocks) {
    if (!mockCatalog) {
      mockCatalog = buildMockCatalog();
    }
    return cloneCatalog(mockCatalog);
  }
  return requestJson<SystemCustomFieldsCatalog>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/custom-fields`,
    { headers: buildAuthHeaders() },
    '获取自定义字段失败'
  );
};

export const updateSystemCustomFieldsCatalog = async (
  tenantId: string,
  payload: SystemCustomFieldsCatalog
): Promise<SystemCustomFieldsCatalog> => {
  if (useMocks) {
    mockCatalog = cloneCatalog(payload);
    return cloneCatalog(mockCatalog);
  }
  return requestJson<SystemCustomFieldsCatalog>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/custom-fields`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '保存自定义字段失败'
  );
};
