import { describe, expect, it } from 'vitest';

import type { AppMenuItem } from '@/app/layout/menu';

import { buildDashboardQuickLinks, buildDashboardWorkbenchCards } from './dashboardWorkbenchModel';

const buildMenuItems = (paths: string[]): AppMenuItem[] =>
  paths.map((path) => ({
    path,
    name: path,
  }));

describe('dashboardWorkbenchModel', () => {
  it('builds dashboard cards only for the preserved review, channel, and settings entries', () => {
    const cards = buildDashboardWorkbenchCards(
      buildMenuItems(['/orders/review', '/orders/recheck', '/channels', '/sales/settings']),
    );

    expect(cards.map((card) => card.key)).toEqual(['review', 'channels', 'settings']);
    expect(cards[0]).toMatchObject({
      primaryAction: {
        path: '/orders/review',
      },
      actions: [expect.objectContaining({ path: '/orders/recheck' })],
    });
  });

  it('returns an empty list when none of the preserved dashboard actions are reachable', () => {
    expect(buildDashboardWorkbenchCards(buildMenuItems(['/orders']))).toEqual([]);
  });

  it('keeps dashboard quick links aligned with the reduced UI shell', () => {
    const links = buildDashboardQuickLinks(
      buildMenuItems(['/dashboard', '/orders/review', '/orders/recheck', '/channels', '/sales/settings']),
    );

    expect(links).toEqual([
      { label: '/orders/review', path: '/orders/review' },
      { label: '/orders/recheck', path: '/orders/recheck' },
      { label: '/channels', path: '/channels' },
      { label: '/sales/settings', path: '/sales/settings' },
    ]);
  });
});
