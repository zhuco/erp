import { Navigate, Outlet, useLocation } from 'react-router-dom';

import { useSessionStore } from '@/features/sessionStore';

export const AuthGuard = () => {
  const token = useSessionStore((state) => state.token);
  const location = useLocation();

  if (!token) {
    const from = `${location.pathname}${location.search}`;
    return <Navigate to="/login" replace state={{ from }} />;
  }

  return <Outlet />;
};
