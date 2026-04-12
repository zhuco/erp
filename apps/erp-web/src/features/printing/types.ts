import type { PlatformCode } from '@/features/platform';

export type PrintShippingLabelPayload = {
  order_id: string;
  platform: PlatformCode;
  shop_id: string;
  shipment_id?: string;
  tracking_no?: string | null;
  carrier_code?: string | null;
};
