import type { ReactNode } from 'react';

import { useSessionStore } from '@/features/sessionStore';

export type Permission =
  | 'orders.read'
  | 'orders.write'
  | 'orders.read_raw'
  | 'orders.review'
  | 'orders.review.read'
  | 'orders.review.write'
  | 'orders.exception.assign'
  | 'orders.exception.resolve'
  | 'orders.fulfillment.submit'
  | 'shipping.read'
  | 'shipping.write'
  | 'aftersales.read'
  | 'aftersales.read_raw'
  | 'aftersales.write'
  | 'inventory.read'
  | 'inventory.write'
  | 'inventory_insights.read'
  | 'skus.read'
  | 'skus.write'
  | 'products.write'
  | 'kits.read'
  | 'kits.write'
  | 'channels.read'
  | 'channels.write'
  | 'couriers.read'
  | 'couriers.write'
  | 'suppliers.read'
  | 'suppliers.write'
  | 'customers.read'
  | 'customers.write'
  | 'purchase_orders.read'
  | 'purchase_orders.write'
  | 'purchase_suggestions.read'
  | 'purchase_returns.read'
  | 'purchase_returns.write'
  | 'procurement_requests.read'
  | 'procurement_requests.write'
  | 'procurement_plans.read'
  | 'procurement_plans.write'
  | 'procurement_sourcing.read'
  | 'procurement_sourcing.write'
  | 'procurement_inspections.read'
  | 'procurement_inspections.write'
  | 'procurement_reconciliations.read'
  | 'procurement_reconciliations.write'
  | 'supplier_performances.read'
  | 'supplier_performances.write'
  | 'channel_sku_mappings.read'
  | 'channel_sku_mappings.write'
  | 'mappings.read'
  | 'mappings.write'
  | 'warehouses.read'
  | 'warehouses.write'
  | 'carriers.read'
  | 'carriers.write'
  | 'logistics_products.read'
  | 'logistics_products.write'
  | 'logistics_prices.read'
  | 'logistics_prices.write'
  | 'logistics_match_rules.read'
  | 'logistics_match_rules.write'
  | 'order_tags.read'
  | 'order_tags.write'
  | 'sku_tags.read'
  | 'sku_tags.write'
  | 'shop_products.read'
  | 'shop_products.write'
  | 'marketing_prices.read'
  | 'marketing_prices.write'
  | 'marketing_promotions.read'
  | 'marketing_promotions.write'
  | 'marketing_gifts.read'
  | 'marketing_gifts.write'
  | 'marketing_coupons.read'
  | 'marketing_coupons.write'
  | 'marketing_costs.read'
  | 'marketing_costs.write'
  | 'marketing_postmortems.read'
  | 'marketing_postmortems.write'
  | 'print_devices.read'
  | 'print_devices.write'
  | 'print_templates.read'
  | 'print_templates.write'
  | 'print_jobs.read'
  | 'print_jobs.write'
  | 'print_mappings.read'
  | 'print_mappings.write'
  | 'reports.read'
  | 'stats.read'
  | 'exports.read'
  | 'exports.write'
  | 'operation_logs.read'
  | 'settings.read'
  | 'roles.read'
  | 'roles.write'
  | 'users.read'
  | 'users.write'
  | 'permissions.read'
  | 'audit.read'
  | 'dlq.read'
  | 'dlq.replay'
  | 'OMS.ORDER.READ'
  | 'OMS.ORDER.WRITE'
  | 'OMS.ORDER.RAW'
  | 'OMS.ORDER.REVIEW'
  | 'OMS.SHIPPING.READ'
  | 'OMS.SHIPPING.WRITE'
  | 'OMS.AFTERSALES.READ'
  | 'OMS.AFTERSALES.RAW'
  | 'OMS.AFTERSALES.WRITE'
  | 'WMS.INVENTORY.READ'
  | 'WMS.INVENTORY.WRITE'
  | 'wms.read'
  | 'wms.write'
  | 'wms.wave.read'
  | 'wms.wave.write'
  | 'wms.pick.execute'
  | 'wms.pack.execute'
  | 'wms.handover.execute'
  | 'wms.handover.rollback'
  | 'wms.inbounds.read'
  | 'wms.inbounds.write'
  | 'shipping.push.retry'
  | 'finance.read'
  | 'finance.write'
  | 'BASIC.SKU.READ'
  | 'BASIC.SKU.WRITE'
  | 'BASIC.KIT.READ'
  | 'BASIC.KIT.WRITE'
  | 'BASIC.CHANNEL.READ'
  | 'BASIC.CHANNEL.WRITE'
  | 'BASIC.COURIER.READ'
  | 'BASIC.COURIER.WRITE'
  | 'BASIC.SUPPLIER.READ'
  | 'BASIC.SUPPLIER.WRITE'
  | 'BASIC.CUSTOMER.READ'
  | 'BASIC.CUSTOMER.WRITE'
  | 'BASIC.MAPPING.READ'
  | 'BASIC.MAPPING.WRITE'
  | 'BASIC.WAREHOUSE.READ'
  | 'BASIC.WAREHOUSE.WRITE'
  | 'BASIC.CARRIER.READ'
  | 'BASIC.CARRIER.WRITE'
  | 'BASIC.LOGISTICS_PRODUCT.READ'
  | 'BASIC.LOGISTICS_PRODUCT.WRITE'
  | 'BASIC.LOGISTICS_PRICE.READ'
  | 'BASIC.LOGISTICS_PRICE.WRITE'
  | 'BASIC.LOGISTICS_MATCH_RULE.READ'
  | 'BASIC.LOGISTICS_MATCH_RULE.WRITE'
  | 'BASIC.ORDER_TAG.READ'
  | 'BASIC.ORDER_TAG.WRITE'
  | 'BASIC.SKU_TAG.READ'
  | 'BASIC.SKU_TAG.WRITE'
  | 'BASIC.SHOP_PRODUCT.READ'
  | 'BASIC.SHOP_PRODUCT.WRITE'
  | 'MARKETING.PRICE.READ'
  | 'MARKETING.PRICE.WRITE'
  | 'MARKETING.PROMOTION.READ'
  | 'MARKETING.PROMOTION.WRITE'
  | 'MARKETING.GIFT.READ'
  | 'MARKETING.GIFT.WRITE'
  | 'MARKETING.COUPON.READ'
  | 'MARKETING.COUPON.WRITE'
  | 'MARKETING.COST.READ'
  | 'MARKETING.COST.WRITE'
  | 'MARKETING.POSTMORTEM.READ'
  | 'MARKETING.POSTMORTEM.WRITE'
  | 'PROCUREMENT.REQUEST.READ'
  | 'PROCUREMENT.REQUEST.WRITE'
  | 'PROCUREMENT.PLAN.READ'
  | 'PROCUREMENT.PLAN.WRITE'
  | 'PROCUREMENT.SOURCING.READ'
  | 'PROCUREMENT.SOURCING.WRITE'
  | 'PROCUREMENT.INSPECTION.READ'
  | 'PROCUREMENT.INSPECTION.WRITE'
  | 'PROCUREMENT.RECONCILIATION.READ'
  | 'PROCUREMENT.RECONCILIATION.WRITE'
  | 'PROCUREMENT.PERFORMANCE.READ'
  | 'PROCUREMENT.PERFORMANCE.WRITE'
  | 'PRINT.DEVICE.READ'
  | 'PRINT.DEVICE.WRITE'
  | 'PRINT.TEMPLATE.READ'
  | 'PRINT.TEMPLATE.WRITE'
  | 'PRINT.JOB.READ'
  | 'PRINT.JOB.WRITE'
  | 'PRINT.MAPPING.READ'
  | 'PRINT.MAPPING.WRITE'
  | 'OPS.OPERATION_LOG.READ'
  | 'OPS.REPORT.READ'
  | 'OPS.STATS.READ'
  | 'OPS.EXPORT.READ'
  | 'OPS.EXPORT.WRITE'
  | 'SYS.PERMISSION.READ'
  | 'SYS.ROLE.READ'
  | 'SYS.ROLE.WRITE'
  | 'SYS.USER.READ'
  | 'SYS.USER.WRITE'
  | 'SYS.AUDIT.READ'
  | 'SYS.SETTINGS.READ'
  | 'SYS.SETTINGS.WRITE'
  | 'SYS.DLQ.READ'
  | 'SYS.DLQ.REPLAY';

export const defaultPermissions: Permission[] = [
  'OMS.ORDER.READ',
  'OMS.ORDER.WRITE',
  'OMS.ORDER.RAW',
  'OMS.ORDER.REVIEW',
  'orders.review.read',
  'orders.review.write',
  'orders.exception.assign',
  'orders.exception.resolve',
  'orders.fulfillment.submit',
  'OMS.SHIPPING.READ',
  'OMS.SHIPPING.WRITE',
  'OMS.AFTERSALES.READ',
  'OMS.AFTERSALES.RAW',
  'OMS.AFTERSALES.WRITE',
  'WMS.INVENTORY.READ',
  'WMS.INVENTORY.WRITE',
  'inventory_insights.read',
  'wms.read',
  'wms.write',
  'wms.wave.read',
  'wms.wave.write',
  'wms.pick.execute',
  'wms.pack.execute',
  'wms.handover.execute',
  'wms.handover.rollback',
  'wms.inbounds.read',
  'wms.inbounds.write',
  'shipping.push.retry',
  'finance.read',
  'finance.write',
  'suppliers.read',
  'suppliers.write',
  'customers.read',
  'customers.write',
  'purchase_orders.read',
  'purchase_orders.write',
  'purchase_suggestions.read',
  'purchase_returns.read',
  'purchase_returns.write',
  'procurement_requests.read',
  'procurement_requests.write',
  'procurement_plans.read',
  'procurement_plans.write',
  'procurement_sourcing.read',
  'procurement_sourcing.write',
  'procurement_inspections.read',
  'procurement_inspections.write',
  'procurement_reconciliations.read',
  'procurement_reconciliations.write',
  'supplier_performances.read',
  'supplier_performances.write',
  'BASIC.SKU.READ',
  'BASIC.SKU.WRITE',
  'BASIC.KIT.READ',
  'BASIC.KIT.WRITE',
  'BASIC.CHANNEL.READ',
  'BASIC.CHANNEL.WRITE',
  'BASIC.COURIER.READ',
  'BASIC.COURIER.WRITE',
  'BASIC.SUPPLIER.READ',
  'BASIC.SUPPLIER.WRITE',
  'BASIC.MAPPING.READ',
  'BASIC.MAPPING.WRITE',
  'BASIC.WAREHOUSE.READ',
  'BASIC.WAREHOUSE.WRITE',
  'BASIC.CARRIER.READ',
  'BASIC.CARRIER.WRITE',
  'BASIC.LOGISTICS_PRODUCT.READ',
  'BASIC.LOGISTICS_PRODUCT.WRITE',
  'BASIC.LOGISTICS_PRICE.READ',
  'BASIC.LOGISTICS_PRICE.WRITE',
  'BASIC.LOGISTICS_MATCH_RULE.READ',
  'BASIC.LOGISTICS_MATCH_RULE.WRITE',
  'BASIC.ORDER_TAG.READ',
  'BASIC.ORDER_TAG.WRITE',
  'BASIC.SKU_TAG.READ',
  'BASIC.SKU_TAG.WRITE',
  'BASIC.SHOP_PRODUCT.READ',
  'BASIC.SHOP_PRODUCT.WRITE',
  'MARKETING.PRICE.READ',
  'MARKETING.PRICE.WRITE',
  'MARKETING.PROMOTION.READ',
  'MARKETING.PROMOTION.WRITE',
  'MARKETING.GIFT.READ',
  'MARKETING.GIFT.WRITE',
  'MARKETING.COUPON.READ',
  'MARKETING.COUPON.WRITE',
  'MARKETING.COST.READ',
  'MARKETING.COST.WRITE',
  'MARKETING.POSTMORTEM.READ',
  'MARKETING.POSTMORTEM.WRITE',
  'PRINT.DEVICE.READ',
  'PRINT.DEVICE.WRITE',
  'PRINT.TEMPLATE.READ',
  'PRINT.TEMPLATE.WRITE',
  'PRINT.JOB.READ',
  'PRINT.JOB.WRITE',
  'PRINT.MAPPING.READ',
  'PRINT.MAPPING.WRITE',
  'OPS.OPERATION_LOG.READ',
  'OPS.REPORT.READ',
  'OPS.STATS.READ',
  'OPS.EXPORT.READ',
  'OPS.EXPORT.WRITE',
  'SYS.PERMISSION.READ',
  'SYS.ROLE.READ',
  'SYS.ROLE.WRITE',
  'SYS.USER.READ',
  'SYS.USER.WRITE',
  'SYS.AUDIT.READ',
  'SYS.SETTINGS.READ',
  'SYS.SETTINGS.WRITE',
  'SYS.DLQ.READ',
  'SYS.DLQ.REPLAY'
];

const permissionAliases: Record<string, Permission> = {
  'orders.read': 'OMS.ORDER.READ',
  'orders.write': 'OMS.ORDER.WRITE',
  'orders.read_raw': 'OMS.ORDER.RAW',
  'orders.review': 'OMS.ORDER.REVIEW',
  'orders.review.read': 'OMS.ORDER.REVIEW',
  'orders.review.write': 'OMS.ORDER.REVIEW',
  'orders.exception.assign': 'OMS.ORDER.REVIEW',
  'orders.exception.resolve': 'OMS.ORDER.REVIEW',
  'orders.fulfillment.submit': 'OMS.SHIPPING.WRITE',
  'shipping.read': 'OMS.SHIPPING.READ',
  'shipping.write': 'OMS.SHIPPING.WRITE',
  'aftersales.read': 'OMS.AFTERSALES.READ',
  'aftersales.read_raw': 'OMS.AFTERSALES.RAW',
  'aftersales.write': 'OMS.AFTERSALES.WRITE',
  'inventory.read': 'WMS.INVENTORY.READ',
  'inventory.write': 'WMS.INVENTORY.WRITE',
  'wms.read': 'WMS.INVENTORY.READ',
  'wms.write': 'WMS.INVENTORY.WRITE',
  'wms.wave.read': 'WMS.INVENTORY.READ',
  'wms.wave.write': 'WMS.INVENTORY.WRITE',
  'wms.pick.execute': 'WMS.INVENTORY.WRITE',
  'wms.pack.execute': 'WMS.INVENTORY.WRITE',
  'wms.handover.execute': 'WMS.INVENTORY.WRITE',
  'wms.handover.rollback': 'WMS.INVENTORY.WRITE',
  'wms.inbounds.read': 'WMS.INVENTORY.READ',
  'wms.inbounds.write': 'WMS.INVENTORY.WRITE',
  'shipping.push.retry': 'OMS.SHIPPING.WRITE',
  'skus.read': 'BASIC.SKU.READ',
  'skus.write': 'BASIC.SKU.WRITE',
  'products.write': 'BASIC.SKU.WRITE',
  'kits.read': 'BASIC.KIT.READ',
  'kits.write': 'BASIC.KIT.WRITE',
  'channels.read': 'BASIC.CHANNEL.READ',
  'channels.write': 'BASIC.CHANNEL.WRITE',
  'couriers.read': 'BASIC.COURIER.READ',
  'couriers.write': 'BASIC.COURIER.WRITE',
  'suppliers.read': 'BASIC.SUPPLIER.READ',
  'suppliers.write': 'BASIC.SUPPLIER.WRITE',
  'customers.read': 'BASIC.CUSTOMER.READ',
  'customers.write': 'BASIC.CUSTOMER.WRITE',
  'procurement_requests.read': 'PROCUREMENT.REQUEST.READ',
  'procurement_requests.write': 'PROCUREMENT.REQUEST.WRITE',
  'procurement_plans.read': 'PROCUREMENT.PLAN.READ',
  'procurement_plans.write': 'PROCUREMENT.PLAN.WRITE',
  'procurement_sourcing.read': 'PROCUREMENT.SOURCING.READ',
  'procurement_sourcing.write': 'PROCUREMENT.SOURCING.WRITE',
  'procurement_inspections.read': 'PROCUREMENT.INSPECTION.READ',
  'procurement_inspections.write': 'PROCUREMENT.INSPECTION.WRITE',
  'procurement_reconciliations.read': 'PROCUREMENT.RECONCILIATION.READ',
  'procurement_reconciliations.write': 'PROCUREMENT.RECONCILIATION.WRITE',
  'supplier_performances.read': 'PROCUREMENT.PERFORMANCE.READ',
  'supplier_performances.write': 'PROCUREMENT.PERFORMANCE.WRITE',
  'channel_sku_mappings.read': 'BASIC.MAPPING.READ',
  'channel_sku_mappings.write': 'BASIC.MAPPING.WRITE',
  'mappings.read': 'BASIC.MAPPING.READ',
  'mappings.write': 'BASIC.MAPPING.WRITE',
  'warehouses.read': 'BASIC.WAREHOUSE.READ',
  'warehouses.write': 'BASIC.WAREHOUSE.WRITE',
  'carriers.read': 'BASIC.CARRIER.READ',
  'carriers.write': 'BASIC.CARRIER.WRITE',
  'logistics_products.read': 'BASIC.LOGISTICS_PRODUCT.READ',
  'logistics_products.write': 'BASIC.LOGISTICS_PRODUCT.WRITE',
  'logistics_prices.read': 'BASIC.LOGISTICS_PRICE.READ',
  'logistics_prices.write': 'BASIC.LOGISTICS_PRICE.WRITE',
  'logistics_match_rules.read': 'BASIC.LOGISTICS_MATCH_RULE.READ',
  'logistics_match_rules.write': 'BASIC.LOGISTICS_MATCH_RULE.WRITE',
  'order_tags.read': 'BASIC.ORDER_TAG.READ',
  'order_tags.write': 'BASIC.ORDER_TAG.WRITE',
  'sku_tags.read': 'BASIC.SKU_TAG.READ',
  'sku_tags.write': 'BASIC.SKU_TAG.WRITE',
  'shop_products.read': 'BASIC.SHOP_PRODUCT.READ',
  'shop_products.write': 'BASIC.SHOP_PRODUCT.WRITE',
  'marketing_prices.read': 'MARKETING.PRICE.READ',
  'marketing_prices.write': 'MARKETING.PRICE.WRITE',
  'marketing_promotions.read': 'MARKETING.PROMOTION.READ',
  'marketing_promotions.write': 'MARKETING.PROMOTION.WRITE',
  'marketing_gifts.read': 'MARKETING.GIFT.READ',
  'marketing_gifts.write': 'MARKETING.GIFT.WRITE',
  'marketing_coupons.read': 'MARKETING.COUPON.READ',
  'marketing_coupons.write': 'MARKETING.COUPON.WRITE',
  'marketing_costs.read': 'MARKETING.COST.READ',
  'marketing_costs.write': 'MARKETING.COST.WRITE',
  'marketing_postmortems.read': 'MARKETING.POSTMORTEM.READ',
  'marketing_postmortems.write': 'MARKETING.POSTMORTEM.WRITE',
  'print_devices.read': 'PRINT.DEVICE.READ',
  'print_devices.write': 'PRINT.DEVICE.WRITE',
  'print_templates.read': 'PRINT.TEMPLATE.READ',
  'print_templates.write': 'PRINT.TEMPLATE.WRITE',
  'print_jobs.read': 'PRINT.JOB.READ',
  'print_jobs.write': 'PRINT.JOB.WRITE',
  'print_mappings.read': 'PRINT.MAPPING.READ',
  'print_mappings.write': 'PRINT.MAPPING.WRITE',
  'reports.read': 'OPS.REPORT.READ',
  'stats.read': 'OPS.STATS.READ',
  'exports.read': 'OPS.EXPORT.READ',
  'exports.write': 'OPS.EXPORT.WRITE',
  'operation_logs.read': 'OPS.OPERATION_LOG.READ',
  'settings.read': 'SYS.SETTINGS.READ',
  'roles.read': 'SYS.ROLE.READ',
  'roles.write': 'SYS.ROLE.WRITE',
  'users.read': 'SYS.USER.READ',
  'users.write': 'SYS.USER.WRITE',
  'permissions.read': 'SYS.PERMISSION.READ',
  'audit.read': 'SYS.AUDIT.READ',
  'dlq.read': 'SYS.DLQ.READ',
  'dlq.replay': 'SYS.DLQ.REPLAY'
};

const normalizePermission = (permission: string): Permission =>
  permissionAliases[permission] ?? (permission as Permission);

export const hasPermission = (permissions: Permission[], required: Permission): boolean => {
  const normalizedRequired = normalizePermission(required);
  return permissions.map(normalizePermission).includes(normalizedRequired);
};

export const useCan = (permission: Permission): boolean => {
  const permissions = useSessionStore((state) => state.permissions);
  return hasPermission(permissions, permission);
};

type CanProps = {
  permission: Permission;
  children: ReactNode;
  fallback?: ReactNode;
};

export const Can = ({ permission, children, fallback = null }: CanProps) =>
  useCan(permission) ? <>{children}</> : <>{fallback}</>;
