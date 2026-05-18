const COMPACT_RIGHT_COLLAPSED = '1';
const COMPACT_BOTTOM_COLLAPSED = '1';
const EXPANDED_RIGHT_COLLAPSED = '0';
const EXPANDED_BOTTOM_COLLAPSED = '0';
const QUERY_RETURN_OMIT_KEYS = ['edit', 'rightCollapsed', 'bottomCollapsed'] as const;

const buildSearchParams = (search: string) => {
  const raw = search.startsWith('?') ? search.slice(1) : search;
  return new URLSearchParams(raw);
};

const normalizeSearch = (params: URLSearchParams) => {
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const applyCompactWorkbenchLayout = (
  params: URLSearchParams,
  mode: 'force' | 'fill-missing',
) => {
  params.delete('edit');
  if (mode === 'force' || !params.has('rightCollapsed')) {
    params.set('rightCollapsed', COMPACT_RIGHT_COLLAPSED);
  }
  if (mode === 'force' || !params.has('bottomCollapsed')) {
    params.set('bottomCollapsed', COMPACT_BOTTOM_COLLAPSED);
  }
  return params;
};

export const createGoodsEditSearchParams = (search: string) => {
  return normalizeSearch(applyCompactWorkbenchLayout(buildSearchParams(search), 'force'));
};

export const createGoodsCompactWorkbenchSearchParams = (search: string) =>
  normalizeSearch(applyCompactWorkbenchLayout(buildSearchParams(search), 'fill-missing'));

export const buildGoodsCompactWorkbenchPath = (pathname: string, search: string) =>
  `${pathname}${createGoodsCompactWorkbenchSearchParams(search)}`;

export const buildGoodsProductCreatePath = (search: string) =>
  `/goods/products/new${createGoodsEditSearchParams(search)}`;

export const buildGoodsKitCreatePath = (search: string) =>
  `/goods/kits/new${createGoodsEditSearchParams(search)}`;

export const buildGoodsProductEditPath = (productId: string, search: string) =>
  `/goods/products/${encodeURIComponent(productId)}/edit${createGoodsEditSearchParams(search)}`;

export const buildGoodsKitEditPath = (kitSkuId: string, search: string) =>
  `/goods/kits/${encodeURIComponent(kitSkuId)}/edit${createGoodsEditSearchParams(search)}`;

export const buildGoodsQueryPath = (pathname: string, search: string) => {
  const params = buildSearchParams(search);
  QUERY_RETURN_OMIT_KEYS.forEach((key) => params.delete(key));
  params.set('rightCollapsed', EXPANDED_RIGHT_COLLAPSED);
  params.set('bottomCollapsed', EXPANDED_BOTTOM_COLLAPSED);
  return `${pathname}${normalizeSearch(params)}`;
};
