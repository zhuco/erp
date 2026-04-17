import { useEffect, useRef } from 'react';

type UseFocusReturnOptions = {
  enabled?: boolean;
  fallback?: HTMLElement | null;
};

const resolveFocusableTarget = (target: HTMLElement | null | undefined) => {
  if (!target || !target.isConnected) {
    return null;
  }
  return target;
};

export const useFocusReturn = (open: boolean, options?: UseFocusReturnOptions) => {
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);
  const prevOpenRef = useRef(open);
  const enabled = options?.enabled ?? true;

  useEffect(() => {
    if (!enabled) {
      prevOpenRef.current = open;
      return;
    }

    const wasOpen = prevOpenRef.current;
    if (open && !wasOpen) {
      const activeElement = document.activeElement;
      previouslyFocusedRef.current =
        activeElement instanceof HTMLElement ? activeElement : null;
      prevOpenRef.current = open;
      return;
    }

    if (!open && wasOpen) {
      const focusTarget =
        resolveFocusableTarget(previouslyFocusedRef.current) ??
        resolveFocusableTarget(options?.fallback);
      if (focusTarget) {
        window.requestAnimationFrame(() => {
          focusTarget.focus({ preventScroll: true });
        });
      }
    }

    prevOpenRef.current = open;
  }, [enabled, open, options?.fallback]);
};
