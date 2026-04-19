import { describe, expect, it } from 'vitest';

import type { HeaderModuleItem } from './headerMenuModel';
import {
  resolveSharedAnchorRect,
  resolveSharedPanelGeometry,
  resolveSharedPanelMaxHeight,
  resolveSharedPanelToggle,
  resolveSharedPanelVisibleHeight,
  SHARED_SUBMENU_VERTICAL_GAP,
} from './headerSharedSubmenu';

const groupedModule: HeaderModuleItem = {
  key: 'sales',
  label: '销售',
  defaultPath: '/orders/review',
  leafPaths: ['/orders/review', '/orders'],
  groups: [
    {
      key: 'sales-orders',
      label: '订单中心',
      items: [{ key: 'orders-review', label: '审单订单', path: '/orders/review' }],
    },
  ],
};

const directModule: HeaderModuleItem = {
  key: 'dashboard',
  label: '首页',
  defaultPath: '/dashboard',
  leafPaths: ['/dashboard'],
  groups: [],
};

describe('headerSharedSubmenu', () => {
  it('opens grouped modules and closes when the same module is clicked again', () => {
    expect(resolveSharedPanelToggle(null, groupedModule)).toEqual({
      nextOpenModuleKey: 'sales',
    });

    expect(resolveSharedPanelToggle('sales', groupedModule)).toEqual({
      nextOpenModuleKey: null,
    });
  });

  it('navigates direct modules without opening the shared panel', () => {
    expect(resolveSharedPanelToggle('sales', directModule)).toEqual({
      navigateTo: '/dashboard',
      nextOpenModuleKey: null,
    });
  });

  it('derives fixed panel geometry from the shared anchor only', () => {
    expect(
      resolveSharedPanelGeometry({
        bottom: 96,
        height: 40,
        left: 120,
        right: 920,
        top: 56,
        width: 800,
      }),
    ).toEqual({
      left: 120,
      right: 920,
      top: 96,
      width: 800,
    });
  });

  it('keeps the shared submenu close to the primary menu with a small vertical gap', () => {
    expect(
      resolveSharedAnchorRect({
        bottom: 96,
        height: 40,
        left: 120,
        right: 920,
        top: 56,
        width: 800,
      }),
    ).toEqual({
      bottom: 96 + SHARED_SUBMENU_VERTICAL_GAP,
      height: 40,
      left: 120,
      right: 920,
      top: 56,
      width: 800,
    });
  });

  it('clamps panel height to the shared panel max height', () => {
    expect(resolveSharedPanelMaxHeight(900, 180, 24)).toBe(696);
    expect(resolveSharedPanelVisibleHeight(420, 320)).toBe(320);
    expect(resolveSharedPanelVisibleHeight(180, 320)).toBe(180);
  });
});
