import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type PrintMapping = {
  mapping_id: string;
  tenant_id: string;
  doc_type: string;
  template_id: string;
  device_fingerprint?: string | null;
  printer_name?: string | null;
  created_at: string;
  updated_at: string;
};

export type PrintMappingUpsertPayload = {
  doc_type: string;
  template_id: string;
  device_fingerprint?: string | null;
  printer_name?: string | null;
};

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const response = await fetch(input, init);
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

export const listPrintMappings = async (tenantId: string): Promise<PrintMapping[]> => {
  if (useMocks) {
    return [];
  }
  return requestJson<PrintMapping[]>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-mappings`,
    { headers: buildAuthHeaders() },
    '获取打印映射失败'
  );
};

export const upsertPrintMapping = async (
  tenantId: string,
  payload: PrintMappingUpsertPayload
): Promise<PrintMapping> => {
  if (useMocks) {
    return {
      mapping_id: `${Date.now()}`,
      tenant_id: tenantId,
      doc_type: payload.doc_type,
      template_id: payload.template_id,
      device_fingerprint: payload.device_fingerprint ?? null,
      printer_name: payload.printer_name ?? null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<PrintMapping>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-mappings`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新打印映射失败'
  );
};

export const deletePrintMapping = async (
  tenantId: string,
  docType: string
): Promise<PrintMapping> => {
  if (useMocks) {
    return {
      mapping_id: `${Date.now()}`,
      tenant_id: tenantId,
      doc_type: docType,
      template_id: '',
      device_fingerprint: null,
      printer_name: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<PrintMapping>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/print-mappings/${docType}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除打印映射失败'
  );
};
