import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  getMockWarehouseWorkflowConfig,
  updateMockWarehouseWorkflowConfig,
} from '@/api/mocks';

export type WarehouseWorkflowConfig = {
  warehouse_id: string;
  enable_wave: boolean;
  require_inspection: boolean;
  require_weighing: boolean;
  require_pack: boolean;
};

export type WarehouseWorkflowConfigPayload = {
  enable_wave: boolean;
  require_inspection: boolean;
  require_weighing: boolean;
  require_pack: boolean;
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

export const getWarehouseWorkflowConfig = async (
  tenantId: string,
  warehouseId: string
): Promise<WarehouseWorkflowConfig> => {
  if (useMocks) {
    void tenantId;
    return getMockWarehouseWorkflowConfig(warehouseId);
  }
  return requestJson<WarehouseWorkflowConfig>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/workflow-configs/${warehouseId}`,
    { headers: buildAuthHeaders() },
    '获取流程配置失败'
  );
};

export const updateWarehouseWorkflowConfig = async (
  tenantId: string,
  warehouseId: string,
  payload: WarehouseWorkflowConfigPayload
): Promise<WarehouseWorkflowConfig> => {
  if (useMocks) {
    void tenantId;
    return updateMockWarehouseWorkflowConfig(warehouseId, payload);
  }
  return requestJson<WarehouseWorkflowConfig>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/wms/workflow-configs/${warehouseId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新流程配置失败'
  );
};
