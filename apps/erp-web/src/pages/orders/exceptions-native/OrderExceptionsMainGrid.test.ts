import { describe, expect, it, vi } from 'vitest';

import { buildOrderExceptionsMainGridColDefs } from './OrderExceptionsMainGrid';

describe('OrderExceptionsMainGrid column config', () => {
  it('adds a standardized mark column for exception workbench rows', () => {
    const colDefs = buildOrderExceptionsMainGridColDefs({
      platformOptions: [],
      shopOptions: [],
      warehouseOptions: [],
      auditStatusOptions: [],
      exceptionStatusOptions: [],
      slaStatusOptions: [],
      assigneeOptions: [],
      onToggleAllSelection: vi.fn(),
      resolveAuditLabel: (value) => value ?? '-',
      resolveExceptionLabel: (value) => value ?? '-',
    });

    const colDefMap = new Map(colDefs.map((colDef) => [String(colDef.colId ?? ''), colDef]));

    expect(colDefMap.get('builtin_marks')).toBeDefined();
    expect(colDefMap.get('builtin_marks')?.headerName).toBe('记号');
  });
});
