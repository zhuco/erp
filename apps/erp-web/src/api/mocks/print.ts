import dayjs from 'dayjs';

import type {
  CreatePrintJobPayload,
  PrintJobDetail,
  PrintJobDispatchStatus,
  PrintJobListQuery,
  PrintJobStatus,
  PrintTemplateListQuery,
  UpdatePrintJobPayload
} from './data';
import type {
  PrintTemplate,
  PrintTemplateCreatePayload,
  PrintTemplateUpdatePayload
} from '../printTemplates';
import {
  applyPagination,
  appendPrintJobEvent,
  buildPrintJobSummary,
  mockPrintJobs,
  mockPrintTemplates
} from './data';

export const getMockPrintTemplateList = (query: PrintTemplateListQuery) => {
  const filtered = mockPrintTemplates.filter((template) => {
    if (query.platform && template.platform !== query.platform) {
      return false;
    }
    if (query.shop_id && template.shop_id !== query.shop_id) {
      return false;
    }
    if (query.template_type && template.template_type !== query.template_type) {
      return false;
    }
    if (query.enabled != null && template.enabled !== query.enabled) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? 20;
  const items = applyPagination(filtered, offset, limit).map((item) => ({
    content_format: 'HTML',
    ...item
  }));
  return {
    items,
    total: filtered.length
  };
};

export const getMockPrintJobList = (query: PrintJobListQuery) => {
  const filtered = mockPrintJobs.filter((job) => {
    if (query.status && job.status !== query.status) {
      return false;
    }
    if (query.job_type && job.job_type !== query.job_type) {
      return false;
    }
    if (query.ref_type && job.ref_type !== query.ref_type) {
      return false;
    }
    if (query.ref_id && job.ref_id !== query.ref_id) {
      return false;
    }
    return true;
  });
  const offset = query.offset ?? 0;
  const limit = query.limit ?? 20;
  return {
    items: applyPagination(filtered, offset, limit).map(buildPrintJobSummary),
    total: filtered.length
  };
};

const resolveDispatchStatus = (status: PrintJobStatus): PrintJobDispatchStatus => {
  switch (status) {
    case 'RUNNING':
      return 'DISPATCHED';
    case 'DONE':
      return 'PRINTED';
    case 'FAILED':
      return 'FAILED';
    case 'PENDING':
    default:
      return 'QUEUED';
  }
};

export const getMockPrintJobDetail = (printJobId: string): PrintJobDetail => {
  return (
    mockPrintJobs.find((job) => job.job_id === printJobId) ?? {
      job_id: printJobId,
      job_type: 'SHIPPING_LABEL',
      ref_type: 'ORDER',
      ref_id: printJobId,
      status: 'FAILED',
      dispatch_status: resolveDispatchStatus('FAILED'),
      output_url: null,
      device_name: 'Mock Device',
      device_fingerprint: 'mock-device',
      attempts: 0,
      last_error: '打印设备离线',
      created_at: dayjs().toISOString(),
      updated_at: dayjs().toISOString(),
      events: [],
      payload: null
    }
  );
};

export const createMockPrintJob = (payload: CreatePrintJobPayload): PrintJobDetail => {
  const now = dayjs().toISOString();
  const job: PrintJobDetail = {
    job_id: `PRINT-${Date.now()}`,
    job_type: payload.job_type,
    ref_type: payload.ref_type,
    ref_id: payload.ref_id,
    status: 'PENDING',
    dispatch_status: resolveDispatchStatus('PENDING'),
    output_url: null,
    attempts: 0,
    last_error: '',
    device_name: 'Mock Device',
    device_fingerprint: 'mock-device',
    created_at: now,
    updated_at: now,
    events: [],
    payload: payload.payload ?? null
  };
  appendPrintJobEvent(job, 'PENDING');
  mockPrintJobs.unshift(job);
  return job;
};

export const updateMockPrintJobStatus = (
  printJobId: string,
  payload: UpdatePrintJobPayload
): PrintJobDetail => {
  const job = mockPrintJobs.find((item) => item.job_id === printJobId);
  const target = job ?? getMockPrintJobDetail(printJobId);
  if (payload.status) {
    target.status = payload.status;
    target.dispatch_status = resolveDispatchStatus(payload.status);
  }
  if (payload.payload) {
    target.payload = payload.payload;
  }
  if (payload.output_url !== undefined) {
    target.output_url = payload.output_url;
  }
  target.last_error = payload.last_error ?? target.last_error ?? '';
  target.updated_at = dayjs().toISOString();
  if (payload.status) {
    appendPrintJobEvent(
      target,
      payload.status,
      payload.status === 'FAILED' ? payload.last_error ?? '打印任务失败' : null
    );
  }
  if (!job) {
    mockPrintJobs.unshift(target);
  }
  return target;
};

export const dispatchMockPrintJob = (printJobId: string): PrintJobDetail => {
  const job = mockPrintJobs.find((item) => item.job_id === printJobId);
  const target = job ?? getMockPrintJobDetail(printJobId);
  target.status = 'PENDING';
  target.dispatch_status = 'QUEUED';
  target.last_error = '';
  target.updated_at = dayjs().toISOString();
  appendPrintJobEvent(target, 'PENDING');
  if (!job) {
    mockPrintJobs.unshift(target);
  }
  return target;
};

export const createMockPrintTemplate = (
  payload: PrintTemplateCreatePayload
): PrintTemplate => {
  const timestamp = dayjs().toISOString();
  const template: PrintTemplate = {
    template_id: `TPL-${Date.now()}`,
    platform: payload.platform,
    shop_id: payload.shop_id,
    template_type: payload.template_type,
    version: payload.version ?? 1,
    content_format: payload.content_format ?? 'HTML',
    content: payload.content ?? '',
    enabled: payload.enabled ?? true,
    created_at: timestamp,
    updated_at: timestamp
  };
  mockPrintTemplates.unshift(template);
  return template;
};

export const updateMockPrintTemplate = (
  templateId: string,
  payload: PrintTemplateUpdatePayload
): PrintTemplate => {
  const index = mockPrintTemplates.findIndex((item) => item.template_id === templateId);
  const base: PrintTemplate =
    index >= 0
      ? {
          content_format: 'HTML',
          ...mockPrintTemplates[index]
        }
      : {
          template_id: templateId,
          platform: 'unknown',
          shop_id: 'unknown',
          template_type: 'SHIPPING_LABEL',
          version: 1,
          content_format: 'HTML',
          content: '',
          enabled: true,
          created_at: dayjs().toISOString(),
          updated_at: dayjs().toISOString()
        };
  const updated: PrintTemplate = {
    ...base,
    content_format: payload.content_format ?? base.content_format ?? 'HTML',
    content: payload.content ?? base.content ?? '',
    enabled: payload.enabled ?? base.enabled,
    updated_at: dayjs().toISOString()
  };
  if (index >= 0) {
    mockPrintTemplates[index] = updated as PrintTemplate;
  } else {
    mockPrintTemplates.unshift(updated as PrintTemplate);
  }
  return updated;
};

export const deleteMockPrintTemplate = (templateId: string): PrintTemplate => {
  const index = mockPrintTemplates.findIndex((item) => item.template_id === templateId);
  if (index >= 0) {
    const [removed] = mockPrintTemplates.splice(index, 1);
    return { content_format: 'HTML', ...removed } as PrintTemplate;
  }
  return {
    template_id: templateId,
    platform: 'unknown',
    shop_id: 'unknown',
    template_type: 'SHIPPING_LABEL',
    version: 1,
    content_format: 'HTML',
    content: '',
    enabled: false,
    created_at: dayjs().toISOString(),
    updated_at: dayjs().toISOString()
  };
};
