const normalizePath = (id: string) => id.replace(/\\/g, '/');

const resolveOrdersChunk = (normalizedId: string) => {
  if (!normalizedId.includes('/src/pages/orders/')) {
    return undefined;
  }

  if (
    normalizedId.includes('/src/pages/orders/OrderQueryPage') ||
    normalizedId.includes('/src/pages/orders/components/OrderQuery')
  ) {
    return 'page-orders-query';
  }

  if (
    normalizedId.includes('/src/pages/orders/OrderDetailPage') ||
    normalizedId.includes('/src/pages/orders/ShipOrderDrawer')
  ) {
    return 'page-orders-detail';
  }

  return 'page-orders';
};

const resolvePageChunk = (normalizedId: string) => {
  const ordersChunk = resolveOrdersChunk(normalizedId);
  if (ordersChunk) {
    return ordersChunk;
  }

  if (normalizedId.includes('/src/pages/wms/')) {
    return 'page-wms';
  }

  if (normalizedId.includes('/src/pages/Dashboard')) {
    return 'page-dashboard';
  }

  if (
    normalizedId.includes('/src/pages/channels/') ||
    normalizedId.includes('/src/pages/sales/')
  ) {
    return 'page-commerce';
  }

  if (normalizedId.includes('/src/pages/afterSales/AfterSalesListPage')) {
    return 'page-after-sales-list';
  }

  if (normalizedId.includes('/src/pages/afterSales/AfterSalesDetailPage')) {
    return 'page-after-sales-detail';
  }

  if (normalizedId.includes('/src/pages/afterSales/AfterSalesSettingsPage')) {
    return 'page-after-sales-settings';
  }

  return undefined;
};

const resolveNodeModulesChunk = (normalizedId: string) => {
  if (!normalizedId.includes('/node_modules/')) {
    return undefined;
  }

  if (normalizedId.includes('/node_modules/@tanstack/')) {
    return 'vendor-data';
  }

  if (normalizedId.includes('/node_modules/dayjs/')) {
    return 'vendor-core';
  }

  if (normalizedId.includes('/node_modules/xlsx/')) {
    return 'vendor-heavy';
  }

  if (normalizedId.includes('/node_modules/ag-grid-enterprise/')) {
    return 'vendor-grid-enterprise';
  }

  if (
    normalizedId.includes('/node_modules/ag-grid-react/') ||
    normalizedId.includes('/node_modules/ag-grid-community/')
  ) {
    return 'vendor-grid-core';
  }

  if (
    normalizedId.includes('/node_modules/antd/') ||
    normalizedId.includes('/node_modules/@ant-design/') ||
    normalizedId.includes('/node_modules/rc-') ||
    normalizedId.includes('/node_modules/@rc-component/') ||
    normalizedId.includes('/node_modules/@babel/runtime/') ||
    normalizedId.includes('/node_modules/clsx/')
  ) {
    return 'vendor-ui';
  }

  if (
    normalizedId.includes('/node_modules/react/') ||
    normalizedId.includes('/node_modules/react-dom/') ||
    normalizedId.includes('/node_modules/react-router-dom/') ||
    normalizedId.includes('/node_modules/react-draggable/') ||
    normalizedId.includes('/node_modules/zustand/')
  ) {
    return 'vendor-react';
  }

  return undefined;
};

const ROLLUP_RUNTIME_HELPERS = new Set([
  '\u0000commonjsHelpers.js',
  '\u0000vite/preload-helper.js',
  '\u0000vite/modulepreload-polyfill.js',
]);

export const resolveManualChunk = (id: string) => {
  if (!id) {
    return undefined;
  }

  if (ROLLUP_RUNTIME_HELPERS.has(id)) {
    return 'vendor-runtime';
  }

  if (id.startsWith('\u0000commonjs-proxy:')) {
    return resolveManualChunk(id.slice('\u0000commonjs-proxy:'.length));
  }

  if (id.startsWith('\u0000')) {
    return undefined;
  }

  const normalizedId = normalizePath(id);

  const pageChunk = resolvePageChunk(normalizedId);
  if (pageChunk) {
    return pageChunk;
  }

  return resolveNodeModulesChunk(normalizedId);
};
