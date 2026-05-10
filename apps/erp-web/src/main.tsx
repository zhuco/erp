import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { RouterProvider } from 'react-router-dom';

import { AppProviders } from '@/app/AppProviders';
import { router } from '@/app/router';
import { installAgGridLicenseLogFilter } from '@/utils/suppressAgGridLicenseLogs';
import { apiBaseUrl, buildVersion } from '@/utils/env';
import '@/styles/theme.css';
import 'ag-grid-community/styles/ag-grid.css';
import 'ag-grid-community/styles/ag-theme-quartz.css';

installAgGridLicenseLogFilter();

// Log version info to console for debugging
console.log(`%c衡策云 Web UI v${buildVersion}`, 'font-weight: bold; font-size: 14px; color: #1f3a5f');
console.log(`API Base URL: ${apiBaseUrl}`);
console.log(`Build Time: ${new Date().toISOString()}`);

const container = document.getElementById('root');

if (!container) {
  throw new Error('Root container missing in index.html');
}

createRoot(container).render(
  <StrictMode>
    <AppProviders>
      <RouterProvider router={router} future={{ v7_startTransition: true }} />
    </AppProviders>
  </StrictMode>
);
