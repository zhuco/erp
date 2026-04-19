import dayjs from 'dayjs';

const MOCK_ORDER_NO_PREFIX = 'HC';
const MOCK_ORDER_NO_PATTERN = /^HC(\d{8})(\d{4,})$/;

type MockOrderItemLike = {
  qty?: number | null;
  quantity?: number | null;
  price_cent?: number | null;
};

const normalizeDateKey = (value?: string | dayjs.Dayjs) => {
  const targetDate = dayjs(value ?? undefined);
  if (targetDate.isValid()) {
    return targetDate.format('YYYYMMDD');
  }
  return dayjs().format('YYYYMMDD');
};

const normalizePositiveInteger = (value: number) => {
  const normalizedValue = Number.isFinite(value) ? Math.trunc(value) : 1;
  return normalizedValue > 0 ? normalizedValue : 1;
};

const normalizeSequenceText = (sequence: number) => {
  const normalizedSequence = normalizePositiveInteger(sequence);
  const minWidth = normalizedSequence > 9999 ? String(normalizedSequence).length : 4;
  return String(normalizedSequence).padStart(minWidth, '0');
};

const parseMockOrderNo = (orderNo: string) => {
  const normalizedOrderNo = String(orderNo ?? '').trim().toUpperCase();
  const matched = normalizedOrderNo.match(MOCK_ORDER_NO_PATTERN);
  if (!matched) {
    return null;
  }

  return {
    dateKey: matched[1],
    sequence: Number(matched[2]),
  };
};

const readItemQuantity = (item: MockOrderItemLike) => {
  const rawQuantity = item.qty ?? item.quantity ?? 0;
  const normalizedQuantity = Number(rawQuantity);
  if (!Number.isFinite(normalizedQuantity)) {
    return 0;
  }
  return Math.max(0, Math.trunc(normalizedQuantity));
};

const readItemPriceCent = (item: MockOrderItemLike) => {
  const normalizedPriceCent = Number(item.price_cent ?? 0);
  if (!Number.isFinite(normalizedPriceCent)) {
    return 0;
  }
  return Math.max(0, Math.trunc(normalizedPriceCent));
};

export const buildMockOrderNo = (dateValue: string | dayjs.Dayjs | undefined, sequence: number) =>
  `${MOCK_ORDER_NO_PREFIX}${normalizeDateKey(dateValue)}${normalizeSequenceText(sequence)}`;

export const createMockOrderNoRegistry = (existingOrderNos: string[] = []) => {
  const sequenceByDate = new Map<string, number>();

  existingOrderNos.forEach((orderNo) => {
    const parsed = parseMockOrderNo(orderNo);
    if (!parsed || !Number.isFinite(parsed.sequence)) {
      return;
    }
    const currentSequence = sequenceByDate.get(parsed.dateKey) ?? 0;
    sequenceByDate.set(parsed.dateKey, Math.max(currentSequence, parsed.sequence));
  });

  const sync = (orderNo: string) => {
    const parsed = parseMockOrderNo(orderNo);
    if (!parsed || !Number.isFinite(parsed.sequence)) {
      return;
    }
    const currentSequence = sequenceByDate.get(parsed.dateKey) ?? 0;
    sequenceByDate.set(parsed.dateKey, Math.max(currentSequence, parsed.sequence));
  };

  const reserve = (dateValue?: string | dayjs.Dayjs) => {
    const dateKey = normalizeDateKey(dateValue);
    const nextSequence = (sequenceByDate.get(dateKey) ?? 0) + 1;
    sequenceByDate.set(dateKey, nextSequence);
    return buildMockOrderNo(dateKey, nextSequence);
  };

  return {
    reserve,
    sync,
  };
};

export const calculateMockOrderGoodsQty = (items: MockOrderItemLike[] = []) =>
  items.reduce((total, item) => total + readItemQuantity(item), 0);

export const calculateMockOrderGoodsAmountCent = (items: MockOrderItemLike[] = []) =>
  items.reduce((total, item) => total + readItemQuantity(item) * readItemPriceCent(item), 0);

export const hasMockOrderGoods = (items: MockOrderItemLike[] = []) => calculateMockOrderGoodsQty(items) > 0;

export const ensureMockOrderCanReview = (items: MockOrderItemLike[] = [], orderNo?: string) => {
  if (hasMockOrderGoods(items)) {
    return;
  }

  const normalizedOrderNo = String(orderNo ?? '').trim();
  if (normalizedOrderNo) {
    throw new Error(`订单 ${normalizedOrderNo} 无货品，不允许审核`);
  }

  throw new Error('订单无货品，不允许审核');
};
