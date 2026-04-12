import dayjs from 'dayjs';

import type {
  LogisticsInterface,
  LogisticsInterfaceApproval,
  LogisticsInterfaceApprovalCreatePayload,
  LogisticsInterfaceCreatePayload,
  LogisticsInterfaceListQuery,
  LogisticsInterfaceListResponse,
  LogisticsInterfaceUpdatePayload
} from '../logisticsInterfaces';
import { applyPagination } from './data';

const mockInterfaces: LogisticsInterface[] = [
  {
    interface_id: 'IF-SF',
    tenant_id: 'TENANT-DEFAULT',
    carrier_code: 'SF',
    carrier_name: '顺丰速运',
    api_base_url: 'https://api.sf.com/oms',
    template_name: '顺丰电子面单模板A',
    status: 'HEALTHY',
    last_checked_at: dayjs().subtract(2, 'hour').toISOString(),
    retry_policy: '失败重试 3 次/10 分钟',
    alert_channel: '钉钉-物流群',
    approval_status: 'APPROVED',
    approved_by: '系统管理员',
    approved_at: dayjs().subtract(2, 'day').toISOString(),
    notes: '高峰期优先',
    created_at: dayjs().subtract(10, 'day').toISOString(),
    updated_at: dayjs().subtract(2, 'day').toISOString()
  },
  {
    interface_id: 'IF-JD',
    tenant_id: 'TENANT-DEFAULT',
    carrier_code: 'JD',
    carrier_name: '京东物流',
    api_base_url: 'https://open.jd.com/logistics',
    template_name: '京东物流标准模板',
    status: 'WARNING',
    last_checked_at: dayjs().subtract(5, 'hour').toISOString(),
    retry_policy: '失败重试 5 次/15 分钟',
    alert_channel: '企业微信-履约预警',
    approval_status: 'PENDING',
    notes: '接口升级待确认',
    created_at: dayjs().subtract(8, 'day').toISOString(),
    updated_at: dayjs().subtract(1, 'day').toISOString()
  }
];

const mockApprovals: Record<string, LogisticsInterfaceApproval[]> = {
  'IF-SF': [
    {
      approval_id: 'APR-001',
      tenant_id: 'TENANT-DEFAULT',
      interface_id: 'IF-SF',
      action: '创建接口配置',
      operator: '系统管理员',
      comment: '首次接入顺丰接口',
      created_at: dayjs().subtract(8, 'day').toISOString()
    },
    {
      approval_id: 'APR-002',
      tenant_id: 'TENANT-DEFAULT',
      interface_id: 'IF-SF',
      action: '审批通过',
      operator: '物流负责人',
      comment: '接口联调完成',
      created_at: dayjs().subtract(7, 'day').toISOString()
    }
  ],
  'IF-JD': [
    {
      approval_id: 'APR-003',
      tenant_id: 'TENANT-DEFAULT',
      interface_id: 'IF-JD',
      action: '变更提交',
      operator: '接口管理员',
      comment: '切换新密钥',
      created_at: dayjs().subtract(1, 'day').toISOString()
    }
  ]
};

export const listMockLogisticsInterfaces = (
  query: LogisticsInterfaceListQuery
): LogisticsInterfaceListResponse => {
  const keyword = query.keyword?.trim().toLowerCase();
  const filtered = mockInterfaces.filter((item) => {
    if (keyword) {
      const haystack = `${item.carrier_code} ${item.carrier_name}`.toLowerCase();
      if (!haystack.includes(keyword)) {
        return false;
      }
    }
    if (query.status && item.status !== query.status) {
      return false;
    }
    if (query.approval_status && item.approval_status !== query.approval_status) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length
  };
};

export const createMockLogisticsInterface = (
  payload: LogisticsInterfaceCreatePayload
): LogisticsInterface => {
  const now = dayjs().toISOString();
  const interfaceId = `IF-${payload.carrier_code.toUpperCase()}-${Date.now()}`;
  const record: LogisticsInterface = {
    interface_id: interfaceId,
    tenant_id: 'TENANT-DEFAULT',
    carrier_code: payload.carrier_code,
    carrier_name: payload.carrier_name,
    api_base_url: payload.api_base_url,
    template_name: payload.template_name,
    status: payload.status ?? 'HEALTHY',
    last_checked_at: payload.last_checked_at ?? now,
    retry_policy: payload.retry_policy,
    alert_channel: payload.alert_channel,
    approval_status: payload.approval_status ?? 'PENDING',
    approved_by: payload.approved_by ?? null,
    approved_at: payload.approved_at ?? null,
    notes: payload.notes ?? null,
    created_at: now,
    updated_at: now
  };
  mockInterfaces.unshift(record);
  return record;
};

export const updateMockLogisticsInterface = (
  interfaceId: string,
  payload: LogisticsInterfaceUpdatePayload
): LogisticsInterface => {
  const target = mockInterfaces.find((item) => item.interface_id === interfaceId);
  const now = dayjs().toISOString();
  if (!target) {
    const fallback: LogisticsInterface = {
      interface_id: interfaceId,
      tenant_id: 'TENANT-DEFAULT',
      carrier_code: payload.carrier_code ?? 'UNKNOWN',
      carrier_name: payload.carrier_name ?? '未知承运商',
      api_base_url: payload.api_base_url ?? '-',
      template_name: payload.template_name ?? '-',
      status: payload.status ?? 'HEALTHY',
      last_checked_at: payload.last_checked_at ?? now,
      retry_policy: payload.retry_policy ?? '-',
      alert_channel: payload.alert_channel ?? '-',
      approval_status: payload.approval_status ?? 'PENDING',
      approved_by: payload.approved_by ?? null,
      approved_at: payload.approved_at ?? null,
      notes: payload.notes ?? null,
      created_at: now,
      updated_at: now
    };
    mockInterfaces.unshift(fallback);
    return fallback;
  }

  Object.assign(target, {
    carrier_code: payload.carrier_code ?? target.carrier_code,
    carrier_name: payload.carrier_name ?? target.carrier_name,
    api_base_url: payload.api_base_url ?? target.api_base_url,
    template_name: payload.template_name ?? target.template_name,
    status: payload.status ?? target.status,
    last_checked_at: payload.last_checked_at ?? target.last_checked_at,
    retry_policy: payload.retry_policy ?? target.retry_policy,
    alert_channel: payload.alert_channel ?? target.alert_channel,
    approval_status: payload.approval_status ?? target.approval_status,
    approved_by: payload.approved_by ?? target.approved_by,
    approved_at: payload.approved_at ?? target.approved_at,
    notes: payload.notes ?? target.notes,
    updated_at: now
  });
  return target;
};

export const listMockLogisticsInterfaceApprovals = (
  interfaceId: string
): LogisticsInterfaceApproval[] => mockApprovals[interfaceId]?.map((item) => ({ ...item })) ?? [];

export const createMockLogisticsInterfaceApproval = (
  interfaceId: string,
  payload: LogisticsInterfaceApprovalCreatePayload
): LogisticsInterfaceApproval => {
  const now = dayjs().toISOString();
  const record: LogisticsInterfaceApproval = {
    approval_id: `APR-${Date.now()}`,
    tenant_id: 'TENANT-DEFAULT',
    interface_id: interfaceId,
    action: payload.action,
    operator: payload.operator,
    comment: payload.comment ?? null,
    created_at: now
  };
  mockApprovals[interfaceId] = [record, ...(mockApprovals[interfaceId] ?? [])];
  return record;
};
