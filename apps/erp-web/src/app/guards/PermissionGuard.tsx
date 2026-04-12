import type { ReactNode } from 'react';

import { ForbiddenPage } from '@/pages/Forbidden';
import type { Permission } from '@/features/rbac';
import { useCan } from '@/features/rbac';

type PermissionGuardProps = {
  permission: Permission;
  children: ReactNode;
};

export const PermissionGuard = ({ permission, children }: PermissionGuardProps) => {
  const allowed = useCan(permission);
  if (!allowed) {
    return <ForbiddenPage />;
  }
  return <>{children}</>;
};
