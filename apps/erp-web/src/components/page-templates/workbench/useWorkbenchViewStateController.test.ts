import { describe, expect, it } from 'vitest';

import {
  serializeWorkbenchViewState,
  splitWorkbenchViewState,
} from './useWorkbenchViewStateController';

describe('useWorkbenchViewStateController helpers', () => {
  it('hydrates selection fields alongside configured filter payloads', () => {
    const payload = splitWorkbenchViewState(
      {
        keywordInput: '异常单',
        quickFilterExpanded: true,
        selectedRowKeys: ['row-1', 'row-1', 'row-2'],
        focusedRowKey: 'row-2',
        activeRowKey: 'row-1',
      },
      ['keywordInput', 'quickFilterExpanded'],
    );

    expect(payload.extraState).toEqual({
      keywordInput: '异常单',
      quickFilterExpanded: true,
    });
    expect(payload.selectionState).toEqual({
      selectedRowKeys: ['row-1', 'row-2'],
      focusedRowKey: 'row-2',
      activeRowKey: 'row-1',
    });
  });

  it('serializes only configured extra fields plus the standard workbench selection payload', () => {
    const payload = serializeWorkbenchViewState({
      extraState: {
        keywordInput: '待复核',
        quickFilterExpanded: false,
        ignoredField: 'should-not-persist',
      },
      extraStateKeys: ['keywordInput', 'quickFilterExpanded'],
      selectionState: {
        selectedRowKeys: ['row-2', 'row-2', 'row-3'],
        focusedRowKey: 'row-3',
        activeRowKey: 'row-2',
        selectionAnchorKey: 'row-2',
      },
    });

    expect(payload).toEqual({
      keywordInput: '待复核',
      quickFilterExpanded: false,
      selectedRowKeys: ['row-2', 'row-3'],
      focusedRowKey: 'row-3',
      activeRowKey: 'row-2',
      selectionAnchorKey: 'row-2',
    });
    expect(payload).not.toHaveProperty('ignoredField');
  });
  it('skips selection serialization when the page keeps selection transient', () => {
    const payload = serializeWorkbenchViewState({
      extraState: {
        keywordInput: 'query',
        quickFilterExpanded: true,
      },
      extraStateKeys: ['keywordInput', 'quickFilterExpanded'],
      selectionState: {
        selectedRowKeys: ['row-1', 'row-2'],
        focusedRowKey: 'row-2',
        activeRowKey: 'row-1',
        selectionAnchorKey: 'row-1',
      },
      persistSelectionState: false,
    });

    expect(payload).toEqual({
      keywordInput: 'query',
      quickFilterExpanded: true,
    });
    expect(payload).not.toHaveProperty('selectedRowKeys');
    expect(payload).not.toHaveProperty('focusedRowKey');
  });

  it('ignores hydrated selection fields when selection persistence is disabled', () => {
    const payload = splitWorkbenchViewState(
      {
        keywordInput: 'query',
        selectedRowKeys: ['row-1', 'row-2'],
        focusedRowKey: 'row-2',
        activeRowKey: 'row-1',
        selectionAnchorKey: 'row-1',
      },
      ['keywordInput'],
      false,
    );

    expect(payload.extraState).toEqual({
      keywordInput: 'query',
    });
    expect(payload.selectionState).toEqual({});
  });
});
