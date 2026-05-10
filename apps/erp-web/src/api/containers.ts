import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type Container = {
  container_id: string;
  code: string;
  container_type: string;
  capacity?: number | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type ContainerListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type ContainerListResponse = {
  items: Container[];
  total: number;
};

export type ContainerCreatePayload = {
  code: string;
  container_type: string;
  capacity?: number | null;
};

export type ContainerUpdatePayload = {
  container_type?: string;
  capacity?: number | null;
};

const mockContainers: Container[] = [
  {
    container_id: 'CT-001',
    code: 'BOX-01',
    container_type: '周转箱',
    capacity: 50,
    created_at: new Date(Date.now() - 86400000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString()
  },
  {
    container_id: 'CT-002',
    code: 'PALLET-01',
    container_type: '托盘',
    capacity: 120,
    created_at: new Date(Date.now() - 172800000).toISOString(),
    updated_at: new Date(Date.now() - 7200000).toISOString()
  },
  {
    container_id: 'CT-003',
    code: 'TOTE-01',
    container_type: '拣货筐',
    capacity: 30,
    created_at: new Date(Date.now() - 259200000).toISOString(),
    updated_at: new Date(Date.now() - 10800000).toISOString()
  },
  {
    container_id: 'CT-004',
    code: 'ROLL-01',
    container_type: '笼车',
    capacity: 200,
    created_at: new Date(Date.now() - 345600000).toISOString(),
    updated_at: new Date(Date.now() - 14400000).toISOString()
  },
  {
    container_id: 'CT-005',
    code: 'COLD-01',
    container_type: '冷链箱',
    capacity: 18,
    created_at: new Date(Date.now() - 432000000).toISOString(),
    updated_at: new Date(Date.now() - 18000000).toISOString()
  },
  {
    container_id: 'CT-006',
    code: 'BIN-01',
    container_type: '料箱',
    capacity: 24,
    created_at: new Date(Date.now() - 518400000).toISOString(),
    updated_at: new Date(Date.now() - 21600000).toISOString()
  },
  {
    container_id: 'CT-007',
    code: 'PALLET-02',
    container_type: '托盘',
    capacity: 150,
    created_at: new Date(Date.now() - 604800000).toISOString(),
    updated_at: new Date(Date.now() - 25200000).toISOString()
  },
  {
    container_id: 'CT-008',
    code: 'BOX-02',
    container_type: '周转箱',
    capacity: 65,
    created_at: new Date(Date.now() - 691200000).toISOString(),
    updated_at: new Date(Date.now() - 28800000).toISOString()
  }
];

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

const buildQueryString = (query: ContainerListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
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

export const listContainers = async (
  tenantId: string,
  query: ContainerListQuery
): Promise<ContainerListResponse> => {
  if (useMocks) {
    const keyword = query.keyword?.trim().toLowerCase();
    const filtered = mockContainers.filter((item) => {
      if (!keyword) {
        return true;
      }
      return (
        item.code.toLowerCase().includes(keyword) ||
        item.container_type.toLowerCase().includes(keyword)
      );
    });
    const offset = query.offset ?? 0;
    const limit = query.limit ?? 20;
    return {
      items: filtered.slice(offset, offset + limit),
      total: filtered.length
    };
  }
  return requestJson<ContainerListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/containers${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取容器列表失败'
  );
};

export const createContainer = async (
  tenantId: string,
  payload: ContainerCreatePayload
): Promise<Container> => {
  if (useMocks) {
    const container: Container = {
      container_id: `mock-${Date.now()}`,
      code: payload.code,
      container_type: payload.container_type,
      capacity: payload.capacity ?? null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    mockContainers.unshift(container);
    return container;
  }
  return requestJson<Container>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/containers`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建容器失败'
  );
};

export const updateContainer = async (
  tenantId: string,
  containerId: string,
  payload: ContainerUpdatePayload
): Promise<Container> => {
  if (useMocks) {
    const index = mockContainers.findIndex((item) => item.container_id === containerId);
    const base =
      index >= 0
        ? mockContainers[index]
        : {
            container_id: containerId,
            code: containerId,
            container_type: payload.container_type ?? '未知',
            capacity: payload.capacity ?? null,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString()
          };
    const updated: Container = {
      ...base,
      container_type: payload.container_type ?? base.container_type,
      capacity: payload.capacity ?? base.capacity ?? null,
      updated_at: new Date().toISOString()
    };
    if (index >= 0) {
      mockContainers[index] = updated;
    } else {
      mockContainers.unshift(updated);
    }
    return updated;
  }
  return requestJson<Container>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/containers/${containerId}`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新容器失败'
  );
};

export const deleteContainer = async (
  tenantId: string,
  containerId: string
): Promise<Container> => {
  if (useMocks) {
    const index = mockContainers.findIndex((item) => item.container_id === containerId);
    if (index >= 0) {
      const [removed] = mockContainers.splice(index, 1);
      return removed;
    }
    return {
      container_id: containerId,
      code: containerId,
      container_type: '未知',
      capacity: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<Container>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/containers/${containerId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除容器失败'
  );
};
