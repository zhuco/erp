import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockPrintTemplate,
  deleteMockPrintTemplate,
  getMockPrintTemplateList,
  updateMockPrintTemplate
} from '@/api/mocks';
import type { PlatformCode } from '@/features/platform';

export type PrintTemplateType = 'SHIPPING_LABEL' | 'PICK_LIST' | 'PACK_LIST' | 'INVOICE';
export type PrintTemplateFormat = 'HTML' | 'ZPL' | 'TEXT';

export type PrintTemplate = {
  template_id: string;
  platform: PlatformCode | string;
  shop_id: string;
  template_type: PrintTemplateType | string;
  version: number;
  content_format: PrintTemplateFormat | string;
  content?: unknown;
  enabled: boolean;
  created_at?: string | null;
  updated_at?: string | null;
};

export type PrintTemplateListQuery = {
  platform?: PlatformCode;
  shop_id?: string;
  template_type?: PrintTemplateType;
  enabled?: boolean;
  offset?: number;
  limit?: number;
};

export type PrintTemplateListResponse = {
  items: PrintTemplate[];
  total: number;
};

export type PrintTemplateCreatePayload = {
  shop_id: string;
  platform: PlatformCode;
  template_type: PrintTemplateType;
  version?: number;
  content_format?: PrintTemplateFormat;
  content?: unknown;
  enabled?: boolean;
};

export type PrintTemplateUpdatePayload = {
  platform_template_id?: string | null;
  content_format?: PrintTemplateFormat;
  content?: unknown;
  enabled?: boolean;
};

type PrintTemplateListApiResponse = {
  data?: PrintTemplate[];
  items?: PrintTemplate[];
  total?: number;
};

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

const buildQueryString = (query: PrintTemplateListQuery) => {
  const params = new URLSearchParams();
  if (query.platform) {
    params.set('platform', query.platform);
  }
  if (query.shop_id) {
    params.set('shop_id', query.shop_id);
  }
  if (query.template_type) {
    params.set('template_type', query.template_type);
  }
  if (query.enabled != null) {
    params.set('enabled', String(query.enabled));
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const normalizeListResponse = (
  payload: PrintTemplateListApiResponse | PrintTemplateListResponse
): PrintTemplateListResponse => {
  if ('items' in payload) {
    return {
      items: payload.items ?? [],
      total: payload.total ?? payload.items?.length ?? 0
    };
  }
  return {
    items: payload.data ?? [],
    total: payload.total ?? payload.data?.length ?? 0
  };
};

export const listPrintTemplates = async (
  tenantId: string,
  query: PrintTemplateListQuery
): Promise<PrintTemplateListResponse> => {
  if (useMocks) {
    return getMockPrintTemplateList(query);
  }
  const response = await requestJson<PrintTemplateListApiResponse | PrintTemplateListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-templates${buildQueryString(query)}`,
    {
      headers: buildAuthHeaders()
    },
    '获取打印模板失败'
  );
  return normalizeListResponse(response);
};

export const createPrintTemplate = async (
  tenantId: string,
  payload: PrintTemplateCreatePayload
): Promise<PrintTemplate> => {
  if (useMocks) {
    return createMockPrintTemplate(payload);
  }
  return requestJson<PrintTemplate>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-templates`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建打印模板失败'
  );
};

export const updatePrintTemplate = async (
  tenantId: string,
  templateId: string,
  payload: PrintTemplateUpdatePayload
): Promise<PrintTemplate> => {
  if (useMocks) {
    return updateMockPrintTemplate(templateId, payload);
  }
  return requestJson<PrintTemplate>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-templates/${templateId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新打印模板失败'
  );
};

export const deletePrintTemplate = async (
  tenantId: string,
  templateId: string
): Promise<PrintTemplate> => {
  if (useMocks) {
    return deleteMockPrintTemplate(templateId);
  }
  return requestJson<PrintTemplate>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-templates/${templateId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除打印模板失败'
  );
};
