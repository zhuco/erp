import { useEffect, useRef, useState } from 'react';

type UseWorkbenchSavedViewStateOptions<TState> = {
  storageKey?: string;
  hydrate: (state: TState) => void;
  serialize: () => TState;
};

export const shouldHydrateWorkbenchSavedView = ({
  storageKey,
  hydratedStorageKey,
}: {
  storageKey?: string;
  hydratedStorageKey: string | null;
}) => Boolean(storageKey) && storageKey !== hydratedStorageKey;

export const useWorkbenchSavedViewState = <TState extends object>({
  storageKey,
  hydrate,
  serialize,
}: UseWorkbenchSavedViewStateOptions<TState>) => {
  const [hydrated, setHydrated] = useState(false);
  const hydrateRef = useRef(hydrate);
  const serializeRef = useRef(serialize);
  const hydratedStorageKeyRef = useRef<string | null>(null);

  useEffect(() => {
    hydrateRef.current = hydrate;
  }, [hydrate]);

  useEffect(() => {
    serializeRef.current = serialize;
  }, [serialize]);

  useEffect(() => {
    if (!storageKey || typeof window === 'undefined') {
      hydratedStorageKeyRef.current = storageKey ?? null;
      setHydrated(true);
      return;
    }

    if (
      !shouldHydrateWorkbenchSavedView({
        storageKey,
        hydratedStorageKey: hydratedStorageKeyRef.current,
      })
    ) {
      setHydrated(true);
      return;
    }

    setHydrated(false);

    const raw = window.localStorage.getItem(storageKey);
    if (!raw) {
      hydratedStorageKeyRef.current = storageKey;
      setHydrated(true);
      return;
    }

    try {
      hydrateRef.current(JSON.parse(raw) as TState);
    } catch {
      // ignore invalid cache
    } finally {
      hydratedStorageKeyRef.current = storageKey;
      setHydrated(true);
    }
  }, [storageKey]);

  useEffect(() => {
    if (!storageKey || !hydrated || typeof window === 'undefined') {
      return;
    }

    window.localStorage.setItem(storageKey, JSON.stringify(serializeRef.current()));
  }, [hydrated, serialize, storageKey]);

  return hydrated;
};
