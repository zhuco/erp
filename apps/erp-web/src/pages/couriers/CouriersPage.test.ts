import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('CouriersPage source contract', () => {
  it('uses the unified full workbench shell instead of a page-level filter form', () => {
    const source = readFileSync(resolve(__dirname, './CouriersPage.tsx'), 'utf8');

    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('<WorkbenchToolbar');
    expect(source).toContain('leftPanel={leftPanel}');
    expect(source).toContain('rightPanel={rightPanel}');
    expect(source).toContain('bottomPanel={bottomPanel}');
    expect(source).toContain('leftPanelVisible={true}');
    expect(source).toContain('rightPanelVisible={true}');
    expect(source).toContain('bottomPanelVisible={true}');
    expect(source).toContain('showSelectionColumn');
    expect(source).toContain('selectedRowKeys={selectedCourierIds}');
    expect(source).toContain('activeRowKey={activeCourierId}');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).not.toContain('<Switch');
    expect(source).not.toContain('header={header}');
    expect(source).not.toContain('Typography.Title level={4}');
    expect(source).not.toContain('Form form={filterForm}');
  });
});
