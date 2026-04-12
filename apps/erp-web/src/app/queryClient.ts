import { MutationCache, QueryCache, QueryClient } from '@tanstack/react-query';

import { notifyRequestError } from '@/utils/requestError';

const handleError = (error: unknown) => {
  notifyRequestError(error);
};

export const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: handleError
  }),
  mutationCache: new MutationCache({
    onError: handleError
  }),
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false
    }
  }
});
