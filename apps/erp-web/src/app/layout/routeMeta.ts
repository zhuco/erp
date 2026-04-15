import { matchPath } from 'react-router-dom';

type RouteMeta = {
  pattern: string;
  title: string;
};

const dynamicRouteMeta: RouteMeta[] = [
  { pattern: '/system/profile', title: '个人信息' },
  { pattern: '/account/profile', title: '个人信息' },
  { pattern: '/orders/:order_id', title: '订单详情' },
  { pattern: '/after-sales/:after_sales_id', title: '售后详情' },
  { pattern: '/after-sales/:after_sales_id/receive', title: '售后收货' },
  { pattern: '/after-sale-cases/:case_id', title: '平台售后详情' },
  { pattern: '/wms/inbounds/:doc_id', title: '入库单详情' },
  { pattern: '/wms/inbounds/:doc_id/receive', title: '入库收货' },
  { pattern: '/wms/inbounds/:doc_id/putaway', title: '入库上架' },
  { pattern: '/procurement/orders/:po_id', title: '采购订单详情' },
  { pattern: '/kits/:kit_sku_id', title: '组合装编辑' },
  { pattern: '/system/erp-data-grid-next', title: 'AG Grid Enterprise 演示' },
  { pattern: '/system/ag-grid-enterprise', title: 'AG Grid Enterprise 演示' },
  { pattern: '/system/demo/ag-grid-enterprise', title: 'AG Grid Enterprise 演示' },
];

export type RouteTitleResult = {
  title: string;
  matched: boolean;
  source: 'menu' | 'dynamic' | 'fallback';
};

export const resolveRouteTitleInfo = (
  pathname: string,
  menuTitleMap: Map<string, string>
): RouteTitleResult => {
  const exact = menuTitleMap.get(pathname);
  if (exact) {
    return { title: exact, matched: true, source: 'menu' };
  }
  const matched = dynamicRouteMeta.find((meta) => matchPath(meta.pattern, pathname));
  if (matched) {
    return { title: matched.title, matched: true, source: 'dynamic' };
  }
  return { title: pathname, matched: false, source: 'fallback' };
};

export const resolveRouteTitle = (
  pathname: string,
  menuTitleMap: Map<string, string>
): string => resolveRouteTitleInfo(pathname, menuTitleMap).title;
