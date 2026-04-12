import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';

export type NumberRuleStatus = 'ACTIVE' | 'DISABLED';
export type WorkflowStatus = 'DRAFT' | 'ACTIVE' | 'ARCHIVED';
export type WorkflowPublishStatus = 'DRAFT' | 'PUBLISHED' | 'ARCHIVED' | 'FAILED';

export type NumberRule = {
  rule_id: string;
  doc_type: string;
  org_scope: string;
  prefix: string;
  date_format: string;
  sequence_length: number;
  next_number: number;
  status: NumberRuleStatus;
  updated_at: string;
};

export type WorkflowGovernance = {
  engine: string;
  process_key: string;
  definition_id?: string | null;
  decision_key?: string | null;
  publish_status: WorkflowPublishStatus | string;
  approval_mode: string;
  effective_at?: string | null;
  last_deploy_at?: string | null;
  last_deploy_result?: string | null;
};

export type WorkflowDefinition = {
  workflow_id: string;
  doc_type: string;
  name: string;
  version: string;
  status: WorkflowStatus;
  steps: string[];
  governance?: WorkflowGovernance;
  updated_at: string;
};

export type SystemWorkflowCatalog = {
  rules: NumberRule[];
  workflows: WorkflowDefinition[];
};

const cloneCatalog = (catalog: SystemWorkflowCatalog): SystemWorkflowCatalog =>
  JSON.parse(JSON.stringify(catalog)) as SystemWorkflowCatalog;

const buildMockCatalog = (): SystemWorkflowCatalog => ({
  rules: [
    {
      rule_id: 'RULE-PO-HQ',
      doc_type: '采购订单',
      org_scope: '总部',
      prefix: 'PO',
      date_format: 'YYYYMM',
      sequence_length: 4,
      next_number: 128,
      status: 'ACTIVE',
      updated_at: '2026-04-06T00:00:00.000Z'
    },
    {
      rule_id: 'RULE-PO-EAST',
      doc_type: '采购订单',
      org_scope: '华东事业部',
      prefix: 'PO-E',
      date_format: 'YYYYMM',
      sequence_length: 4,
      next_number: 56,
      status: 'ACTIVE',
      updated_at: '2026-04-07T00:00:00.000Z'
    },
    {
      rule_id: 'RULE-RTN',
      doc_type: '售后单',
      org_scope: '总部',
      prefix: 'AF',
      date_format: 'YYYYMMDD',
      sequence_length: 3,
      next_number: 9,
      status: 'DISABLED',
      updated_at: '2026-04-02T00:00:00.000Z'
    }
  ],
  workflows: [
    {
      workflow_id: 'WF-PO-1',
      doc_type: '采购订单',
      name: '采购订单审批',
      version: 'v2.1',
      status: 'ACTIVE',
      steps: ['采购经理', '财务审核', '采购总监'],
      governance: {
        engine: 'CAMUNDA8',
        process_key: 'purchase_order_approval',
        definition_id: 'purchase_order_approval:21:default',
        decision_key: 'purchase_order_gate',
        publish_status: 'PUBLISHED',
        approval_mode: 'HUMAN_TASK',
        effective_at: '2026-04-05T00:00:00.000Z',
        last_deploy_at: '2026-04-05T00:00:00.000Z',
        last_deploy_result: 'SUCCESS'
      },
      updated_at: '2026-04-05T00:00:00.000Z'
    },
    {
      workflow_id: 'WF-PO-2',
      doc_type: '采购订单',
      name: '采购订单审批',
      version: 'v2.0',
      status: 'ARCHIVED',
      steps: ['采购经理', '财务审核'],
      governance: {
        engine: 'CAMUNDA8',
        process_key: 'purchase_order_approval',
        definition_id: 'purchase_order_approval:20:archived',
        decision_key: 'purchase_order_gate',
        publish_status: 'ARCHIVED',
        approval_mode: 'HUMAN_TASK',
        effective_at: '2026-03-19T00:00:00.000Z',
        last_deploy_at: '2026-03-19T00:00:00.000Z',
        last_deploy_result: 'SUCCESS'
      },
      updated_at: '2026-03-19T00:00:00.000Z'
    },
    {
      workflow_id: 'WF-TR-1',
      doc_type: '调拨单',
      name: '调拨单审批',
      version: 'v1.3',
      status: 'DRAFT',
      steps: ['仓储主管', '物流经理'],
      governance: {
        engine: 'CAMUNDA8',
        process_key: 'transfer_order_approval',
        definition_id: null,
        decision_key: 'transfer_order_gate',
        publish_status: 'DRAFT',
        approval_mode: 'HUMAN_TASK',
        effective_at: null,
        last_deploy_at: null,
        last_deploy_result: 'PENDING'
      },
      updated_at: '2026-04-07T00:00:00.000Z'
    }
  ]
});

let mockCatalog: SystemWorkflowCatalog | null = null;

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

export const getSystemWorkflowCatalog = async (
  tenantId: string
): Promise<SystemWorkflowCatalog> => {
  if (useMocks) {
    if (!mockCatalog) {
      mockCatalog = buildMockCatalog();
    }
    return cloneCatalog(mockCatalog);
  }
  return requestJson<SystemWorkflowCatalog>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/workflows`,
    { headers: buildAuthHeaders() },
    '获取单据编号与流程配置失败'
  );
};

export const updateSystemWorkflowCatalog = async (
  tenantId: string,
  payload: SystemWorkflowCatalog
): Promise<SystemWorkflowCatalog> => {
  if (useMocks) {
    mockCatalog = cloneCatalog(payload);
    return cloneCatalog(mockCatalog);
  }
  return requestJson<SystemWorkflowCatalog>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/workflows`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '保存单据编号与流程配置失败'
  );
};
