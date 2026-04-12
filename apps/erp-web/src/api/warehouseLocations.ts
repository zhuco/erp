import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type WarehouseLocation = {
  location_id: string;
  warehouse_id: string;
  area_id?: string | null;
  code: string;
  name?: string | null;
  barcode?: string | null;
  volume?: number | null;
  max_weight?: number | null;
  is_picking: boolean;
  is_disabled: boolean;
  created_at?: string | null;
  updated_at?: string | null;
};

export type WarehouseLocationListQuery = {
  warehouse_id?: string;
  area_id?: string;
  keyword?: string;
  is_picking?: boolean;
  is_disabled?: boolean;
  offset?: number;
  limit?: number;
};

export type WarehouseLocationListResponse = {
  items: WarehouseLocation[];
  total: number;
};

export type WarehouseLocationCreatePayload = {
  warehouse_id: string;
  area_id?: string;
  code: string;
  name?: string;
  barcode?: string;
  volume?: number | null;
  max_weight?: number | null;
  is_picking?: boolean;
  is_disabled?: boolean;
};

export type WarehouseLocationUpdatePayload = {
  area_id?: string | null;
  name?: string | null;
  barcode?: string | null;
  volume?: number | null;
  max_weight?: number | null;
  is_picking?: boolean;
  is_disabled?: boolean;
};

export type WarehouseLocationBatchItem = {
  area_id?: string;
  code: string;
  name?: string;
  barcode?: string;
  volume?: number | null;
  max_weight?: number | null;
  is_picking?: boolean;
  is_disabled?: boolean;
};

export type WarehouseLocationBatchPayload = {
  warehouse_id: string;
  area_id?: string;
  locations: WarehouseLocationBatchItem[];
};

const mockWarehouseLocations: WarehouseLocation[] = [
  {
    location_id: 'LOC-001',
    warehouse_id: 'WH-DEFAULT',
    area_id: 'AREA-001',
    code: 'A01-01',
    name: '货架 1-1',
    barcode: 'BAR-0001',
    volume: 1.2,
    max_weight: 50,
    is_picking: true,
    is_disabled: false,
    created_at: new Date(Date.now() - 86400000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString()
  },
  {
    location_id: 'LOC-002',
    warehouse_id: 'WH-DEFAULT',
    area_id: 'AREA-002',
    code: 'B02-01',
    name: '货架 2-1',
    barcode: 'BAR-0002',
    volume: 1.0,
    max_weight: 40,
    is_picking: false,
    is_disabled: false,
    created_at: new Date(Date.now() - 172800000).toISOString(),
    updated_at: new Date(Date.now() - 7200000).toISOString()
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

const buildQueryString = (query: WarehouseLocationListQuery): string => {
  const params = new URLSearchParams();
  if (query.warehouse_id) {
    params.set('warehouse_id', query.warehouse_id);
  }
  if (query.area_id) {
    params.set('area_id', query.area_id);
  }
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.is_picking !== undefined) {
    params.set('is_picking', String(query.is_picking));
  }
  if (query.is_disabled !== undefined) {
    params.set('is_disabled', String(query.is_disabled));
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

export const listWarehouseLocations = async (
  tenantId: string,
  query: WarehouseLocationListQuery
): Promise<WarehouseLocationListResponse> => {
  if (useMocks) {
    const keyword = query.keyword?.trim().toLowerCase();
    const filtered = mockWarehouseLocations.filter((location) => {
      if (query.warehouse_id && location.warehouse_id !== query.warehouse_id) {
        return false;
      }
      if (query.area_id && location.area_id !== query.area_id) {
        return false;
      }
      if (query.is_picking != null && location.is_picking !== query.is_picking) {
        return false;
      }
      if (query.is_disabled != null && location.is_disabled !== query.is_disabled) {
        return false;
      }
      if (!keyword) {
        return true;
      }
      return (
        location.code.toLowerCase().includes(keyword) ||
        (location.name ?? '').toLowerCase().includes(keyword) ||
        (location.barcode ?? '').toLowerCase().includes(keyword)
      );
    });
    const offset = query.offset ?? 0;
    const limit = query.limit ?? 20;
    return {
      items: filtered.slice(offset, offset + limit),
      total: filtered.length
    };
  }
  return requestJson<WarehouseLocationListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-locations${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取货位列表失败'
  );
};

export const createWarehouseLocation = async (
  tenantId: string,
  payload: WarehouseLocationCreatePayload
): Promise<WarehouseLocation> => {
  if (useMocks) {
    const record: WarehouseLocation = {
      location_id: `mock-${Date.now()}`,
      warehouse_id: payload.warehouse_id,
      area_id: payload.area_id ?? null,
      code: payload.code,
      name: payload.name ?? null,
      barcode: payload.barcode ?? null,
      volume: payload.volume ?? null,
      max_weight: payload.max_weight ?? null,
      is_picking: payload.is_picking ?? false,
      is_disabled: payload.is_disabled ?? false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    mockWarehouseLocations.unshift(record);
    return record;
  }
  return requestJson<WarehouseLocation>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-locations`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建货位失败'
  );
};

export const batchCreateWarehouseLocations = async (
  tenantId: string,
  payload: WarehouseLocationBatchPayload
): Promise<WarehouseLocationListResponse> => {
  if (useMocks) {
    const created = payload.locations.map((item) => {
      const record: WarehouseLocation = {
        location_id: `mock-${Date.now()}-${item.code}`,
        warehouse_id: payload.warehouse_id,
        area_id: item.area_id ?? payload.area_id ?? null,
        code: item.code,
        name: item.name ?? null,
        barcode: item.barcode ?? null,
        volume: item.volume ?? null,
        max_weight: item.max_weight ?? null,
        is_picking: item.is_picking ?? false,
        is_disabled: item.is_disabled ?? false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString()
      };
      mockWarehouseLocations.unshift(record);
      return record;
    });
    return { items: created, total: created.length };
  }
  return requestJson<WarehouseLocationListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-locations/batch`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '批量导入货位失败'
  );
};

export const updateWarehouseLocation = async (
  tenantId: string,
  locationId: string,
  payload: WarehouseLocationUpdatePayload
): Promise<WarehouseLocation> => {
  if (useMocks) {
    const index = mockWarehouseLocations.findIndex(
      (location) => location.location_id === locationId
    );
    const base =
      index >= 0
        ? mockWarehouseLocations[index]
        : {
            location_id: locationId,
            warehouse_id: 'WH-DEFAULT',
            area_id: payload.area_id ?? null,
            code: locationId,
            name: payload.name ?? null,
            barcode: payload.barcode ?? null,
            volume: payload.volume ?? null,
            max_weight: payload.max_weight ?? null,
            is_picking: payload.is_picking ?? false,
            is_disabled: payload.is_disabled ?? false,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString()
          };
    const updated: WarehouseLocation = {
      ...base,
      area_id: payload.area_id ?? base.area_id ?? null,
      name: payload.name ?? base.name ?? null,
      barcode: payload.barcode ?? base.barcode ?? null,
      volume: payload.volume ?? base.volume ?? null,
      max_weight: payload.max_weight ?? base.max_weight ?? null,
      is_picking: payload.is_picking ?? base.is_picking,
      is_disabled: payload.is_disabled ?? base.is_disabled,
      updated_at: new Date().toISOString()
    };
    if (index >= 0) {
      mockWarehouseLocations[index] = updated;
    } else {
      mockWarehouseLocations.unshift(updated);
    }
    return updated;
  }
  return requestJson<WarehouseLocation>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-locations/${locationId}`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新货位失败'
  );
};

export const deleteWarehouseLocation = async (
  tenantId: string,
  locationId: string
): Promise<WarehouseLocation> => {
  if (useMocks) {
    const index = mockWarehouseLocations.findIndex(
      (location) => location.location_id === locationId
    );
    if (index >= 0) {
      const [removed] = mockWarehouseLocations.splice(index, 1);
      return removed;
    }
    return {
      location_id: locationId,
      warehouse_id: 'WH-DEFAULT',
      area_id: null,
      code: locationId,
      name: null,
      barcode: null,
      volume: null,
      max_weight: null,
      is_picking: false,
      is_disabled: false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<WarehouseLocation>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/warehouse-locations/${locationId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除货位失败'
  );
};
