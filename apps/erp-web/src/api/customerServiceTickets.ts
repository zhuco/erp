import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import {
  createMockCustomerServiceTicket,
  deleteMockCustomerServiceTicket,
  getMockCustomerServiceTickets,
  updateMockCustomerServiceTicket
} from '@/api/mocks';

export type CustomerServiceTicketPriority = 'LOW' | 'MEDIUM' | 'HIGH';
export type CustomerServiceTicketStatus = 'OPEN' | 'IN_PROGRESS' | 'RESOLVED' | 'CLOSED';

export type CustomerServiceTicket = {
  ticket_id: string;
  customer_id: string;
  customer_name: string;
  contact_phone?: string | null;
  channel?: string | null;
  category?: string | null;
  priority?: CustomerServiceTicketPriority | null;
  status?: CustomerServiceTicketStatus | null;
  related_order_id?: string | null;
  satisfaction_score?: number | null;
  handler?: string | null;
  description?: string | null;
  created_at?: string | null;
  updated_at?: string | null;
};

export type CustomerServiceTicketListQuery = {
  keyword?: string;
  status?: CustomerServiceTicketStatus;
  priority?: CustomerServiceTicketPriority;
  channel?: string;
  category?: string;
  handler?: string;
  updated_from?: string;
  updated_to?: string;
  offset?: number;
  limit?: number;
};

export type CustomerServiceTicketListResponse = {
  items: CustomerServiceTicket[];
  total: number;
};

export type CustomerServiceTicketCreatePayload = {
  customer_id?: string;
  customer_name: string;
  contact_phone?: string;
  channel?: string;
  category?: string;
  priority?: CustomerServiceTicketPriority;
  status?: CustomerServiceTicketStatus;
  related_order_id?: string;
  satisfaction_score?: number | null;
  handler?: string;
  description?: string;
};

export type CustomerServiceTicketUpdatePayload = {
  customer_id?: string;
  customer_name?: string;
  contact_phone?: string;
  channel?: string;
  category?: string;
  priority?: CustomerServiceTicketPriority | null;
  status?: CustomerServiceTicketStatus | null;
  related_order_id?: string | null;
  satisfaction_score?: number | null;
  handler?: string | null;
  description?: string | null;
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

const buildQueryString = (query: CustomerServiceTicketListQuery): string => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.priority) {
    params.set('priority', query.priority);
  }
  if (query.channel) {
    params.set('channel', query.channel);
  }
  if (query.category) {
    params.set('category', query.category);
  }
  if (query.handler) {
    params.set('handler', query.handler);
  }
  if (query.updated_from) {
    params.set('updated_from', query.updated_from);
  }
  if (query.updated_to) {
    params.set('updated_to', query.updated_to);
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

export const listCustomerServiceTickets = async (
  tenantId: string,
  query: CustomerServiceTicketListQuery
): Promise<CustomerServiceTicketListResponse> => {
  if (useMocks) {
    return getMockCustomerServiceTickets(query);
  }
  return requestJson<CustomerServiceTicketListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/service-tickets${buildQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取客服工单失败'
  );
};

export const createCustomerServiceTicket = async (
  tenantId: string,
  payload: CustomerServiceTicketCreatePayload
): Promise<CustomerServiceTicket> => {
  if (useMocks) {
    return createMockCustomerServiceTicket(payload);
  }
  return requestJson<CustomerServiceTicket>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/service-tickets`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建客服工单失败'
  );
};

export const updateCustomerServiceTicket = async (
  tenantId: string,
  ticketId: string,
  payload: CustomerServiceTicketUpdatePayload
): Promise<CustomerServiceTicket> => {
  if (useMocks) {
    return updateMockCustomerServiceTicket(ticketId, payload);
  }
  return requestJson<CustomerServiceTicket>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/service-tickets/${ticketId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新客服工单失败'
  );
};

export const deleteCustomerServiceTicket = async (
  tenantId: string,
  ticketId: string
): Promise<CustomerServiceTicket> => {
  if (useMocks) {
    return deleteMockCustomerServiceTicket(ticketId);
  }
  return requestJson<CustomerServiceTicket>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/crm/service-tickets/${ticketId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除客服工单失败'
  );
};
