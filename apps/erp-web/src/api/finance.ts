import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders, fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { listMockProfitSnapshots } from '@/api/mocks';

export type StatementImportType = 'PLATFORM' | 'SHIPPING' | 'PAYMENT';

export type PlatformStatementLineInput = {
  platform_order_id: string;
  gross_cent?: number;
  fee_cent?: number;
  refund_cent?: number;
  net_cent?: number;
  occurred_at?: string | null;
  raw_json?: Record<string, unknown> | null;
};

export type ShippingStatementLineInput = {
  tracking_no: string;
  fee_cent?: number;
  weight?: number | null;
  occurred_at?: string | null;
  raw_json?: Record<string, unknown> | null;
};

export type PaymentStatementLineInput = {
  platform_order_id: string;
  paid_cent?: number;
  fee_cent?: number;
  net_cent?: number;
  occurred_at?: string | null;
  raw_json?: Record<string, unknown> | null;
};

export type StatementImportRequest = {
  statement_type: StatementImportType;
  platform?: string;
  shop_id?: string;
  channel?: string;
  period_start?: string;
  period_end?: string;
  lines?: PlatformStatementLineInput[];
  shipping_lines?: ShippingStatementLineInput[];
  payment_lines?: PaymentStatementLineInput[];
};

export type StatementImportResponse = {
  statement_type: StatementImportType;
  statement_id?: string | null;
  inserted_lines: number;
};

export type ReconcileRunRequest = {
  job_type: 'PLATFORM' | 'SHIPPING';
  statement_id?: string;
  occurred_from?: string;
  occurred_to?: string;
};

export type ReconcileRunResponse = {
  job_id: string;
  status: string;
  diff_count: number;
};

export type ReconcileJob = {
  job_id: string;
  job_type: string;
  status: string;
  started_at?: string | null;
  finished_at?: string | null;
  last_error?: string | null;
  created_at: string;
};

export type ReconcileJobItem = {
  job_id: string;
  ref_type: string;
  ref_id: string;
  expected_cent: number;
  actual_cent: number;
  diff_cent: number;
  diff_reason?: string | null;
  raw_json?: Record<string, unknown>;
  created_at: string;
};

export type ReconcileJobListResponse = {
  items: ReconcileJob[];
  total: number;
};

export type ReconcileJobItemListResponse = {
  items: ReconcileJobItem[];
  total: number;
};

export type PaymentReconcileRunRequest = {
  statement_id: string;
};

export type PaymentReconcileRunResponse = {
  job_id: string;
  status: string;
  diff_count: number;
};

export type PaymentReconcileJob = {
  job_id: string;
  statement_id?: string | null;
  status: string;
  diff_count: number;
  started_at?: string | null;
  finished_at?: string | null;
  last_error?: string | null;
  created_at: string;
};

export type PaymentReconcileJobListResponse = {
  items: PaymentReconcileJob[];
  total: number;
};

export type PaymentReconcileItem = {
  job_id: string;
  platform_order_id: string;
  order_id?: string | null;
  expected_cent: number;
  actual_cent: number;
  diff_cent: number;
  diff_reason?: string | null;
  raw_json?: Record<string, unknown>;
  created_at: string;
};

export type PaymentReconcileItemListResponse = {
  items: PaymentReconcileItem[];
  total: number;
};

export type InvoiceApplyRequest = {
  order_id: string;
  invoice_title: string;
  tax_no?: string;
};

export type Invoice = {
  invoice_id: string;
  order_id: string;
  invoice_title: string;
  tax_no?: string | null;
  status: string;
  issued_at?: string | null;
  result_json?: Record<string, unknown>;
  created_at: string;
  updated_at: string;
};

export type InvoiceListResponse = {
  items: Invoice[];
  total: number;
};

export type ApDoc = {
  ap_id: string;
  supplier_id: string;
  supplier_name?: string | null;
  ref_type: string;
  ref_id: string;
  amount_cent: number;
  status: string;
  settlement_status: string;
  paid_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type ApDocListResponse = {
  items: ApDoc[];
  total: number;
};

export type ProfitSnapshot = {
  order_id: string;
  shop_id: string;
  platform_order_id: string;
  order_created_at: string;
  revenue_cent: number;
  cost_cent: number;
  platform_fee_cent: number;
  shipping_fee_cent: number;
  gross_profit_cent: number;
  computed_at: string;
};

export type ProfitSnapshotListResponse = {
  items: ProfitSnapshot[];
  total: number;
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

export const importStatementCsv = async (
  tenantId: string,
  params: {
    statement_type: StatementImportType;
    platform?: string;
    shop_id?: string;
    channel?: string;
    period_start?: string;
    period_end?: string;
  },
  csvText: string
): Promise<StatementImportResponse> => {
  const query = new URLSearchParams();
  query.set('statement_type', params.statement_type);
  if (params.platform) {
    query.set('platform', params.platform);
  }
  if (params.shop_id) {
    query.set('shop_id', params.shop_id);
  }
  if (params.channel) {
    query.set('channel', params.channel);
  }
  if (params.period_start) {
    query.set('period_start', params.period_start);
  }
  if (params.period_end) {
    query.set('period_end', params.period_end);
  }
  return requestJson<StatementImportResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/statements/import?${query.toString()}`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'text/csv'
      },
      body: csvText
    },
    '账单导入失败'
  );
};

export const importStatementJson = async (
  tenantId: string,
  payload: StatementImportRequest
): Promise<StatementImportResponse> =>
  requestJson<StatementImportResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/statements/import`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '账单导入失败'
  );

export const runReconcile = async (
  tenantId: string,
  payload: ReconcileRunRequest
): Promise<ReconcileRunResponse> =>
  requestJson<ReconcileRunResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/reconcile/run`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '对账执行失败'
  );

export const listReconcileJobs = async (
  tenantId: string,
  query: { job_type?: string; status?: string; offset?: number; limit?: number }
): Promise<ReconcileJobListResponse> => {
  const params = new URLSearchParams();
  if (query.job_type) {
    params.set('job_type', query.job_type);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<ReconcileJobListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/reconcile/jobs?${params.toString()}`,
    {
      headers: buildAuthHeaders()
    },
    '获取对账任务失败'
  );
};

export const listReconcileItems = async (
  tenantId: string,
  query: {
    job_id: string;
    ref_type?: string;
    diff_only?: boolean;
    offset?: number;
    limit?: number;
  }
): Promise<ReconcileJobItemListResponse> => {
  const params = new URLSearchParams();
  params.set('job_id', query.job_id);
  if (query.ref_type) {
    params.set('ref_type', query.ref_type);
  }
  if (query.diff_only != null) {
    params.set('diff_only', String(query.diff_only));
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<ReconcileJobItemListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/reconcile/items?${params.toString()}`,
    {
      headers: buildAuthHeaders()
    },
    '获取差异明细失败'
  );
};

export const runPaymentReconcile = async (
  tenantId: string,
  payload: PaymentReconcileRunRequest
): Promise<PaymentReconcileRunResponse> =>
  requestJson<PaymentReconcileRunResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/payment-reconcile`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '鏀粯瀵硅处鎵ц澶辫触'
  );

export const listPaymentReconcileJobs = async (
  tenantId: string,
  query: { status?: string; statement_id?: string; offset?: number; limit?: number }
): Promise<PaymentReconcileJobListResponse> => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.statement_id) {
    params.set('statement_id', query.statement_id);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<PaymentReconcileJobListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/payment-reconcile/jobs?${params.toString()}`,
    { headers: buildAuthHeaders() },
    '鑾峰彇鏀粯瀵硅处浠诲姟澶辫触'
  );
};

export const listPaymentReconcileItems = async (
  tenantId: string,
  query: { job_id: string; diff_only?: boolean; offset?: number; limit?: number }
): Promise<PaymentReconcileItemListResponse> => {
  const params = new URLSearchParams();
  params.set('job_id', query.job_id);
  if (query.diff_only != null) {
    params.set('diff_only', String(query.diff_only));
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<PaymentReconcileItemListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/payment-reconcile/items?${params.toString()}`,
    { headers: buildAuthHeaders() },
    '鑾峰彇鏀粯瀵硅处宸紓澶辫触'
  );
};

export const applyInvoice = async (
  tenantId: string,
  payload: InvoiceApplyRequest
): Promise<Invoice> =>
  requestJson<Invoice>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/invoices/apply`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '开票申请失败'
  );

export const listInvoices = async (
  tenantId: string,
  query: { status?: string; order_id?: string; offset?: number; limit?: number }
): Promise<InvoiceListResponse> => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.order_id) {
    params.set('order_id', query.order_id);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<InvoiceListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/invoices?${params.toString()}`,
    {
      headers: buildAuthHeaders()
    },
    '获取开票列表失败'
  );
};

export const listApDocs = async (
  tenantId: string,
  query: { status?: string; supplier_id?: string; offset?: number; limit?: number }
): Promise<ApDocListResponse> => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.supplier_id) {
    params.set('supplier_id', query.supplier_id);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<ApDocListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-docs?${params.toString()}`,
    {
      headers: buildAuthHeaders()
    },
    '鑾峰彇搴旀敮鍒楄〃澶辫触'
  );
};

export const getApDoc = async (tenantId: string, apId: string): Promise<ApDoc> =>
  requestJson<ApDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-docs/${apId}`,
    {
      headers: buildAuthHeaders()
    },
    '鑾峰彇搴旀敮鏄庣粏澶辫触'
  );

export const markApDocPaid = async (tenantId: string, apId: string): Promise<ApDoc> =>
  requestJson<ApDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-docs/${apId}/mark-paid`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '鏍囥��搴旀敮宸茬粨澶辫触'
  );

export const closeApDoc = async (tenantId: string, apId: string): Promise<ApDoc> =>
  requestJson<ApDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-docs/${apId}/close`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '鍏抽棴搴斾粯鍗曟嵁澶辫触'
  );

export const listProfitSnapshots = async (
  tenantId: string,
  query: {
    shop_id?: string;
    sku_id?: string;
    created_from?: string;
    created_to?: string;
    offset?: number;
    limit?: number;
  }
): Promise<ProfitSnapshotListResponse> => {
  if (useMocks) {
    return listMockProfitSnapshots(query);
  }
  const params = new URLSearchParams();
  if (query.shop_id) {
    params.set('shop_id', query.shop_id);
  }
  if (query.sku_id) {
    params.set('sku_id', query.sku_id);
  }
  if (query.created_from) {
    params.set('created_from', query.created_from);
  }
  if (query.created_to) {
    params.set('created_to', query.created_to);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<ProfitSnapshotListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/profit?${params.toString()}`,
    {
      headers: buildAuthHeaders()
    },
    '获取利润报表失败'
  );
};

export type ArDoc = {
  ar_id: string;
  customer_id?: string | null;
  customer_name?: string | null;
  ref_type: string;
  ref_id: string;
  amount_cent: number;
  currency: string;
  status: string;
  due_date?: string | null;
  written_off_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type ArDocListResponse = {
  items: ArDoc[];
  total: number;
};

export type ArDocCreatePayload = {
  customer_id?: string;
  ref_type: string;
  ref_id: string;
  amount_cent: number;
  currency?: string;
  status?: string;
  due_date?: string;
};

export type ArDocUpdatePayload = Partial<ArDocCreatePayload>;

export const listArDocs = async (
  tenantId: string,
  query: {
    keyword?: string;
    status?: string;
    customer_id?: string;
    offset?: number;
    limit?: number;
  }
): Promise<ArDocListResponse> => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.customer_id) {
    params.set('customer_id', query.customer_id);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<ArDocListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ar-docs?${params.toString()}`,
    {
      headers: buildAuthHeaders()
    },
    '获取应收列表失败'
  );
};

export const getArDoc = async (tenantId: string, arId: string): Promise<ArDoc> =>
  requestJson<ArDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ar-docs/${arId}`,
    { headers: buildAuthHeaders() },
    '获取应收详情失败'
  );

export const createArDoc = async (
  tenantId: string,
  payload: ArDocCreatePayload
): Promise<ArDoc> =>
  requestJson<ArDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ar-docs`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '创建应收失败'
  );

export const updateArDoc = async (
  tenantId: string,
  arId: string,
  payload: ArDocUpdatePayload
): Promise<ArDoc> =>
  requestJson<ArDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ar-docs/${arId}`,
    {
      method: 'PUT',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '更新应收失败'
  );

export const deleteArDoc = async (tenantId: string, arId: string): Promise<ArDoc> =>
  requestJson<ArDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ar-docs/${arId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除应收失败'
  );

export const writeOffArDoc = async (tenantId: string, arId: string): Promise<ArDoc> =>
  requestJson<ArDoc>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ar-docs/${arId}/write-off`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '应收核销失败'
  );

export type ApPaymentPlan = {
  plan_id: string;
  ap_id: string;
  supplier_id?: string | null;
  supplier_name?: string | null;
  planned_date?: string | null;
  amount_cent: number;
  status: string;
  paid_at?: string | null;
  notes?: string | null;
  created_at: string;
  updated_at: string;
};

export type ApPaymentPlanListResponse = {
  items: ApPaymentPlan[];
  total: number;
};

export type ApPaymentPlanCreatePayload = {
  ap_id: string;
  supplier_id?: string;
  planned_date?: string;
  amount_cent: number;
  status?: string;
  notes?: string;
};

export type ApPaymentPlanUpdatePayload = Partial<Omit<ApPaymentPlanCreatePayload, 'ap_id'>>;

export const listApPaymentPlans = async (
  tenantId: string,
  query: {
    status?: string;
    supplier_id?: string;
    ap_id?: string;
    offset?: number;
    limit?: number;
  }
): Promise<ApPaymentPlanListResponse> => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.supplier_id) {
    params.set('supplier_id', query.supplier_id);
  }
  if (query.ap_id) {
    params.set('ap_id', query.ap_id);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<ApPaymentPlanListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-plans?${params.toString()}`,
    { headers: buildAuthHeaders() },
    '获取付款计划失败'
  );
};

export const createApPaymentPlan = async (
  tenantId: string,
  payload: ApPaymentPlanCreatePayload
): Promise<ApPaymentPlan> =>
  requestJson<ApPaymentPlan>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-plans`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '创建付款计划失败'
  );

export const updateApPaymentPlan = async (
  tenantId: string,
  planId: string,
  payload: ApPaymentPlanUpdatePayload
): Promise<ApPaymentPlan> =>
  requestJson<ApPaymentPlan>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-plans/${planId}`,
    {
      method: 'PUT',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '更新付款计划失败'
  );

export const deleteApPaymentPlan = async (
  tenantId: string,
  planId: string
): Promise<ApPaymentPlan> =>
  requestJson<ApPaymentPlan>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-plans/${planId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除付款计划失败'
  );

export const markApPaymentPlanPaid = async (
  tenantId: string,
  planId: string
): Promise<ApPaymentPlan> =>
  requestJson<ApPaymentPlan>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/ap-plans/${planId}/mark-paid`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '付款计划标记失败'
  );

export type FinanceExpense = {
  expense_id: string;
  name: string;
  category: string;
  amount_cent: number;
  currency: string;
  allocation_rule?: Record<string, unknown>;
  period_start?: string | null;
  period_end?: string | null;
  status: string;
  created_at: string;
  updated_at: string;
};

export type FinanceExpenseListResponse = {
  items: FinanceExpense[];
  total: number;
};

export type FinanceExpenseCreatePayload = {
  name: string;
  category: string;
  amount_cent: number;
  currency?: string;
  allocation_rule?: Record<string, unknown>;
  period_start?: string;
  period_end?: string;
  status?: string;
};

export type FinanceExpenseUpdatePayload = Partial<FinanceExpenseCreatePayload>;

export const listFinanceExpenses = async (
  tenantId: string,
  query: { keyword?: string; status?: string; category?: string; offset?: number; limit?: number }
): Promise<FinanceExpenseListResponse> => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.category) {
    params.set('category', query.category);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<FinanceExpenseListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/expenses?${params.toString()}`,
    { headers: buildAuthHeaders() },
    '获取费用列表失败'
  );
};

export const createFinanceExpense = async (
  tenantId: string,
  payload: FinanceExpenseCreatePayload
): Promise<FinanceExpense> =>
  requestJson<FinanceExpense>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/expenses`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '创建费用失败'
  );

export const updateFinanceExpense = async (
  tenantId: string,
  expenseId: string,
  payload: FinanceExpenseUpdatePayload
): Promise<FinanceExpense> =>
  requestJson<FinanceExpense>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/expenses/${expenseId}`,
    {
      method: 'PUT',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '更新费用失败'
  );

export const deleteFinanceExpense = async (
  tenantId: string,
  expenseId: string
): Promise<FinanceExpense> =>
  requestJson<FinanceExpense>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/expenses/${expenseId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除费用失败'
  );

export type InventoryCostingRun = {
  costing_id: string;
  method: string;
  period_start?: string | null;
  period_end?: string | null;
  status: string;
  variance_cent: number;
  result_json?: Record<string, unknown>;
  created_at: string;
  updated_at: string;
};

export type InventoryCostingRunListResponse = {
  items: InventoryCostingRun[];
  total: number;
};

export type InventoryCostingRunCreatePayload = {
  method: string;
  period_start?: string;
  period_end?: string;
  status?: string;
  variance_cent?: number;
  result_json?: Record<string, unknown>;
};

export type InventoryCostingRunUpdatePayload = Partial<InventoryCostingRunCreatePayload>;

export const listInventoryCostingRuns = async (
  tenantId: string,
  query: { status?: string; method?: string; offset?: number; limit?: number }
): Promise<InventoryCostingRunListResponse> => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.method) {
    params.set('method', query.method);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<InventoryCostingRunListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/costing-runs?${params.toString()}`,
    { headers: buildAuthHeaders() },
    '获取成本核算失败'
  );
};

export const createInventoryCostingRun = async (
  tenantId: string,
  payload: InventoryCostingRunCreatePayload
): Promise<InventoryCostingRun> =>
  requestJson<InventoryCostingRun>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/costing-runs`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '创建成本核算失败'
  );

export const updateInventoryCostingRun = async (
  tenantId: string,
  costingId: string,
  payload: InventoryCostingRunUpdatePayload
): Promise<InventoryCostingRun> =>
  requestJson<InventoryCostingRun>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/costing-runs/${costingId}`,
    {
      method: 'PUT',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '更新成本核算失败'
  );

export type ConsignmentSettlement = {
  settlement_id: string;
  partner_name: string;
  agreement_no?: string | null;
  period_start?: string | null;
  period_end?: string | null;
  sales_amount_cent: number;
  commission_rate: number;
  commission_cent: number;
  status: string;
  settled_at?: string | null;
  notes?: string | null;
  created_at: string;
  updated_at: string;
};

export type ConsignmentSettlementListResponse = {
  items: ConsignmentSettlement[];
  total: number;
};

export type ConsignmentSettlementCreatePayload = {
  partner_name: string;
  agreement_no?: string;
  period_start?: string;
  period_end?: string;
  sales_amount_cent: number;
  commission_rate: number;
  commission_cent: number;
  status?: string;
  notes?: string;
};

export type ConsignmentSettlementUpdatePayload = Partial<ConsignmentSettlementCreatePayload>;

export const listConsignmentSettlements = async (
  tenantId: string,
  query: { keyword?: string; status?: string; offset?: number; limit?: number }
): Promise<ConsignmentSettlementListResponse> => {
  const params = new URLSearchParams();
  if (query.keyword) {
    params.set('keyword', query.keyword);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<ConsignmentSettlementListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/consignments?${params.toString()}`,
    { headers: buildAuthHeaders() },
    '获取代销结算失败'
  );
};

export const createConsignmentSettlement = async (
  tenantId: string,
  payload: ConsignmentSettlementCreatePayload
): Promise<ConsignmentSettlement> =>
  requestJson<ConsignmentSettlement>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/consignments`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '创建代销结算失败'
  );

export const updateConsignmentSettlement = async (
  tenantId: string,
  settlementId: string,
  payload: ConsignmentSettlementUpdatePayload
): Promise<ConsignmentSettlement> =>
  requestJson<ConsignmentSettlement>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/consignments/${settlementId}`,
    {
      method: 'PUT',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '更新代销结算失败'
  );

export const deleteConsignmentSettlement = async (
  tenantId: string,
  settlementId: string
): Promise<ConsignmentSettlement> =>
  requestJson<ConsignmentSettlement>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/consignments/${settlementId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除代销结算失败'
  );

export const markConsignmentSettlement = async (
  tenantId: string,
  settlementId: string
): Promise<ConsignmentSettlement> =>
  requestJson<ConsignmentSettlement>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/consignments/${settlementId}/mark-settled`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '代销结算确认失败'
  );

export type TaxRule = {
  rule_id: string;
  tax_code: string;
  tax_rate: number;
  effective_from?: string | null;
  effective_to?: string | null;
  status: string;
  created_at: string;
  updated_at: string;
};

export type TaxRuleListResponse = {
  items: TaxRule[];
  total: number;
};

export type TaxRuleCreatePayload = {
  tax_code: string;
  tax_rate: number;
  effective_from?: string;
  effective_to?: string;
  status?: string;
};

export type TaxRuleUpdatePayload = Partial<TaxRuleCreatePayload>;

export const listTaxRules = async (
  tenantId: string,
  query: { tax_code?: string; status?: string; offset?: number; limit?: number }
): Promise<TaxRuleListResponse> => {
  const params = new URLSearchParams();
  if (query.tax_code) {
    params.set('tax_code', query.tax_code);
  }
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<TaxRuleListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/tax-rules?${params.toString()}`,
    { headers: buildAuthHeaders() },
    '获取税率失败'
  );
};

export const createTaxRule = async (
  tenantId: string,
  payload: TaxRuleCreatePayload
): Promise<TaxRule> =>
  requestJson<TaxRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/tax-rules`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '创建税率失败'
  );

export const updateTaxRule = async (
  tenantId: string,
  ruleId: string,
  payload: TaxRuleUpdatePayload
): Promise<TaxRule> =>
  requestJson<TaxRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/tax-rules/${ruleId}`,
    {
      method: 'PUT',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '更新税率失败'
  );

export const deleteTaxRule = async (tenantId: string, ruleId: string): Promise<TaxRule> =>
  requestJson<TaxRule>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/tax-rules/${ruleId}`,
    {
      method: 'DELETE',
      headers: buildAuthHeaders()
    },
    '删除税率失败'
  );

export type InvoiceAdjustment = {
  adjustment_id: string;
  invoice_id: string;
  adjustment_type: string;
  reason?: string | null;
  status: string;
  applied_at?: string | null;
  created_at: string;
  updated_at: string;
};

export type InvoiceAdjustmentListResponse = {
  items: InvoiceAdjustment[];
  total: number;
};

export type InvoiceAdjustmentCreatePayload = {
  invoice_id: string;
  adjustment_type: string;
  reason?: string;
  status?: string;
};

export const listInvoiceAdjustments = async (
  tenantId: string,
  query: { status?: string; invoice_id?: string; offset?: number; limit?: number }
): Promise<InvoiceAdjustmentListResponse> => {
  const params = new URLSearchParams();
  if (query.status) {
    params.set('status', query.status);
  }
  if (query.invoice_id) {
    params.set('invoice_id', query.invoice_id);
  }
  if (query.offset != null) {
    params.set('offset', String(query.offset));
  }
  if (query.limit != null) {
    params.set('limit', String(query.limit));
  }
  return requestJson<InvoiceAdjustmentListResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/invoice-adjustments?${params.toString()}`,
    { headers: buildAuthHeaders() },
    '获取发票调整失败'
  );
};

export const createInvoiceAdjustment = async (
  tenantId: string,
  payload: InvoiceAdjustmentCreatePayload
): Promise<InvoiceAdjustment> =>
  requestJson<InvoiceAdjustment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/invoice-adjustments`,
    {
      method: 'POST',
      headers: {
        ...buildAuthHeaders(),
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(payload)
    },
    '创建发票调整失败'
  );

export const markInvoiceAdjustmentApplied = async (
  tenantId: string,
  adjustmentId: string
): Promise<InvoiceAdjustment> =>
  requestJson<InvoiceAdjustment>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/fin/invoice-adjustments/${adjustmentId}/mark-applied`,
    {
      method: 'POST',
      headers: buildAuthHeaders()
    },
    '发票调整失败'
  );
