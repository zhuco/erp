import { useEffect, useState } from 'react';

type UseWorkbenchSavedViewStateOptions<TState> = {
  storageKey?: string;
  hydrate: (state: TState) => void;
  serialize: () => TState;
};

export const useWorkbenchSavedViewState = <TState extends object>({
  storageKey,
  hydrate,
  serialize,
}: UseWorkbenchSavedViewStateOptions<TState>) => {
  const [hydrated, setHydrated] = useState(false);

  useEffect(() => {
    setHydrated(false);
    if (!storageKey || typeof window === 'undefined') {
      setHydrated(true);
      return;
    }

    const raw = window.localStorage.getItem(storageKey);
    if (!raw) {
      setHydrated(true);
      return;
    }

    try {
      hydrate(JSON.parse(raw) as TState);
    } catch {
      // ignore invalid cache
    } finally {
      setHydrated(true);
    }
  }, [hydrate, storageKey]);

  useEffect(() => {
    if (!storageKey || !hydrated || typeof window === 'undefined') {
      return;
    }

    window.localStorage.setItem(storageKey, JSON.stringify(serialize()));
  }, [hydrated, serialize, storageKey]);

  return hydrated;
};
