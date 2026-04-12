import dayjs from 'dayjs';

import type {
  CustomerLifecycleTask,
  CustomerLifecycleTaskCreatePayload,
  CustomerLifecycleTaskListQuery,
  CustomerLifecycleTaskListResponse,
  CustomerLifecycleTaskUpdatePayload
} from '@/api/customerLifecycle';
import type {
  CustomerPointLedger,
  CustomerPointLedgerListQuery,
  CustomerPointLedgerListResponse,
  CustomerStoredValueLedger,
  CustomerStoredValueLedgerListQuery,
  CustomerStoredValueLedgerListResponse
} from '@/api/customerLoyalty';
import type {
  CustomerSegment,
  CustomerSegmentCreatePayload,
  CustomerSegmentListQuery,
  CustomerSegmentListResponse,
  CustomerSegmentUpdatePayload
} from '@/api/customerSegments';
import type {
  CustomerServiceTicket,
  CustomerServiceTicketCreatePayload,
  CustomerServiceTicketListQuery,
  CustomerServiceTicketListResponse,
  CustomerServiceTicketUpdatePayload
} from '@/api/customerServiceTickets';

import { applyPagination, mockCustomers } from './data';

const customerMap = new Map(mockCustomers.map((customer) => [customer.customer_id, customer]));

const mockPointLedgers: CustomerPointLedger[] = mockCustomers.slice(0, 18).map((customer, index) => {
  const sign = index % 3 === 0 ? -1 : 1;
  const change = (index + 1) * 20 * sign;
  const balance = 1200 + index * 35 + change;
  return {
    ledger_id: `POINT-${index + 1}`,
    customer_id: customer.customer_id,
    customer_name: customer.name,
    change_points: change,
    balance_points: Math.max(balance, 0),
    reason: sign > 0 ? '订单完成' : '退款回滚',
    ref_type: 'ORDER',
    ref_id: `ORDER-${1000 + index}`,
    created_at: dayjs().subtract(index, 'day').toISOString()
  };
});

const mockStoredValueLedgers: CustomerStoredValueLedger[] = mockCustomers
  .slice(0, 16)
  .map((customer, index) => {
    const sign = index % 4 === 0 ? -1 : 1;
    const change = (index + 2) * 5000 * sign;
    const balance = 500000 + index * 2500 + change;
    return {
      ledger_id: `VALUE-${index + 1}`,
      customer_id: customer.customer_id,
      customer_name: customer.name,
      change_cent: change,
      balance_cent: Math.max(balance, 0),
      reason: sign > 0 ? '储值充值' : '订单抵扣',
      ref_type: sign > 0 ? 'RECHARGE' : 'ORDER',
      ref_id: sign > 0 ? `RECHARGE-${2000 + index}` : `ORDER-${2000 + index}`,
      created_at: dayjs().subtract(index * 2, 'day').toISOString()
    };
  });

const mockSegments: CustomerSegment[] = [
  {
    segment_id: 'SEG-1001',
    name: '高价值客户',
    segment_type: 'DYNAMIC',
    rule_summary: '近90天消费金额>5000',
    customer_count: 128,
    status: 'ACTIVE',
    created_at: dayjs().subtract(14, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    segment_id: 'SEG-1002',
    name: '高退货率客户',
    segment_type: 'DYNAMIC',
    rule_summary: '近30天退货率>20%',
    customer_count: 32,
    status: 'ACTIVE',
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(3, 'day').toISOString()
  },
  {
    segment_id: 'SEG-1003',
    name: '金卡会员名单',
    segment_type: 'STATIC',
    rule_summary: '手动维护金卡客户名单',
    customer_count: 56,
    status: 'ACTIVE',
    created_at: dayjs().subtract(20, 'day').toISOString(),
    updated_at: dayjs().subtract(5, 'day').toISOString()
  },
  {
    segment_id: 'SEG-1004',
    name: '沉默客户提醒',
    segment_type: 'DYNAMIC',
    rule_summary: '60天无复购',
    customer_count: 210,
    status: 'INACTIVE',
    created_at: dayjs().subtract(25, 'day').toISOString(),
    updated_at: dayjs().subtract(7, 'day').toISOString()
  }
];

const mockLifecycleTasks: CustomerLifecycleTask[] = [
  {
    task_id: 'LIFE-1001',
    stage: 'NEW',
    strategy: '新客首单引导短信',
    trigger_rule: '注册后24小时未下单',
    owner: '客户运营组',
    status: 'ACTIVE',
    last_triggered_at: dayjs().subtract(2, 'hour').toISOString(),
    created_at: dayjs().subtract(12, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  },
  {
    task_id: 'LIFE-1002',
    stage: 'ACTIVE',
    strategy: '活跃客户积分加速',
    trigger_rule: '30天内订单>=3',
    owner: '会员运营组',
    status: 'ACTIVE',
    last_triggered_at: dayjs().subtract(1, 'day').toISOString(),
    created_at: dayjs().subtract(9, 'day').toISOString(),
    updated_at: dayjs().subtract(6, 'hour').toISOString()
  },
  {
    task_id: 'LIFE-1003',
    stage: 'DORMANT',
    strategy: '沉默客户召回券提醒',
    trigger_rule: '60天无下单',
    owner: '客户运营组',
    status: 'ACTIVE',
    last_triggered_at: dayjs().subtract(3, 'day').toISOString(),
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    task_id: 'LIFE-1004',
    stage: 'CHURNED',
    strategy: '流失客户调研回访',
    trigger_rule: '90天无下单',
    owner: '客服主管',
    status: 'INACTIVE',
    last_triggered_at: null,
    created_at: dayjs().subtract(20, 'day').toISOString(),
    updated_at: dayjs().subtract(5, 'day').toISOString()
  }
];

const mockServiceTickets: CustomerServiceTicket[] = mockCustomers.slice(0, 12).map((customer, index) => {
  const status =
    index % 4 === 0 ? 'OPEN' : index % 4 === 1 ? 'IN_PROGRESS' : index % 4 === 2 ? 'RESOLVED' : 'CLOSED';
  const priority = index % 3 === 0 ? 'HIGH' : index % 3 === 1 ? 'MEDIUM' : 'LOW';
  return {
    ticket_id: `CS-${1000 + index}`,
    customer_id: customer.customer_id,
    customer_name: customer.name,
    contact_phone: customer.phone ?? null,
    channel: index % 2 === 0 ? '电话' : '在线',
    category: index % 3 === 0 ? '投诉' : index % 3 === 1 ? '咨询' : '售后',
    priority,
    status,
    related_order_id: `ORDER-${200 + index}`,
    satisfaction_score: status === 'RESOLVED' || status === 'CLOSED' ? 4 - (index % 3) : null,
    handler: index % 2 === 0 ? '客服-李雷' : '客服-韩梅梅',
    description: index % 3 === 0 ? '物流异常跟进' : index % 3 === 1 ? '商品咨询' : '售后处理',
    created_at: dayjs().subtract(index + 1, 'day').toISOString(),
    updated_at: dayjs().subtract(index, 'hour').toISOString()
  };
});

const matchKeyword = (keyword: string | undefined, value: string | null | undefined): boolean => {
  if (!keyword) {
    return true;
  }
  const normalized = keyword.trim().toLowerCase();
  if (!normalized) {
    return true;
  }
  return (value ?? '').toLowerCase().includes(normalized);
};

const withinDateRange = (value: string, from?: string, to?: string): boolean => {
  const time = dayjs(value);
  if (from && time.isBefore(dayjs(from))) {
    return false;
  }
  if (to && time.isAfter(dayjs(to))) {
    return false;
  }
  return true;
};

export const getMockCustomerPointLedgers = (
  query: CustomerPointLedgerListQuery
): CustomerPointLedgerListResponse => {
  const filtered = mockPointLedgers.filter((ledger) => {
    if (!withinDateRange(ledger.created_at, query.created_from, query.created_to)) {
      return false;
    }
    if (query.customer_id && ledger.customer_id !== query.customer_id) {
      return false;
    }
    if (query.keyword) {
      const customer = customerMap.get(ledger.customer_id);
      const matchesCustomer =
        matchKeyword(query.keyword, ledger.customer_name) ||
        matchKeyword(query.keyword, ledger.customer_id) ||
        matchKeyword(query.keyword, customer?.phone ?? null);
      if (!matchesCustomer) {
        return false;
      }
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const getMockCustomerStoredValueLedgers = (
  query: CustomerStoredValueLedgerListQuery
): CustomerStoredValueLedgerListResponse => {
  const filtered = mockStoredValueLedgers.filter((ledger) => {
    if (!withinDateRange(ledger.created_at, query.created_from, query.created_to)) {
      return false;
    }
    if (query.customer_id && ledger.customer_id !== query.customer_id) {
      return false;
    }
    if (query.keyword) {
      const customer = customerMap.get(ledger.customer_id);
      const matchesCustomer =
        matchKeyword(query.keyword, ledger.customer_name) ||
        matchKeyword(query.keyword, ledger.customer_id) ||
        matchKeyword(query.keyword, customer?.phone ?? null);
      if (!matchesCustomer) {
        return false;
      }
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const getMockCustomerSegmentList = (
  query: CustomerSegmentListQuery
): CustomerSegmentListResponse => {
  const filtered = mockSegments.filter((segment) => {
    if (query.segment_type && segment.segment_type !== query.segment_type) {
      return false;
    }
    if (query.status && segment.status !== query.status) {
      return false;
    }
    if (query.keyword && !matchKeyword(query.keyword, segment.name)) {
      return false;
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockCustomerSegment = (
  payload: CustomerSegmentCreatePayload
): CustomerSegment => {
  const now = dayjs().toISOString();
  const segment: CustomerSegment = {
    segment_id: `SEG-${Date.now()}`,
    name: payload.name,
    segment_type: payload.segment_type,
    rule_summary: payload.rule_summary ?? null,
    customer_count: Math.floor(40 + Math.random() * 200),
    status: payload.status ?? 'ACTIVE',
    created_at: now,
    updated_at: now
  };
  mockSegments.unshift(segment);
  return segment;
};

export const updateMockCustomerSegment = (
  segmentId: string,
  payload: CustomerSegmentUpdatePayload
): CustomerSegment => {
  const now = dayjs().toISOString();
  const index = mockSegments.findIndex((segment) => segment.segment_id === segmentId);
  const base =
    index >= 0
      ? mockSegments[index]
      : ({
          segment_id: segmentId,
          name: payload.name ?? segmentId,
          segment_type: payload.segment_type ?? 'DYNAMIC',
          customer_count: 0,
          created_at: now
        } as CustomerSegment);
  const updated: CustomerSegment = {
    ...base,
    name: payload.name ?? base.name,
    segment_type: payload.segment_type ?? base.segment_type,
    rule_summary: payload.rule_summary ?? base.rule_summary ?? null,
    status: payload.status ?? base.status ?? 'ACTIVE',
    updated_at: now
  };
  if (index >= 0) {
    mockSegments[index] = updated;
  } else {
    mockSegments.unshift(updated);
  }
  return updated;
};

export const deleteMockCustomerSegment = (segmentId: string): CustomerSegment => {
  const index = mockSegments.findIndex((segment) => segment.segment_id === segmentId);
  if (index >= 0) {
    const [removed] = mockSegments.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    segment_id: segmentId,
    name: '未命名分群',
    segment_type: 'STATIC',
    rule_summary: null,
    customer_count: 0,
    status: 'INACTIVE',
    created_at: now,
    updated_at: now
  };
};

export const getMockCustomerLifecycleTasks = (
  query: CustomerLifecycleTaskListQuery
): CustomerLifecycleTaskListResponse => {
  const filtered = mockLifecycleTasks.filter((task) => {
    if (query.stage && task.stage !== query.stage) {
      return false;
    }
    if (query.status && task.status !== query.status) {
      return false;
    }
    if (query.keyword) {
      const matches =
        matchKeyword(query.keyword, task.strategy) ||
        matchKeyword(query.keyword, task.owner ?? null);
      if (!matches) {
        return false;
      }
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockCustomerLifecycleTask = (
  payload: CustomerLifecycleTaskCreatePayload
): CustomerLifecycleTask => {
  const now = dayjs().toISOString();
  const task: CustomerLifecycleTask = {
    task_id: `LIFE-${Date.now()}`,
    stage: payload.stage,
    strategy: payload.strategy,
    trigger_rule: payload.trigger_rule ?? null,
    owner: payload.owner ?? null,
    status: payload.status ?? 'ACTIVE',
    last_triggered_at: null,
    created_at: now,
    updated_at: now
  };
  mockLifecycleTasks.unshift(task);
  return task;
};

export const updateMockCustomerLifecycleTask = (
  taskId: string,
  payload: CustomerLifecycleTaskUpdatePayload
): CustomerLifecycleTask => {
  const now = dayjs().toISOString();
  const index = mockLifecycleTasks.findIndex((task) => task.task_id === taskId);
  const base =
    index >= 0
      ? mockLifecycleTasks[index]
      : ({
          task_id: taskId,
          stage: payload.stage ?? 'NEW',
          strategy: payload.strategy ?? taskId,
          created_at: now
        } as CustomerLifecycleTask);
  const updated: CustomerLifecycleTask = {
    ...base,
    stage: payload.stage ?? base.stage,
    strategy: payload.strategy ?? base.strategy,
    trigger_rule: payload.trigger_rule ?? base.trigger_rule ?? null,
    owner: payload.owner ?? base.owner ?? null,
    status: payload.status ?? base.status ?? 'ACTIVE',
    updated_at: now
  };
  if (index >= 0) {
    mockLifecycleTasks[index] = updated;
  } else {
    mockLifecycleTasks.unshift(updated);
  }
  return updated;
};

export const deleteMockCustomerLifecycleTask = (taskId: string): CustomerLifecycleTask => {
  const index = mockLifecycleTasks.findIndex((task) => task.task_id === taskId);
  if (index >= 0) {
    const [removed] = mockLifecycleTasks.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    task_id: taskId,
    stage: 'CHURNED',
    strategy: '未知策略',
    trigger_rule: null,
    owner: null,
    status: 'INACTIVE',
    created_at: now,
    updated_at: now,
    last_triggered_at: null
  };
};

export const getMockCustomerServiceTickets = (
  query: CustomerServiceTicketListQuery
): CustomerServiceTicketListResponse => {
  const filtered = mockServiceTickets.filter((ticket) => {
    if (query.status && ticket.status !== query.status) {
      return false;
    }
    if (query.priority && ticket.priority !== query.priority) {
      return false;
    }
    if (query.channel && ticket.channel !== query.channel) {
      return false;
    }
    if (query.category && ticket.category !== query.category) {
      return false;
    }
    if (query.handler && ticket.handler !== query.handler) {
      return false;
    }
    if (query.updated_from || query.updated_to) {
      const referenceTime = ticket.updated_at ?? ticket.created_at;
      if (!referenceTime) {
        return false;
      }
      if (!withinDateRange(referenceTime, query.updated_from, query.updated_to)) {
        return false;
      }
    }
    if (query.keyword) {
      const matches =
        matchKeyword(query.keyword, ticket.customer_name) ||
        matchKeyword(query.keyword, ticket.ticket_id) ||
        matchKeyword(query.keyword, ticket.related_order_id ?? null);
      if (!matches) {
        return false;
      }
    }
    return true;
  });
  const items = applyPagination(filtered, query.offset, query.limit);
  return {
    items,
    total: filtered.length
  };
};

export const createMockCustomerServiceTicket = (
  payload: CustomerServiceTicketCreatePayload
): CustomerServiceTicket => {
  const now = dayjs().toISOString();
  const customer = payload.customer_id ? customerMap.get(payload.customer_id) : undefined;
  const ticket: CustomerServiceTicket = {
    ticket_id: `CS-${Date.now()}`,
    customer_id: payload.customer_id ?? customer?.customer_id ?? `CUST-${Date.now()}`,
    customer_name: payload.customer_name ?? customer?.name ?? '未知客户',
    contact_phone: payload.contact_phone ?? customer?.phone ?? null,
    channel: payload.channel ?? '在线',
    category: payload.category ?? '咨询',
    priority: payload.priority ?? 'MEDIUM',
    status: payload.status ?? 'OPEN',
    related_order_id: payload.related_order_id ?? null,
    satisfaction_score: payload.satisfaction_score ?? null,
    handler: payload.handler ?? null,
    description: payload.description ?? null,
    created_at: now,
    updated_at: now
  };
  mockServiceTickets.unshift(ticket);
  return ticket;
};

export const updateMockCustomerServiceTicket = (
  ticketId: string,
  payload: CustomerServiceTicketUpdatePayload
): CustomerServiceTicket => {
  const now = dayjs().toISOString();
  const index = mockServiceTickets.findIndex((ticket) => ticket.ticket_id === ticketId);
  const base =
    index >= 0
      ? mockServiceTickets[index]
      : ({
          ticket_id: ticketId,
          customer_id: payload.customer_id ?? `CUST-${Date.now()}`,
          customer_name: payload.customer_name ?? '未知客户',
          created_at: now
        } as CustomerServiceTicket);
  const updated: CustomerServiceTicket = {
    ...base,
    customer_id: payload.customer_id ?? base.customer_id,
    customer_name: payload.customer_name ?? base.customer_name,
    contact_phone: payload.contact_phone ?? base.contact_phone ?? null,
    channel: payload.channel ?? base.channel ?? null,
    category: payload.category ?? base.category ?? null,
    priority: payload.priority ?? base.priority ?? null,
    status: payload.status ?? base.status ?? null,
    related_order_id: payload.related_order_id ?? base.related_order_id ?? null,
    satisfaction_score:
      payload.satisfaction_score !== undefined
        ? payload.satisfaction_score
        : base.satisfaction_score ?? null,
    handler: payload.handler ?? base.handler ?? null,
    description: payload.description ?? base.description ?? null,
    updated_at: now
  };
  if (index >= 0) {
    mockServiceTickets[index] = updated;
  } else {
    mockServiceTickets.unshift(updated);
  }
  return updated;
};

export const deleteMockCustomerServiceTicket = (
  ticketId: string
): CustomerServiceTicket => {
  const index = mockServiceTickets.findIndex((ticket) => ticket.ticket_id === ticketId);
  if (index >= 0) {
    const [removed] = mockServiceTickets.splice(index, 1);
    return removed;
  }
  const now = dayjs().toISOString();
  return {
    ticket_id: ticketId,
    customer_id: `CUST-${ticketId}`,
    customer_name: '未知客户',
    contact_phone: null,
    channel: null,
    category: null,
    priority: null,
    status: 'CLOSED',
    related_order_id: null,
    satisfaction_score: null,
    handler: null,
    description: null,
    created_at: now,
    updated_at: now
  };
};
