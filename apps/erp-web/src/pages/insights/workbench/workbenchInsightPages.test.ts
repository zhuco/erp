import { describe, expect, it } from 'vitest';

import { ExportQueuePage } from './ExportQueuePage';
import { ReportCenterPage } from './ReportCenterPage';

describe('workbench insight pages', () => {
  it('exports canonical workbench wrappers with stable test ids', () => {
    expect(ReportCenterPage().props['data-testid']).toBe('insights-workbench-report-center-page');
    expect(ExportQueuePage().props['data-testid']).toBe('insights-workbench-export-queue-page');
  });
});
