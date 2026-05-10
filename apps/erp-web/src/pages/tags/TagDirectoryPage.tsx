import { useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';

import { useCan } from '@/features/rbac';

import { TagDirectoryWorkbench, type TagDirectoryTab } from './TagDirectoryWorkbench';

const resolveFallbackTab = (canReadOrders: boolean, canReadSkus: boolean): TagDirectoryTab => {
  if (canReadOrders) {
    return 'orders';
  }
  if (canReadSkus) {
    return 'skus';
  }
  return 'builtin';
};

export const TagDirectoryPage = () => {
  const canReadOrders = useCan('order_tags.read');
  const canReadSkus = useCan('sku_tags.read');
  const [searchParams, setSearchParams] = useSearchParams();

  const activeTab = useMemo<TagDirectoryTab>(() => {
    const raw = searchParams.get('tab');
    if (raw === 'orders' && canReadOrders) {
      return 'orders';
    }
    if (raw === 'skus' && canReadSkus) {
      return 'skus';
    }
    if (raw === 'builtin') {
      return 'builtin';
    }
    return resolveFallbackTab(canReadOrders, canReadSkus);
  }, [canReadOrders, canReadSkus, searchParams]);

  return (
    <TagDirectoryWorkbench
      activeTab={activeTab}
      onTabChange={(tab) => {
        const next = new URLSearchParams(searchParams);
        next.set('tab', tab);
        setSearchParams(next, { replace: true });
      }}
    />
  );
};
