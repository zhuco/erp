import { DatePicker, Input, InputNumber, Select } from 'antd';
import type { Dayjs } from 'dayjs';
import dayjs from 'dayjs';
import type { ReactNode } from 'react';

export type OrderQuickFilterOption = {
  label: ReactNode;
  value: string | number;
};

export type OrderQuickFilterField = {
  key: string;
  type?: 'auto' | 'input' | 'select' | 'multi-select' | 'date-range' | 'date-time-range' | 'number' | 'custom';
  value?: unknown;
  onChange?: (value: unknown) => void;
  onSearch?: (value: string) => void;
  onPaste?: (value: string) => void;
  options?: OrderQuickFilterOption[];
  testId?: string;
  placeholder?: string;
  rangePlaceholder?: [string, string];
  disabled?: boolean;
  allowClear?: boolean;
  multiple?: boolean;
  min?: number;
  max?: number;
  step?: number;
  precision?: number;
  stringMode?: boolean;
  node?: ReactNode;
};

const resolveTodayRange = () => {
  const start = dayjs().startOf('day').add(1, 'second');
  const end = dayjs().endOf('day').millisecond(0);
  return [start, end] as [Dayjs, Dayjs];
};

const resolveRangePlaceholder = (placeholder?: [string, string]) =>
  placeholder ?? ['开始时间', '结束时间'];

export const buildOrderQuickFilterCells = (fields: OrderQuickFilterField[]) => {
  const cells: Record<string, ReactNode> = {};
  fields.forEach((field) => {
    if (!field.key) {
      return;
    }
    if (field.type === 'custom' && field.node) {
      cells[field.key] = field.node;
      return;
    }
    const allowClear = field.allowClear !== false;
    const placeholder = field.placeholder ?? '';
    const disabled = field.disabled ?? false;
    const value = field.value;
    const options = field.options;
    const type = field.type ?? 'auto';
    const isMultiSelect =
      type === 'multi-select' || field.multiple === true || Array.isArray(value);

    if (type === 'number') {
      cells[field.key] = (
        <InputNumber
          size="small"
          data-testid={field.testId}
          min={field.min}
          max={field.max}
          step={field.step}
          precision={field.precision}
          stringMode={field.stringMode}
          placeholder={placeholder}
          style={{ width: '100%' }}
          value={value as string | number | null | undefined}
          onChange={(nextValue) => field.onChange?.(nextValue)}
          disabled={disabled}
        />
      );
      return;
    }

    if (type === 'date-range' || type === 'date-time-range') {
      const rangeValue = (value ?? undefined) as [Dayjs, Dayjs] | undefined;
      const handleOpenChange = (open: boolean) => {
        if (!open || !field.onChange) {
          return;
        }
        if (rangeValue?.[0] && rangeValue?.[1]) {
          return;
        }
        field.onChange(resolveTodayRange());
      };
      cells[field.key] = (
        <DatePicker.RangePicker
          size="small"
          data-testid={field.testId}
          allowClear={allowClear}
          showTime={type === 'date-time-range'}
          format={type === 'date-time-range' ? 'YYYY-MM-DD HH:mm:ss' : 'YYYY-MM-DD'}
          value={rangeValue}
          style={{ width: '100%' }}
          placeholder={resolveRangePlaceholder(field.rangePlaceholder)}
          onChange={(range) => field.onChange?.(range ?? undefined)}
          onOpenChange={handleOpenChange}
          disabled={disabled}
        />
      );
      return;
    }

    const shouldUseSelect = type === 'select' || type === 'multi-select' || Boolean(options?.length);
    if (shouldUseSelect) {
      cells[field.key] = (
        <Select
          size="small"
          data-testid={field.testId}
          allowClear={allowClear}
          mode={isMultiSelect ? 'multiple' : undefined}
          maxTagCount={isMultiSelect ? 'responsive' : undefined}
          placeholder={placeholder}
          options={options}
          value={value as string | number | Array<string | number> | undefined}
          onChange={(nextValue) => field.onChange?.(nextValue)}
          disabled={disabled}
          style={{ width: '100%' }}
        />
      );
      return;
    }

    cells[field.key] = (
      <Input
        size="small"
        data-testid={field.testId}
        allowClear={allowClear}
        placeholder={placeholder}
        value={(value as string) ?? ''}
        onChange={(event) => field.onChange?.(event.target.value)}
        onPressEnter={(event) => field.onSearch?.((event.target as HTMLInputElement).value)}
        onPaste={(event) => {
          if (!field.onPaste) {
            return;
          }
          const input = event.currentTarget;
          const pastedText = event.clipboardData?.getData('text') ?? '';
          if (!pastedText) {
            window.setTimeout(() => {
              field.onPaste?.(input.value);
            }, 0);
            return;
          }
          const selectionStart = input.selectionStart ?? input.value.length;
          const selectionEnd = input.selectionEnd ?? selectionStart;
          const nextValue = `${input.value.slice(0, selectionStart)}${pastedText}${input.value.slice(selectionEnd)}`;
          field.onPaste(nextValue);
        }}
        disabled={disabled}
        style={{ width: '100%' }}
      />
    );
  });
  return cells;
};
