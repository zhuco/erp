import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type WarehouseArea = {
  area_id: string;
  warehouse_id: string;
  code: string;
  name: string;
  area_type: string;
  status: string;
  priority: number;
  created_at?: string | null;
  updated_at?: string | null;
};

export type WarehouseAreaListQuery = {
  warehouse_id?: string;
  area_type?: string;
  status?: string;
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type WarehouseAreaListResponse = {
  items: WarehouseArea[];
  total: number;
};

export type WarehouseAreaCreatePayload = {
  warehouse_id: string;
  code: string;
  name: string;
  area_type?: string;
  status?: string;
  priority?: number;
};

export type WarehouseAreaUpdatePayload = {
  name?: string;
  area_type?: string;
  status?: string;
  priority?: number;
};

const mockWarehouseAreas: WarehouseArea[] = [
  {
    area_id: 'AREA-001',
    warehouse_id: 'WH-DEFAULT',
    code: 'A01',
    name: '常温存储区',
    area_type: 'STORAGE',
    status: 'ACTIVE',
    priority: 1,
    created_at: new Date(Date.now() - 86400000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString()
  },
  {
    area_id: 'AREA-002',
    warehouse_id: 'WH-DEFAULT',
    code: 'B02',
    name: '拣货区',
    area_type: 'PICKING',
    status: 'ACTIVE',
    priority: 2,
    created_at: new Date(Date.now() - 172800000).toISOString(),
    updated_at: new Date(Date.now() - 7200000).toISOString()
  },
  {
    area_id: 'AREA-003',
    warehouse_id: 'WH-EAST',
    code: 'RCV-01',
    name: '收货暂存区',
    area_type: 'RECEIVING',
    status: 'ACTIVE',
    priority: 1,
    created_at: new Date(Date.now() - 259200000).toISOString(),
    updated_at: new Date(Date.now() - 10800000).toISOString()
  },
  {
    area_id: 'AREA-004',
    warehouse_id: 'WH-EAST',
    code: 'SHP-01',
    name: '发货复核区',
    area_type: 'SHIPPING',
    status: 'ACTIVE',
    priority: 2,
    created_at: new Date(Date.now() - 345600000).toISOString(),
    updated_at: new Date(Date.now() - 14400000).toISOString()
  },
  {
    area_id: 'AREA-005',
    warehouse_id: 'WH-SOUTH',
    code: 'STO-02',
    name: '快消品存储区',
    area_type: 'STORAGE',
    status: 'ACTIVE',
    priority: 3,
    created_at: new Date(Date.now() - 432000000).toISOString(),
    updated_at: new Date(Date.now() - 18000000).toISOString()
  },
  {
    area_id: 'AREA-006',
    warehouse_id: 'WH-NORTH',
    code: 'PICK-02',
    name: '直播拣选区',
    area_type: 'PICKING',
    status: 'ACTIVE',
    priority: 2,
    created_at: new Date(Date.now() - 518400000).toISOString(),
    updated_at: new Date(Date.now() - 21600000).toISOString()
  },
  {
    area_id: 'AREA-007',
    warehouse_id: 'WH-RETURN',
    code: 'QA-01',
    name: '退货质检区',
    area_type: 'RECEIVING',
    status: 'DISABLED',
    priority: 4,
    created_at: new Date(Date.now() - 604800000).toISOString(),
    updated_at: new Date(Date.now() - 25200000).toISOString()
  },
  {
    area_id: 'AREA-008',
    warehouse_id: 'WH-CROSS',
    code: 'BOND-01',
    name: '保税备货区',
    area_type: 'STORAGE',
    status: 'ACTIVE',
    priority: 1,
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

const buildQueryString = (query: WarehouseAreaListQuery): string => {
  const params = new URLSearchParams();
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.area_type) {
    params.set('area_type', query.area_type);
  }
  if (query.status) {
    params.set('status', query.status);
  }
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

export const listWarehouseAreas = async (
  tenantId: string,
  query: WarehouseAreaListQuery
): Promise<WarehouseAreaListResponse> => {
  if (useMocks) {
    const keyword = query.keyword?.trim().toLowerCase();
    const filtered = mockWarehouseAreas.filter((area) => {
      if (query.warehouse_id && area.warehouse_id !== query.warehouse_id) {
        return false;
      }
      if (query.area_type && area.area_type !== query.area_type) {
        return false;
      }
      if (query.status && area.status !== query.status) {
        return false;
      }
      if (!keyword) {
        return true;
      }
      return (
        area.name.toLowerCase().includes(keyword) ||
        area.code.toLowerCase().includes(keyword)
      );
    });
    const offset = query.offset ?? 0;
    const limit = query.limit ?? 20;
    return {
      items: filtered.slice(offset, offset + limit),
      total: filtered.length
    };
  }
  return requestJson<WarehouseAreaListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-areas${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取库区列表失败'
  );
};

export const createWarehouseArea = async (
  tenantId: string,
  payload: WarehouseAreaCreatePayload
): Promise<WarehouseArea> => {
  if (useMocks) {
    const record: WarehouseArea = {
      area_id: `mock-${Date.now()}`,
      warehouse_id: payload.warehouse_id,
      code: payload.code,
      name: payload.name,
      area_type: payload.area_type ?? 'STORAGE',
      status: payload.status ?? 'ACTIVE',
      priority: payload.priority ?? 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    mockWarehouseAreas.unshift(record);
    return record;
  }
  return requestJson<WarehouseArea>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-areas`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建库区失败'
  );
};

export const updateWarehouseArea = async (
  tenantId: string,
  areaId: string,
  payload: WarehouseAreaUpdatePayload
): Promise<WarehouseArea> => {
  if (useMocks) {
    const index = mockWarehouseAreas.findIndex((area) => area.area_id === areaId);
    const base =
      index >= 0
        ? mockWarehouseAreas[index]
        : {
            area_id: areaId,
            warehouse_id: 'WH-DEFAULT',
            code: areaId,
            name: payload.name ?? '未知库区',
            area_type: payload.area_type ?? 'STORAGE',
            status: payload.status ?? 'ACTIVE',
            priority: payload.priority ?? 0,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString()
          };
    const updated: WarehouseArea = {
      ...base,
      name: payload.name ?? base.name,
      area_type: payload.area_type ?? base.area_type,
      status: payload.status ?? base.status,
      priority: payload.priority ?? base.priority,
      updated_at: new Date().toISOString()
    };
    if (index >= 0) {
      mockWarehouseAreas[index] = updated;
    } else {
      mockWarehouseAreas.unshift(updated);
    }
    return updated;
  }
  return requestJson<WarehouseArea>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-areas/${areaId}`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新库区失败'
  );
};

export const deleteWarehouseArea = async (
  tenantId: string,
  areaId: string
): Promise<WarehouseArea> => {
  if (useMocks) {
    const index = mockWarehouseAreas.findIndex((area) => area.area_id === areaId);
    if (index >= 0) {
      const [removed] = mockWarehouseAreas.splice(index, 1);
      return removed;
    }
    return {
      area_id: areaId,
      warehouse_id: 'WH-DEFAULT',
      code: areaId,
      name: '未知库区',
      area_type: 'STORAGE',
      status: 'DISABLED',
      priority: 0,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<WarehouseArea>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-areas/${areaId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除库区失败'
  );
};
