import { Button, Checkbox, DatePicker, Dropdown, Space } from 'antd';
import dayjs from 'dayjs';
import { useEffect, useMemo, useState } from 'react';
import type { DateFilterModel } from 'ag-grid-community';
import type { CustomFloatingFilterProps } from 'ag-grid-react';

export type IndustrialWorkbenchFilterOption = {
  label: string;
  value: string;
};

type SetFilterModel = {
  filterType?: 'set';
  values?: string[] | null;
} | null;

type TextFilterModel = {
  filterType?: 'text';
  type?: string | null;
  filter?: string | null;
} | null;

type IndustrialWorkbenchSetFloatingFilterProps = CustomFloatingFilterProps & {
  emptyLabel?: string;
  options?: IndustrialWorkbenchFilterOption[];
  testId: string;
};

type IndustrialWorkbenchTextFloatingFilterProps = CustomFloatingFilterProps & {
  placeholder?: string;
  testId: string;
};

type IndustrialWorkbenchDateRangeFloatingFilterProps = CustomFloatingFilterProps & {
  triggerTestId: string;
};

const normalizeKeyword = (value: string | undefined) => value?.trim() ?? '';
const normalizeDateInput = (value: string | undefined) => value?.trim() ?? '';

const normalizeSetValues = (values: string[] | null | undefined) =>
  Array.from(
    new Set(
      (values ?? [])
        .map((value) => String(value ?? '').trim())
        .filter(Boolean),
    ),
  );

const readSetValues = (model: unknown) => {
  if (!model || typeof model !== 'object' || !('values' in model)) {
    return [] as string[];
  }
  const values = (model as { values?: unknown }).values;
  return Array.isArray(values) ? normalizeSetValues(values as string[]) : [];
};

const readTextValue = (model: unknown) => {
  if (!model || typeof model !== 'object' || !('filter' in model)) {
    return '';
  }
  return normalizeKeyword(String((model as { filter?: unknown }).filter ?? ''));
};

const readDateValue = (model: unknown, key: 'dateFrom' | 'dateTo') => {
  if (!model || typeof model !== 'object' || !(key in model)) {
    return '';
  }
  const raw = String((model as Record<'dateFrom' | 'dateTo', unknown>)[key] ?? '').trim();
  return raw.slice(0, 10);
};

const formatDateWithoutYear = (value: string) => {
  const normalized = normalizeDateInput(value);
  if (!normalized) {
    return '';
  }
  const parsed = dayjs(normalized, 'YYYY-MM-DD', true);
  if (!parsed.isValid()) {
    return normalized.slice(5);
  }
  return parsed.format('MM-DD');
};

const orderDateRange = (from: string, to: string) => {
  const normalizedFrom = normalizeDateInput(from);
  const normalizedTo = normalizeDateInput(to);
  if (!normalizedFrom || !normalizedTo) {
    return [normalizedFrom, normalizedTo] as const;
  }

  const fromDate = dayjs(normalizedFrom, 'YYYY-MM-DD', true);
  const toDate = dayjs(normalizedTo, 'YYYY-MM-DD', true);
  if (!fromDate.isValid() || !toDate.isValid()) {
    return [normalizedFrom, normalizedTo] as const;
  }

  return fromDate.isAfter(toDate)
    ? ([normalizedTo, normalizedFrom] as const)
    : ([normalizedFrom, normalizedTo] as const);
};

const normalizeSetModel = (values: string[] | null | undefined): SetFilterModel => {
  const normalizedValues = normalizeSetValues(values);
  return normalizedValues.length > 0 ? { filterType: 'set', values: normalizedValues } : null;
};

const normalizeTextModel = (value: string): TextFilterModel =>
  value ? { filterType: 'text', type: 'contains', filter: value } : null;

const normalizeDateRangeModel = (from: string, to: string): DateFilterModel | null => {
  const [normalizedFrom, normalizedTo] = orderDateRange(from, to);
  if (!normalizedFrom && !normalizedTo) {
    return null;
  }
  if (normalizedFrom && normalizedTo) {
    return {
      filterType: 'date',
      type: 'inRange',
      dateFrom: `${normalizedFrom} 00:00:00`,
      dateTo: `${normalizedTo} 23:59:59`,
    };
  }
  if (normalizedFrom) {
    return {
      filterType: 'date',
      type: 'greaterThanOrEqual',
      dateFrom: `${normalizedFrom} 00:00:00`,
    };
  }
  return {
    filterType: 'date',
    type: 'lessThanOrEqual',
    dateTo: `${normalizedTo} 23:59:59`,
  };
};

export const IndustrialWorkbenchSetFloatingFilter = ({
  model,
  onModelChange,
  options = [],
  emptyLabel = '全部',
  testId,
}: IndustrialWorkbenchSetFloatingFilterProps) => {
  const selectedValues = readSetValues(model);

  const normalizedOptions = useMemo(
    () =>
      Array.from(
        new Map(
          options
            .map((option) => ({
              label: String(option.label ?? '').trim(),
              value: String(option.value ?? '').trim(),
            }))
            .filter((option) => option.value)
            .map((option) => [option.value, option]),
        ).values(),
      ),
    [options],
  );
  const selectedLabelMap = useMemo(
    () => new Map(normalizedOptions.map((option) => [option.value, option.label || option.value])),
    [normalizedOptions],
  );
  const selectedLabelText =
    selectedValues.length === 0
      ? emptyLabel
      : selectedValues.length === 1
        ? selectedLabelMap.get(selectedValues[0]!) ?? selectedValues[0]!
        : `${selectedValues.length}项`;
  const allChecked = normalizedOptions.length > 0 && selectedValues.length === normalizedOptions.length;
  const indeterminate = selectedValues.length > 0 && !allChecked;

  const updateValues = (nextValues: string[]) => {
    onModelChange(normalizeSetModel(nextValues));
  };

  return (
    <div
      className="order-review-floating-filter-select-shell"
      onClick={(event) => event.stopPropagation()}
      onMouseDown={(event) => event.stopPropagation()}
    >
      <Dropdown
        trigger={['click']}
        popupRender={() => (
          <div
            className="order-review-floating-filter-dropdown"
            onClick={(event) => event.stopPropagation()}
            onMouseDown={(event) => event.stopPropagation()}
          >
            <Checkbox
              indeterminate={indeterminate}
              checked={allChecked}
              onChange={(event) => {
                updateValues(event.target.checked ? normalizedOptions.map((option) => option.value) : []);
              }}
            >
              全部
            </Checkbox>
            <Checkbox.Group
              className="order-review-floating-filter-checkbox-group"
              value={selectedValues}
              onChange={(values) => {
                updateValues(values.map((value) => String(value)));
              }}
            >
              <Space orientation="vertical" size={4}>
                {normalizedOptions.map((option) => (
                  <Checkbox key={option.value} value={option.value}>
                    {option.label || option.value}
                  </Checkbox>
                ))}
              </Space>
            </Checkbox.Group>
          </div>
        )}
      >
        <Button
          size="small"
          className="order-review-floating-filter-select"
          data-testid={testId}
          aria-label={testId}
        >
          {selectedLabelText}
        </Button>
      </Dropdown>
    </div>
  );
};

export const IndustrialWorkbenchTextFloatingFilter = ({
  model,
  onModelChange,
  placeholder,
  testId,
}: IndustrialWorkbenchTextFloatingFilterProps) => {
  const committedValue = readTextValue(model);
  const [draftValue, setDraftValue] = useState(committedValue);

  useEffect(() => {
    setDraftValue(committedValue);
  }, [committedValue]);

  const applyDraftValue = (rawValue: string) => {
    const normalizedValue = normalizeKeyword(rawValue);
    onModelChange(normalizeTextModel(normalizedValue));
  };

  return (
    <div
      className="order-review-floating-filter-text-shell"
      onClick={(event) => event.stopPropagation()}
      onMouseDown={(event) => event.stopPropagation()}
    >
      <input
        className="order-review-floating-filter-text-input"
        data-testid={testId}
        type="text"
        aria-label={testId}
        value={draftValue}
        placeholder={placeholder}
        onChange={(event) => {
          setDraftValue(String(event.target.value ?? ''));
        }}
        onKeyDown={(event) => {
          if (event.key !== 'Enter') {
            return;
          }
          event.preventDefault();
          event.stopPropagation();
          applyDraftValue((event.currentTarget as HTMLInputElement).value);
        }}
        onPaste={(event) => {
          const pastedText = event.clipboardData?.getData('text') ?? '';
          if (!pastedText) {
            const input = event.currentTarget;
            window.setTimeout(() => {
              applyDraftValue(input.value);
              setDraftValue(input.value);
            }, 0);
            return;
          }
          const input = event.currentTarget;
          const selectionStart = input.selectionStart ?? input.value.length;
          const selectionEnd = input.selectionEnd ?? selectionStart;
          const nextValue = `${input.value.slice(0, selectionStart)}${pastedText}${input.value.slice(selectionEnd)}`;
          setDraftValue(nextValue);
          applyDraftValue(nextValue);
        }}
      />
    </div>
  );
};

export const IndustrialWorkbenchDateRangeFloatingFilter = ({
  model,
  onModelChange,
  triggerTestId,
}: IndustrialWorkbenchDateRangeFloatingFilterProps) => {
  const fromValue = readDateValue(model, 'dateFrom');
  const toValue = readDateValue(model, 'dateTo');
  const [pickerOpen, setPickerOpen] = useState(false);
  const pickerValue =
    fromValue && toValue ? ([dayjs(fromValue, 'YYYY-MM-DD', true), dayjs(toValue, 'YYYY-MM-DD', true)] as const) : null;
  const displayFromValue = formatDateWithoutYear(fromValue);
  const displayToValue = formatDateWithoutYear(toValue);
  const canClear = Boolean(fromValue || toValue);

  const triggerLabel =
    displayFromValue && displayToValue
      ? `${displayFromValue} ~ ${displayToValue}`
      : displayFromValue
        ? `${displayFromValue} 起`
        : displayToValue
          ? `${displayToValue} 止`
          : '日期范围';

  const triggerTitle =
    fromValue && toValue
      ? `${fromValue} ~ ${toValue}`
      : fromValue
        ? `${fromValue} 起`
        : toValue
          ? `${toValue} 止`
          : '日期范围';

  return (
    <div
      className="order-review-floating-filter-date-shell"
      onClick={(event) => event.stopPropagation()}
      onMouseDown={(event) => event.stopPropagation()}
    >
      <button
        type="button"
        className="order-review-floating-filter-date-trigger"
        data-testid={triggerTestId}
        aria-label={triggerTitle}
        title={triggerTitle}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          setPickerOpen(true);
        }}
      >
        <span className="order-review-floating-filter-date-trigger-text">{triggerLabel}</span>
      </button>
      <button
        type="button"
        className="order-review-floating-filter-date-clear"
        aria-label="清空日期范围"
        disabled={!canClear}
        onClick={(event) => {
          event.preventDefault();
          event.stopPropagation();
          if (!canClear) {
            return;
          }
          onModelChange(null);
        }}
      >
        清
      </button>
      {pickerOpen ? (
        <div className="order-review-floating-filter-date-picker-anchor">
        <DatePicker.RangePicker
          open={pickerOpen}
          value={pickerValue}
          format="YYYY-MM-DD"
          order
          allowClear={false}
          inputReadOnly
          placement="bottomLeft"
          onOpenChange={(open) => {
            setPickerOpen(open);
          }}
          onChange={(range) => {
            if (!range?.[0] || !range[1]) {
              onModelChange(null);
              setPickerOpen(false);
              return;
            }
            onModelChange(
              normalizeDateRangeModel(range[0].format('YYYY-MM-DD'), range[1].format('YYYY-MM-DD')),
            );
            setPickerOpen(false);
          }}
        />
        </div>
      ) : null}
    </div>
  );
};
