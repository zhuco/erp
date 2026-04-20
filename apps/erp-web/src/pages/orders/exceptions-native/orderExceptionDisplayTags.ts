import type { OrderExceptionDetail } from '@/api/orders';
import { resolveOrderDisplayTags, type OrderDisplayTag } from '@/features/orderTags';

type DetailTagItem = {
  tag?: string | null;
};

export type OrderExceptionDetailPackage = {
  order?: Record<string, unknown> | null;
  items?: Record<string, unknown>[] | null;
  tags?: DetailTagItem[] | null;
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

export const resolveOrderExceptionDetailPackage = (
  detail?: OrderExceptionDetail | null,
): OrderExceptionDetailPackage | null => {
  const rawDetailOrder = detail?.order;
  if (!isRecord(rawDetailOrder)) {
    return null;
  }

  const candidate = rawDetailOrder as Record<string, unknown>;
  if ('order' in candidate || 'items' in candidate || 'tags' in candidate) {
    return {
      order: isRecord(candidate.order) ? candidate.order : null,
      items: Array.isArray(candidate.items) ? (candidate.items as Record<string, unknown>[]) : undefined,
      tags: Array.isArray(candidate.tags) ? (candidate.tags as DetailTagItem[]) : undefined,
    };
  }

  return {
    order: candidate,
    items: undefined,
    tags: undefined,
  };
};

export const extractOrderExceptionExplicitTags = (detail?: OrderExceptionDetail | null): string[] => {
  const detailPackage = resolveOrderExceptionDetailPackage(detail);
  return Array.from(
    new Set(
      (detailPackage?.tags ?? [])
        .map((item) => String(item?.tag ?? '').trim())
        .filter(Boolean),
    ),
  );
};

export const buildOrderExceptionDisplayTagRecord = (
  summary?: Record<string, unknown> | null,
  detail?: OrderExceptionDetail | null,
): Record<string, unknown> => {
  const detailPackage = resolveOrderExceptionDetailPackage(detail);
  const detailOrder = detailPackage?.order ?? null;
  const explicitTags = extractOrderExceptionExplicitTags(detail);
  const rowRecord = isRecord(summary) ? summary : {};

  return {
    ...(detailOrder ?? {}),
    ...rowRecord,
    blocking_reason: detail?.blocking_reason ?? rowRecord.blocking_reason ?? null,
    latest_action: detail?.latest_action ?? rowRecord.latest_action ?? null,
    ...(explicitTags.length > 0
      ? { tags: explicitTags }
      : rowRecord.tags !== undefined
        ? { tags: rowRecord.tags }
        : {}),
  };
};

export const resolveOrderExceptionDisplayTags = (
  summary?: Record<string, unknown> | null,
  detail?: OrderExceptionDetail | null,
  options?: {
    now?: Date;
  },
): OrderDisplayTag[] => resolveOrderDisplayTags(buildOrderExceptionDisplayTagRecord(summary, detail), options);
