export type WorkbenchRecordEditorOption = {
  label: string;
  value: string;
};

export type WorkbenchRecordLineItemDraft = {
  draftKey: string;
  id?: string;
  platformSkuId?: string | null;
  outerSkuId?: string | null;
  title?: string | null;
  quantity: number;
  priceCent: number;
};

export type WorkbenchOrderRecordDraft = {
  id: string;
  orderNo: string;
  platformOrderId?: string;
  platform?: string;
  shopId?: string;
  receiverName: string;
  receiverPhone: string;
  shipWarehouseId?: string;
  logisticsCompany?: string;
  buyerMessage?: string;
  sellerRemark?: string;
  items: WorkbenchRecordLineItemDraft[];
};

export const createWorkbenchRecordLineItemDraft = (
  seed?: Partial<WorkbenchRecordLineItemDraft>,
): WorkbenchRecordLineItemDraft => ({
  draftKey:
    String(seed?.draftKey ?? seed?.id ?? '').trim() ||
    `line-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
  id: String(seed?.id ?? '').trim() || undefined,
  platformSkuId: String(seed?.platformSkuId ?? '').trim() || null,
  outerSkuId: String(seed?.outerSkuId ?? '').trim() || null,
  title: String(seed?.title ?? '').trim() || null,
  quantity: Math.max(1, Math.trunc(Number(seed?.quantity ?? 1) || 1)),
  priceCent: Math.max(0, Math.trunc(Number(seed?.priceCent ?? 0) || 0)),
});
