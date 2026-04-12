import { apiBaseUrl } from '@/utils/env';
import type { PrintJobDetail } from '@/api/printJobs';
import {
  createPrintJob,
  getPrintJobDetail,
  retryPrintJob,
  updatePrintJobStatus
} from '@/api/printJobs';
import { listPrintTemplates } from '@/api/printTemplates';
import { printBridge } from './PrintBridge';
import { getStoredPrintTemplate } from './printTemplateSettings';
import type { PrintShippingLabelPayload } from './types';

export type PrintExecutionResult = {
  job: PrintJobDetail;
  outputUrl?: string | null;
};

export type PrintExecutionOptions = {
  deviceFingerprint?: string;
};

type PrintTemplatePreference = {
  template_id: string;
};

const resolveActiveTemplate = async (
  tenantId: string,
  payload: PrintShippingLabelPayload
): Promise<PrintTemplatePreference | null> => {
  const storedTemplate = getStoredPrintTemplate();
  if (storedTemplate) {
    return storedTemplate;
  }
  try {
    const response = await listPrintTemplates(tenantId, {
      platform: payload.platform,
      shop_id: payload.shop_id,
      template_type: 'SHIPPING_LABEL',
      limit: 50,
      offset: 0
    });
    const resolved = response.items.find((item) => item.enabled) ?? response.items[0] ?? null;
    if (!resolved) {
      return null;
    }
    return {
      template_id: resolved.template_id
    };
  } catch {
    return null;
  }
};

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const waitForPrintJob = async (
  tenantId: string,
  printJobId: string,
  maxAttempts = 15,
  intervalMs = 1000
): Promise<PrintJobDetail> => {
  let lastJob: PrintJobDetail | null = null;
  for (let attempt = 0; attempt < maxAttempts; attempt += 1) {
    const job = await getPrintJobDetail(tenantId, printJobId);
    lastJob = job;
    if (job.status === 'DONE' || job.status === 'FAILED') {
      return job;
    }
    await sleep(intervalMs);
  }
  return lastJob ?? (await getPrintJobDetail(tenantId, printJobId));
};

export const openPrintJobOutput = (job: PrintJobDetail): string | null => {
  const outputUrl = job.output_url ? `${apiBaseUrl}${job.output_url}` : null;
  if (outputUrl) {
    window.open(outputUrl, '_blank');
  }
  return outputUrl;
};

const tryPrintWithBridge = async (
  tenantId: string,
  jobId: string,
  payload: PrintShippingLabelPayload
): Promise<PrintJobDetail | null> => {
  const detectResult = await printBridge.detect();
  if (!detectResult.success) {
    return null;
  }
  const bridgeResult = await printBridge.printShippingLabel(payload);
  const nextStatus = bridgeResult.success ? 'DONE' : 'FAILED';
  const lastError = bridgeResult.success ? '' : bridgeResult.error;
  try {
    return await updatePrintJobStatus(tenantId, jobId, {
      status: nextStatus,
      last_error: lastError
    });
  } catch {
    return null;
  }
};

export const createPrintJobAndPrint = async (
  tenantId: string,
  payload: PrintShippingLabelPayload,
  options?: PrintExecutionOptions
): Promise<PrintExecutionResult> => {
  const template = await resolveActiveTemplate(tenantId, payload);
  const job = await createPrintJob(tenantId, {
    job_type: 'SHIPPING_LABEL',
    ref_type: payload.shipment_id ? 'SHIPMENT' : 'ORDER',
    ref_id: payload.shipment_id ?? payload.order_id,
    template_id: template?.template_id,
    device_fingerprint: options?.deviceFingerprint,
    payload
  });

  if (!options?.deviceFingerprint) {
    const bridged = await tryPrintWithBridge(tenantId, job.job_id, payload);
    if (bridged) {
      return { job: bridged, outputUrl: openPrintJobOutput(bridged) };
    }
  }

  const completed = await waitForPrintJob(tenantId, job.job_id);
  return { job: completed, outputUrl: openPrintJobOutput(completed) };
};

export const retryPrintJobAndPrint = async (
  tenantId: string,
  printJobId: string
): Promise<PrintExecutionResult> => {
  const detail = await retryPrintJob(tenantId, printJobId);
  if (detail.job_type === 'SHIPPING_LABEL' && detail.payload) {
    const payload = detail.payload as PrintShippingLabelPayload;
    if (payload.order_id && payload.shop_id && payload.platform) {
      const bridged = await tryPrintWithBridge(tenantId, detail.job_id, payload);
      if (bridged) {
        return { job: bridged, outputUrl: openPrintJobOutput(bridged) };
      }
    }
  }
  const completed = await waitForPrintJob(tenantId, detail.job_id);
  return { job: completed, outputUrl: openPrintJobOutput(completed) };
};
