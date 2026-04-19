import { afterEach, describe, expect, it, vi } from 'vitest';

describe('useLeftFilterModule left panel width override', () => {
  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    vi.doUnmock('react');
    vi.doUnmock('@/app/layout/workbenchPanels');
  });

  const setupModule = async () => {
    const setLeftPanel = vi.fn();
    const setLeftPanelVisible = vi.fn();
    const setLeftPanelWidth = vi.fn();
    const setLeftPanelCollapsed = vi.fn();

    vi.doMock('react', async () => {
      const actual = await vi.importActual<typeof import('react')>('react');
      return {
        ...actual,
        useState: (initial: unknown) => [
          typeof initial === 'function' ? (initial as () => unknown)() : initial,
          vi.fn(),
        ],
        useMemo: <T,>(factory: () => T) => factory(),
        useCallback: <T,>(callback: T) => callback,
        useEffect: (effect: () => void | (() => void)) => {
          effect();
        },
      };
    });

    vi.doMock('@/app/layout/workbenchPanels', () => ({
      useWorkbenchPanels: () => ({
        setLeftPanel,
        setLeftPanelVisible,
        setLeftPanelWidth,
        setLeftPanelCollapsed,
      }),
    }));

    const module = await import('./useLeftFilterModule');
    return {
      useLeftFilterModule: module.useLeftFilterModule,
      setLeftPanelWidth,
    };
  };

  it('未显式传入左侧宽度时不会覆盖用户已调整的宽度', async () => {
    const { useLeftFilterModule, setLeftPanelWidth } = await setupModule();

    useLeftFilterModule({
      rows: [{ id: 'row-1' }],
      defaultFilters: { status: 'all' },
      initialExpandedKeys: [],
      hasManualFilter: () => false,
      filterRows: (rows) => rows,
      buildCheckedKeys: () => new Set<string>(),
      buildLeftNodes: () => [],
      reduceFiltersByNode: (previous) => previous,
      renderPanel: () => null,
    });

    expect(setLeftPanelWidth).not.toHaveBeenCalled();
  });

  it('显式传入左侧宽度时仍然会同步到工作区布局', async () => {
    const { useLeftFilterModule, setLeftPanelWidth } = await setupModule();

    useLeftFilterModule({
      rows: [{ id: 'row-1' }],
      defaultFilters: { status: 'all' },
      initialExpandedKeys: [],
      leftPanelWidth: 320,
      hasManualFilter: () => false,
      filterRows: (rows) => rows,
      buildCheckedKeys: () => new Set<string>(),
      buildLeftNodes: () => [],
      reduceFiltersByNode: (previous) => previous,
      renderPanel: () => null,
    });

    expect(setLeftPanelWidth).toHaveBeenCalledTimes(1);
    expect(setLeftPanelWidth).toHaveBeenCalledWith(320);
  });
});
