import dayjs from 'dayjs';

import type {
  LogisticsExceptionType,
  LogisticsExceptionTypeCreatePayload,
  LogisticsExceptionTypeListQuery,
  LogisticsExceptionTypeUpdatePayload,
  LogisticsExceptionWorkOrder,
  LogisticsExceptionWorkOrderAssignPayload,
  LogisticsExceptionWorkOrderCreatePayload,
  LogisticsExceptionWorkOrderListQuery,
  LogisticsExceptionWorkOrderListResponse,
  LogisticsExceptionWorkOrderResolvePayload
} from '../logisticsExceptions';
import { applyPagination } from './data';

const mockTypes: LogisticsExceptionType[] = [
  {
    type_code: 'STUCK',
    name: '停滞未更新',
    sla_hours: 8,
    severity: 'MEDIUM',
    enabled: true,
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().toISOString()
  },
  {
    type_code: 'REFUSED',
    name: '拒收',
    sla_hours: 4,
    severity: 'HIGH',
    enabled: true,
    created_at: dayjs().subtract(9, 'day').toISOString(),
    updated_at: dayjs().toISOString()
  },
  {
    type_code: 'DAMAGED',
    name: '破损',
    sla_hours: 6,
    severity: 'HIGH',
    enabled: true,
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().toISOString()
  }
];

const mockWorkOrders: LogisticsExceptionWorkOrder[] = [
  {
    ticket_id: 'EX-2401',
    order_id: 'ORD-100230',
    tracking_no: 'SF1234567890',
    carrier_code: 'SF',
    exception_type: 'STUCK',
    status: 'OPEN',
    assignee: '物流客服A',
    created_at: dayjs().subtract(2, 'hour').toISOString(),
    due_at: dayjs().add(6, 'hour').toISOString(),
    escalation_level: 0,
    resolution: null,
    resolved_at: null
  },
  {
    ticket_id: 'EX-2402',
    order_id: 'ORD-100231',
    tracking_no: 'JD88990022',
    carrier_code: 'JD',
    exception_type: 'REFUSED',
    status: 'ESCALATED',
    assignee: '客服主管',
    created_at: dayjs().subtract(6, 'hour').toISOString(),
    due_at: dayjs().add(2, 'hour').toISOString(),
    escalation_level: 1,
    resolution: null,
    resolved_at: null
  }
];

export const listMockLogisticsExceptionTypes = (
  query: LogisticsExceptionTypeListQuery
): LogisticsExceptionType[] => {
  if (query.enabled == null) {
    return mockTypes.map((item) => ({ ...item }));
  }
  return mockTypes.filter((item) => item.enabled === query.enabled).map((item) => ({ ...item }));
};

export const createMockLogisticsExceptionType = (
  payload: LogisticsExceptionTypeCreatePayload
): LogisticsExceptionType => {
  const now = dayjs().toISOString();
  const record: LogisticsExceptionType = {
    type_code: payload.type_code,
    name: payload.name,
    sla_hours: payload.sla_hours ?? 6,
    severity: payload.severity ?? 'MEDIUM',
    enabled: payload.enabled ?? true,
    created_at: now,
    updated_at: now
  };
  const index = mockTypes.findIndex((item) => item.type_code === payload.type_code);
  if (index >= 0) {
    mockTypes[index] = record;
  } else {
    mockTypes.unshift(record);
  }
  return record;
};

export const updateMockLogisticsExceptionType = (
  typeCode: string,
  payload: LogisticsExceptionTypeUpdatePayload
): LogisticsExceptionType => {
  const target = mockTypes.find((item) => item.type_code === typeCode);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: LogisticsExceptionType = {
      type_code: typeCode,
      name: payload.name ?? '新异常类型',
      sla_hours: payload.sla_hours ?? 6,
      severity: payload.severity ?? 'MEDIUM',
      enabled: payload.enabled ?? true,
      created_at: now,
      updated_at: now
    };
    mockTypes.unshift(fallback);
    return fallback;
  }
  Object.assign(target, {
    name: payload.name ?? target.name,
    sla_hours: payload.sla_hours ?? target.sla_hours,
    severity: payload.severity ?? target.severity,
    enabled: payload.enabled ?? target.enabled,
    updated_at: now
  });
  return target;
};

export const listMockLogisticsExceptionWorkOrders = (
  query: LogisticsExceptionWorkOrderListQuery
): LogisticsExceptionWorkOrderListResponse => {
  const filtered = mockWorkOrders.filter((item) => {
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.exception_type && item.exception_type !== query.exception_type) {
      return false;
    }
    if (query.assignee && item.assignee !== query.assignee) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const createMockLogisticsExceptionWorkOrder = (
  payload: LogisticsExceptionWorkOrderCreatePayload
): LogisticsExceptionWorkOrder => {
  const now = dayjs().toISOString();
  const record: LogisticsExceptionWorkOrder = {
    ticket_id: `EX-${Date.now()}`,
    order_id: payload.order_id,
    tracking_no: payload.tracking_no,
    carrier_code: payload.carrier_code,
    exception_type: payload.exception_type,
    status: payload.status ?? 'OPEN',
    assignee: payload.assignee ?? null,
    created_at: now,
    due_at: payload.due_at ?? null,
    escalation_level: 0,
    resolution: null,
    resolved_at: null
  };
  mockWorkOrders.unshift(record);
  return record;
};

export const assignMockLogisticsExceptionWorkOrder = (
  ticketId: string,
  payload: LogisticsExceptionWorkOrderAssignPayload
): LogisticsExceptionWorkOrder => {
  const target = mockWorkOrders.find((item) => item.ticket_id === ticketId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: LogisticsExceptionWorkOrder = {
      ticket_id: ticketId,
      order_id: 'ORD-UNKNOWN',
      tracking_no: '-',
      carrier_code: '-',
      exception_type: 'STUCK',
      status: 'IN_PROGRESS',
      assignee: payload.assignee,
      created_at: now,
      due_at: dayjs().add(6, 'hour').toISOString(),
      escalation_level: 0,
      resolution: null,
      resolved_at: null
    };
    mockWorkOrders.unshift(fallback);
    return fallback;
  }
  target.assignee = payload.assignee;
  target.status = 'IN_PROGRESS';
  return target;
};

export const resolveMockLogisticsExceptionWorkOrder = (
  ticketId: string,
  payload: LogisticsExceptionWorkOrderResolvePayload
): LogisticsExceptionWorkOrder => {
  const target = mockWorkOrders.find((item) => item.ticket_id === ticketId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: LogisticsExceptionWorkOrder = {
      ticket_id: ticketId,
      order_id: 'ORD-UNKNOWN',
      tracking_no: '-',
      carrier_code: '-',
      exception_type: 'STUCK',
      status: 'RESOLVED',
      assignee: null,
      created_at: now,
      due_at: null,
      escalation_level: 0,
      resolution: payload.resolution,
      resolved_at: now
    };
    mockWorkOrders.unshift(fallback);
    return fallback;
  }
  target.status = 'RESOLVED';
  target.resolution = payload.resolution;
  target.resolved_at = now;
  return target;
};

export const escalateMockLogisticsExceptionWorkOrder = (
  ticketId: string
): LogisticsExceptionWorkOrder => {
  const target = mockWorkOrders.find((item) => item.ticket_id === ticketId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: LogisticsExceptionWorkOrder = {
      ticket_id: ticketId,
      order_id: 'ORD-UNKNOWN',
      tracking_no: '-',
      carrier_code: '-',
      exception_type: 'STUCK',
      status: 'ESCALATED',
      assignee: null,
      created_at: now,
      due_at: dayjs().add(4, 'hour').toISOString(),
      escalation_level: 1,
      resolution: null,
      resolved_at: null
    };
    mockWorkOrders.unshift(fallback);
    return fallback;
  }
  target.status = 'ESCALATED';
  target.escalation_level += 1;
  target.due_at = dayjs(target.due_at ?? now).add(4, 'hour').toISOString();
  return target;
};
