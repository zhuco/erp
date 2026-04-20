import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import { shouldHydrateWorkbenchSavedView } from './useWorkbenchSavedViewState';

describe('useWorkbenchSavedViewState', () => {
  it('同一个 storageKey 不会重复触发 hydrate', () => {
    expect(
      shouldHydrateWorkbenchSavedView({
        storageKey: 'orders.query.saved-view',
        hydratedStorageKey: 'orders.query.saved-view',
      }),
    ).toBe(false);
  });

  it('storageKey 变化后会重新触发 hydrate', () => {
    expect(
      shouldHydrateWorkbenchSavedView({
        storageKey: 'orders.query.saved-view.next',
        hydratedStorageKey: 'orders.query.saved-view.prev',
      }),
    ).toBe(true);
  });

  it('通过 ref 持有最新回调，并且 hydrate effect 只受 storageKey 变化驱动', () => {
    const source = readFileSync(resolve(__dirname, 'useWorkbenchSavedViewState.ts'), 'utf8');

    expect(source).toContain('const hydrateRef = useRef(hydrate);');
    expect(source).toContain('const serializeRef = useRef(serialize);');
    expect(source).toContain('const hydratedStorageKeyRef = useRef<string | null>(null);');
    expect(source).toContain('hydrateRef.current = hydrate;');
    expect(source).toContain('serializeRef.current = serialize;');
    expect(source).toContain('shouldHydrateWorkbenchSavedView({');
    expect(source).toContain('}, [storageKey]);');
  });
});
