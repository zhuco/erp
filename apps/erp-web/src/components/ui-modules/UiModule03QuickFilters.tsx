import { Form, message } from 'antd';
import type { SelectProps } from 'antd';
import type { DragEvent, HTMLAttributes, ReactElement } from 'react';
import { useCallback, useEffect, useMemo, useState } from 'react';

import {
  UiModule01QuickFiltersPanel,
  UiModule01QuickFiltersSettingsModal,
  type UiModule01QuickFilterFormValues
} from './UiModule01QuickFilters';

export type UiModule03QuickFilterRule = {
  platform?: string | string[];
  shopId?: string | string[];
  shipWarehouseId?: string | string[];
  reviewStatus?: string;
  tagged?: boolean;
  exceptionFlags?: string[];
  omsStatus?: string | string[];
};

export type UiModule03QuickFilterConfig = {
  id: string;
  name: string;
  rule: UiModule03QuickFilterRule;
  builtIn?: boolean;
};

type UiModule03QuickFilterStore = {
  order?: string[];
  filters?: UiModule03QuickFilterConfig[];
  disabled?: string[];
};

export type UiModule03QuickFiltersOptions = {
  storageKey: string;
  builtInFilters: UiModule03QuickFilterConfig[];
  defaultRule: UiModule03QuickFilterRule;
  initialActiveFilterId?: string;
  nameMaxLength?: number;
  labelMaxLength?: number;
  defaultFormValues?: Partial<UiModule01QuickFilterFormValues>;
  modalRender?: (modal: ReactElement) => ReactElement;
  titleProps?: HTMLAttributes<HTMLSpanElement>;
  reviewStatusOptions?: SelectProps['options'];
  platformOptions?: SelectProps['options'];
  shopOptions?: SelectProps['options'];
  warehouseOptions?: SelectProps['options'];
  orderStatusOptions?: SelectProps['options'];
  taggedOptions?: SelectProps['options'];
  exceptionFlagOptions?: SelectProps['options'];
  panelTestId?: string;
  settingsButtonTestId?: string;
  filterButtonTestIdPrefix?: string;
};

export type UiModule03QuickFiltersResult = {
  activeRule: UiModule03QuickFilterRule;
  activeFilterId: string;
  quickFilterPanel: ReactElement;
  quickFilterSettingsModal: ReactElement;
};

const buildQuickFilterId = () => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `qf-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

const normalizeMultiValue = <T extends string>(value?: T[]) => {
  if (!value) {
    return undefined;
  }
  const filtered = value.filter((item) => item);
  return filtered.length ? filtered : undefined;
};

export const useUiModule03QuickFilters = ({
  storageKey,
  builtInFilters,
  defaultRule,
  initialActiveFilterId = 'all',
  nameMaxLength = 8,
  labelMaxLength,
  defaultFormValues,
  modalRender,
  titleProps,
  reviewStatusOptions,
  platformOptions,
  shopOptions,
  warehouseOptions,
  orderStatusOptions,
  taggedOptions,
  exceptionFlagOptions,
  panelTestId,
  settingsButtonTestId,
  filterButtonTestIdPrefix
}: UiModule03QuickFiltersOptions): UiModule03QuickFiltersResult => {
  const [quickFilters, setQuickFilters] = useState<UiModule03QuickFilterConfig[]>([]);
  const [quickFilterOrder, setQuickFilterOrder] = useState<string[]>([]);
  const [disabledQuickFilters, setDisabledQuickFilters] = useState<string[]>([]);
  const [activeQuickFilterId, setActiveQuickFilterId] = useState(initialActiveFilterId);
  const [quickFilterModalOpen, setQuickFilterModalOpen] = useState(false);
  const [draggingFilterId, setDraggingFilterId] = useState<string | null>(null);
  const [quickFilterForm] = Form.useForm<UiModule01QuickFilterFormValues>();

  const resolvedLabelMaxLength = labelMaxLength ?? nameMaxLength;
  const normalizeDisabledFilters = useCallback(
    (disabled?: string[]) =>
      Array.isArray(disabled)
        ? disabled.filter((filterId) => filterId !== initialActiveFilterId)
        : [],
    [initialActiveFilterId]
  );

  const combinedQuickFilters = useMemo(
    () => [...builtInFilters, ...quickFilters],
    [builtInFilters, quickFilters]
  );

  const orderedQuickFilters = useMemo(() => {
    if (!quickFilterOrder.length) {
      return combinedQuickFilters;
    }
    const map = new Map(combinedQuickFilters.map((item) => [item.id, item]));
    const ordered = quickFilterOrder
      .map((id) => map.get(id))
      .filter((item): item is UiModule03QuickFilterConfig => Boolean(item));
    const remaining = combinedQuickFilters.filter((item) => !quickFilterOrder.includes(item.id));
    return [...ordered, ...remaining];
  }, [combinedQuickFilters, quickFilterOrder]);

  const visibleQuickFilters = useMemo(
    () => orderedQuickFilters.filter((item) => !disabledQuickFilters.includes(item.id)),
    [disabledQuickFilters, orderedQuickFilters]
  );

  const activeQuickFilter = useMemo(
    () => visibleQuickFilters.find((item) => item.id === activeQuickFilterId),
    [activeQuickFilterId, visibleQuickFilters]
  );

  const activeRule = activeQuickFilter?.rule ?? defaultRule;

  useEffect(() => {
    if (!visibleQuickFilters.length) {
      return;
    }
    if (!visibleQuickFilters.some((item) => item.id === activeQuickFilterId)) {
      setActiveQuickFilterId(visibleQuickFilters[0].id);
    }
  }, [activeQuickFilterId, visibleQuickFilters]);

  useEffect(() => {
    if (!storageKey) {
      setQuickFilters([]);
      setQuickFilterOrder([]);
      setDisabledQuickFilters([]);
      setActiveQuickFilterId(initialActiveFilterId);
      return;
    }
    const raw = localStorage.getItem(storageKey);
    if (!raw) {
      setQuickFilters([]);
      setQuickFilterOrder([]);
      setDisabledQuickFilters([]);
      setActiveQuickFilterId(initialActiveFilterId);
      return;
    }
    try {
      const parsed = JSON.parse(raw) as UiModule03QuickFilterStore;
      setQuickFilters(Array.isArray(parsed.filters) ? parsed.filters : []);
      setQuickFilterOrder(Array.isArray(parsed.order) ? parsed.order : []);
      setDisabledQuickFilters(normalizeDisabledFilters(parsed.disabled));
      setActiveQuickFilterId(initialActiveFilterId);
    } catch {
      setQuickFilters([]);
      setQuickFilterOrder([]);
      setDisabledQuickFilters([]);
      setActiveQuickFilterId(initialActiveFilterId);
    }
  }, [initialActiveFilterId, normalizeDisabledFilters, storageKey]);

  useEffect(() => {
    if (!storageKey) {
      return;
    }
    localStorage.setItem(
      storageKey,
      JSON.stringify({
        filters: quickFilters,
        order: quickFilterOrder,
        disabled: normalizeDisabledFilters(disabledQuickFilters)
      })
    );
  }, [
    disabledQuickFilters,
    normalizeDisabledFilters,
    quickFilterOrder,
    quickFilters,
    storageKey
  ]);

  const resetQuickFilterForm = useCallback(() => {
    quickFilterForm.resetFields();
    if (defaultFormValues) {
      quickFilterForm.setFieldsValue(defaultFormValues);
    }
  }, [defaultFormValues, quickFilterForm]);

  const handleOpenQuickFilterModal = useCallback(() => {
    resetQuickFilterForm();
    setQuickFilterModalOpen(true);
  }, [resetQuickFilterForm]);

  const handleCreateQuickFilter = useCallback(async () => {
    try {
      const values = await quickFilterForm.validateFields();
      const trimmedName = values.name.trim();
      const nameLength = Array.from(trimmedName).length;
      if (!trimmedName || nameLength > nameMaxLength) {
        message.warning(`名称最多 ${nameMaxLength} 个字符`);
        return;
      }
      const newFilter: UiModule03QuickFilterConfig = {
        id: buildQuickFilterId(),
        name: trimmedName,
        rule: {
          platform: normalizeMultiValue(values.platform),
          shopId: normalizeMultiValue(values.shopId),
          shipWarehouseId: normalizeMultiValue(values.shipWarehouseId),
          reviewStatus: values.reviewStatus ?? 'PENDING',
          tagged: values.tagged,
          exceptionFlags: values.exceptionFlags?.length ? values.exceptionFlags : undefined,
          omsStatus: normalizeMultiValue(values.omsStatus)
        }
      };
      setQuickFilters((prev) => [...prev, newFilter]);
      setQuickFilterOrder((prev) => [...prev, newFilter.id]);
      setActiveQuickFilterId(newFilter.id);
      resetQuickFilterForm();
    } catch {
      // 忽略校验错误
    }
  }, [nameMaxLength, quickFilterForm, resetQuickFilterForm]);

  const handleToggleQuickFilterDisabled = useCallback((filterId: string) => {
    setDisabledQuickFilters((prev) =>
      prev.includes(filterId) ? prev.filter((id) => id !== filterId) : [...prev, filterId]
    );
  }, []);

  const handleDeleteQuickFilter = useCallback((filterId: string) => {
    setQuickFilters((prev) => prev.filter((item) => item.id !== filterId));
    setQuickFilterOrder((prev) => prev.filter((id) => id !== filterId));
    setDisabledQuickFilters((prev) => prev.filter((id) => id !== filterId));
    if (activeQuickFilterId === filterId) {
      setActiveQuickFilterId('all');
    }
  }, [activeQuickFilterId]);

  const handleQuickFilterClick = useCallback((filterId: string) => {
    setActiveQuickFilterId(filterId);
  }, []);

  const reorderQuickFilters = useCallback(
    (sourceId: string, targetId: string) => {
      setQuickFilterOrder((prev) => {
        const baseOrder =
          prev.length > 0 ? [...prev] : orderedQuickFilters.map((item) => item.id);
        const sourceIndex = baseOrder.indexOf(sourceId);
        const targetIndex = baseOrder.indexOf(targetId);
        if (sourceIndex === -1 || targetIndex === -1) {
          return baseOrder;
        }
        baseOrder.splice(sourceIndex, 1);
        baseOrder.splice(targetIndex, 0, sourceId);
        return baseOrder;
      });
    },
    [orderedQuickFilters]
  );

  const handleQuickFilterDragStart = useCallback(
    (filterId: string) => (event: DragEvent<HTMLSpanElement>) => {
      setDraggingFilterId(filterId);
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData('text/plain', filterId);
    },
    []
  );

  const handleQuickFilterDragOver = useCallback((event: DragEvent<HTMLDivElement>) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const handleQuickFilterDrop = useCallback(
    (targetId: string) => (event: DragEvent<HTMLDivElement>) => {
      event.preventDefault();
      const sourceId = draggingFilterId ?? event.dataTransfer.getData('text/plain');
      if (!sourceId || sourceId === targetId) {
        setDraggingFilterId(null);
        return;
      }
      reorderQuickFilters(sourceId, targetId);
      setDraggingFilterId(null);
    },
    [draggingFilterId, reorderQuickFilters]
  );

  const handleQuickFilterDragEnd = useCallback(() => {
    setDraggingFilterId(null);
  }, []);

  const quickFilterPanel = useMemo(
    () => (
      <UiModule01QuickFiltersPanel
        filters={visibleQuickFilters}
        activeFilterId={activeQuickFilterId}
        onOpenSettings={handleOpenQuickFilterModal}
        onSelectFilter={handleQuickFilterClick}
        maxLabelLength={resolvedLabelMaxLength}
        dataTestId={panelTestId}
        settingsButtonTestId={settingsButtonTestId}
        filterButtonTestIdPrefix={filterButtonTestIdPrefix}
      />
    ),
    [
      activeQuickFilterId,
      handleOpenQuickFilterModal,
      handleQuickFilterClick,
      resolvedLabelMaxLength,
      visibleQuickFilters
    ]
  );

  const quickFilterSettingsModal = (
    <UiModule01QuickFiltersSettingsModal
      open={quickFilterModalOpen}
      onCancel={() => setQuickFilterModalOpen(false)}
      onOk={() => setQuickFilterModalOpen(false)}
      titleProps={titleProps}
      modalRender={modalRender}
      form={quickFilterForm}
      reviewStatusOptions={reviewStatusOptions}
      platformOptions={platformOptions}
      shopOptions={shopOptions}
      warehouseOptions={warehouseOptions}
      orderStatusOptions={orderStatusOptions}
      taggedOptions={taggedOptions}
      exceptionFlagOptions={exceptionFlagOptions}
      maxNameLength={nameMaxLength}
      onCreateFilter={handleCreateQuickFilter}
      orderedFilters={orderedQuickFilters}
      disabledFilterIds={disabledQuickFilters}
      draggingFilterId={draggingFilterId}
      onDragOver={handleQuickFilterDragOver}
      onDrop={handleQuickFilterDrop}
      onDragStart={handleQuickFilterDragStart}
      onDragEnd={handleQuickFilterDragEnd}
      onToggleFilterDisabled={handleToggleQuickFilterDisabled}
      onDeleteFilter={handleDeleteQuickFilter}
    />
  );

  return {
    activeRule,
    activeFilterId: activeQuickFilterId,
    quickFilterPanel,
    quickFilterSettingsModal
  };
};
