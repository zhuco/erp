import type { OrderSummary } from '@/api/orders';
import { platformOptions } from '@/features/platform';
import dayjs from 'dayjs';

export const defaultPageSize = 100;
export const pageSizeOptions = ['50', '100', '200', '300', '500', '800', '1000', '3000', '5000'];

export const normalizeMulti = (value?: string | string[]) => {
  if (!value) return [];
  if (Array.isArray(value)) return Array.from(new Set(value.map((item) => item.trim()).filter(Boolean)));
  const normalized = value.trim();
  return normalized ? [normalized] : [];
};

export const money = (value?: number | null) =>
  value == null
    ? '-'
    : (value / 100).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

export const moneyValue = (value?: number | null) =>
  (value ?? 0).toLocaleString('zh-CN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });

const formatDateTimeByPattern = (value: string | null | undefined, pattern: string) => {
  const normalized = String(value ?? '').trim();
  if (!normalized) return '-';
  const formatted = dayjs(normalized);
  if (!formatted.isValid()) {
    return normalized;
  }
  return formatted.format(pattern);
};

export const formatDateTime = (value?: string | null) => {
  return formatDateTimeByPattern(value, 'YYYY-MM-DD HH:mm');
};

export const formatDateTimeSeconds = (value?: string | null) => {
  return formatDateTimeByPattern(value, 'YYYY-MM-DD HH:mm:ss');
};

export const platformLabel = (value: string) => {
  const option = platformOptions.find((item) => String(item.value) === value);
  if (!option) return value;
  return typeof option.label === 'string' ? option.label : value;
};

export const platformLabelToCodeMap: Record<string, string> = {
  淘宝: 'taobao',
  京东: 'jd',
  拼多多: 'pdd',
  抖音: 'douyin',
};

export const resolveOrderRowKey = (record: OrderSummary | null | undefined) => {
  if (!record) {
    return '';
  }
  const id = String(record.id ?? '').trim();
  if (id) {
    return id;
  }
  const platformOrderId = String(record.platform_order_id ?? '').trim();
  if (platformOrderId) {
    return platformOrderId;
  }
  const fallback = [
    String(record.shop_id ?? '').trim(),
    String(record.receiver_phone ?? '').trim(),
    String(record.created_at ?? '').trim(),
  ]
    .filter(Boolean)
    .join('|');
  return fallback ? `fallback:${fallback}` : '';
};

export const resolveOrderActionId = (record: OrderSummary | null | undefined) => {
  if (!record) {
    return '';
  }
  const id = String(record.id ?? '').trim();
  if (id) {
    return id;
  }
  const platformOrderId = String(record.platform_order_id ?? '').trim();
  if (platformOrderId) {
    return platformOrderId;
  }
  return '';
};
