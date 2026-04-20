export type WorkbenchViewStatePagination = {
  current?: number;
  pageSize?: number;
};

export type WorkbenchViewStateScroll = {
  scrollTop?: number;
  scrollLeft?: number;
};

export type WorkbenchViewStateCache = {
  pagination?: WorkbenchViewStatePagination;
  scroll?: WorkbenchViewStateScroll;
};

const normalizeFiniteNumber = (value: unknown) => {
  const nextValue = Number(value);
  return Number.isFinite(nextValue) && nextValue >= 0 ? nextValue : undefined;
};

const normalizePositiveNumber = (value: unknown) => {
  const nextValue = Number(value);
  return Number.isFinite(nextValue) && nextValue > 0 ? nextValue : undefined;
};

export const normalizeWorkbenchViewStateCache = (value: unknown): WorkbenchViewStateCache => {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    return {};
  }

  const source = value as {
    pagination?: { current?: unknown; pageSize?: unknown };
    scroll?: { scrollTop?: unknown; scrollLeft?: unknown };
  };

  const current = normalizePositiveNumber(source.pagination?.current);
  const pageSize = normalizePositiveNumber(source.pagination?.pageSize);
  const scrollTop = normalizeFiniteNumber(source.scroll?.scrollTop);
  const scrollLeft = normalizeFiniteNumber(source.scroll?.scrollLeft);

  return {
    pagination:
      current != null || pageSize != null
        ? {
            current,
            pageSize,
          }
        : undefined,
    scroll:
      scrollTop != null || scrollLeft != null
        ? {
            scrollTop,
            scrollLeft,
          }
        : undefined,
  };
};

export const readWorkbenchViewStateCache = (storageKey?: string): WorkbenchViewStateCache => {
  if (!storageKey || typeof window === 'undefined') {
    return {};
  }

  try {
    const raw = window.localStorage.getItem(storageKey);
    if (!raw) {
      return {};
    }
    return normalizeWorkbenchViewStateCache(JSON.parse(raw));
  } catch {
    return {};
  }
};

export const writeWorkbenchViewStateCache = (
  storageKey: string | undefined,
  partialState: Partial<WorkbenchViewStateCache>,
) => {
  if (!storageKey || typeof window === 'undefined') {
    return;
  }

  const currentState = readWorkbenchViewStateCache(storageKey);
  const nextState = normalizeWorkbenchViewStateCache({
    ...currentState,
    ...partialState,
    pagination:
      partialState.pagination === undefined
        ? currentState.pagination
        : {
            ...currentState.pagination,
            ...partialState.pagination,
          },
    scroll:
      partialState.scroll === undefined
        ? currentState.scroll
        : {
            ...currentState.scroll,
            ...partialState.scroll,
          },
  });

  try {
    window.localStorage.setItem(storageKey, JSON.stringify(nextState));
  } catch {
    // ignore write failures
  }
};
