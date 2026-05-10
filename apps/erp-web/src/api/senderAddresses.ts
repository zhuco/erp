import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type SenderAddress = {
  sender_address_id: string;
  name: string;
  phone: string;
  province: string;
  city: string;
  district: string;
  address: string;
  is_default: boolean;
  created_at?: string | null;
  updated_at?: string | null;
};

export type SenderAddressListQuery = {
  keyword?: string;
  offset?: number;
  limit?: number;
};

export type SenderAddressListResponse = {
  items: SenderAddress[];
  total: number;
};

export type SenderAddressCreatePayload = {
  name: string;
  phone: string;
  province: string;
  city: string;
  district: string;
  address: string;
  is_default?: boolean;
};

export type SenderAddressUpdatePayload = {
  name?: string;
  phone?: string;
  province?: string;
  city?: string;
  district?: string;
  address?: string;
  is_default?: boolean;
};

const mockSenderAddresses: SenderAddress[] = [
  {
    sender_address_id: 'SENDER-001',
    name: '默认寄件人',
    phone: '13800001234',
    province: '上海',
    city: '上海市',
    district: '浦东新区',
    address: '张江路 88 号',
    is_default: true,
    created_at: new Date(Date.now() - 86400000).toISOString(),
    updated_at: new Date(Date.now() - 3600000).toISOString()
  },
  {
    sender_address_id: 'SENDER-002',
    name: '华东仓寄方',
    phone: '13700005678',
    province: '江苏',
    city: '苏州市',
    district: '工业园区',
    address: '苏虹中路 18 号',
    is_default: false,
    created_at: new Date(Date.now() - 172800000).toISOString(),
    updated_at: new Date(Date.now() - 7200000).toISOString()
  },
  {
    sender_address_id: 'SENDER-003',
    name: '华南仓寄方',
    phone: '13600002345',
    province: '广东',
    city: '广州市',
    district: '白云区',
    address: '机场路 168 号',
    is_default: false,
    created_at: new Date(Date.now() - 259200000).toISOString(),
    updated_at: new Date(Date.now() - 10800000).toISOString()
  },
  {
    sender_address_id: 'SENDER-004',
    name: '华北仓寄方',
    phone: '13500003456',
    province: '北京',
    city: '北京市',
    district: '顺义区',
    address: '临空大道 66 号',
    is_default: false,
    created_at: new Date(Date.now() - 345600000).toISOString(),
    updated_at: new Date(Date.now() - 14400000).toISOString()
  },
  {
    sender_address_id: 'SENDER-005',
    name: '退货中心',
    phone: '13900004567',
    province: '浙江',
    city: '杭州市',
    district: '钱塘区',
    address: '海拓路 28 号',
    is_default: false,
    created_at: new Date(Date.now() - 432000000).toISOString(),
    updated_at: new Date(Date.now() - 18000000).toISOString()
  },
  {
    sender_address_id: 'SENDER-006',
    name: '跨境保税仓寄方',
    phone: '13400005678',
    province: '广东',
    city: '深圳市',
    district: '盐田区',
    address: '保税一路 9 号',
    is_default: false,
    created_at: new Date(Date.now() - 518400000).toISOString(),
    updated_at: new Date(Date.now() - 21600000).toISOString()
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

const buildQueryString = (query: SenderAddressListQuery): string => {
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

export const listSenderAddresses = async (
  tenantId: string,
  query: SenderAddressListQuery
): Promise<SenderAddressListResponse> => {
  if (useMocks) {
    const keyword = query.keyword?.trim().toLowerCase();
    const filtered = mockSenderAddresses.filter((item) => {
      if (!keyword) {
        return true;
      }
      return (
        item.name.toLowerCase().includes(keyword) ||
        item.phone.includes(keyword) ||
        `${item.province}${item.city}${item.district}${item.address}`.toLowerCase().includes(keyword)
      );
    });
    const offset = query.offset ?? 0;
    const limit = query.limit ?? 20;
    return {
      items: filtered.slice(offset, offset + limit),
      total: filtered.length
    };
  }
  return requestJson<SenderAddressListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sender-addresses${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取寄方地址失败'
  );
};

export const createSenderAddress = async (
  tenantId: string,
  payload: SenderAddressCreatePayload
): Promise<SenderAddress> => {
  if (useMocks) {
    const record: SenderAddress = {
      sender_address_id: `mock-${Date.now()}`,
      name: payload.name,
      phone: payload.phone,
      province: payload.province,
      city: payload.city,
      district: payload.district,
      address: payload.address,
      is_default: payload.is_default ?? false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
    if (record.is_default) {
      mockSenderAddresses.forEach((item) => {
        item.is_default = false;
      });
    }
    mockSenderAddresses.unshift(record);
    return record;
  }
  return requestJson<SenderAddress>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sender-addresses`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建寄方地址失败'
  );
};

export const updateSenderAddress = async (
  tenantId: string,
  senderAddressId: string,
  payload: SenderAddressUpdatePayload
): Promise<SenderAddress> => {
  if (useMocks) {
    const index = mockSenderAddresses.findIndex(
      (item) => item.sender_address_id === senderAddressId
    );
    const base =
      index >= 0
        ? mockSenderAddresses[index]
        : {
            sender_address_id: senderAddressId,
            name: payload.name ?? '未知',
            phone: payload.phone ?? '',
            province: payload.province ?? '',
            city: payload.city ?? '',
            district: payload.district ?? '',
            address: payload.address ?? '',
            is_default: payload.is_default ?? false,
            created_at: new Date().toISOString(),
            updated_at: new Date().toISOString()
          };
    const updated: SenderAddress = {
      ...base,
      name: payload.name ?? base.name,
      phone: payload.phone ?? base.phone,
      province: payload.province ?? base.province,
      city: payload.city ?? base.city,
      district: payload.district ?? base.district,
      address: payload.address ?? base.address,
      is_default: payload.is_default ?? base.is_default,
      updated_at: new Date().toISOString()
    };
    if (updated.is_default) {
      mockSenderAddresses.forEach((item) => {
        item.is_default = item.sender_address_id === updated.sender_address_id;
      });
    }
    if (index >= 0) {
      mockSenderAddresses[index] = updated;
    } else {
      mockSenderAddresses.unshift(updated);
    }
    return updated;
  }
  return requestJson<SenderAddress>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sender-addresses/${senderAddressId}`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新寄方地址失败'
  );
};

export const deleteSenderAddress = async (
  tenantId: string,
  senderAddressId: string
): Promise<SenderAddress> => {
  if (useMocks) {
    const index = mockSenderAddresses.findIndex(
      (item) => item.sender_address_id === senderAddressId
    );
    if (index >= 0) {
      const [removed] = mockSenderAddresses.splice(index, 1);
      return removed;
    }
    return {
      sender_address_id: senderAddressId,
      name: '未知',
      phone: '',
      province: '',
      city: '',
      district: '',
      address: '',
      is_default: false,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<SenderAddress>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/sender-addresses/${senderAddressId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除寄方地址失败'
  );
};
