import type { ComponentPropsWithoutRef } from 'react';

import { UiModule02ActionSearchToolbar } from '@/components/ui-modules';

export type WorkbenchToolbarTemplateProps = ComponentPropsWithoutRef<typeof UiModule02ActionSearchToolbar>;

export const WorkbenchToolbarTemplate = ({
  className,
  searchInputClassName,
  searchButtonClassName,
  searchButtonAriaLabel = 'search',
  canExport = true,
  exporting = false,
  ...rest
}: WorkbenchToolbarTemplateProps) => {
  const rootClassName = ['order-workbench-toolbar', className].filter(Boolean).join(' ');
  const mergedSearchInputClassName = ['order-workbench-toolbar-search-input', searchInputClassName].filter(Boolean).join(' ');
  const mergedSearchButtonClassName = ['order-workbench-toolbar-search-button', searchButtonClassName].filter(Boolean).join(' ');

  return (
    <UiModule02ActionSearchToolbar
      {...rest}
      className={rootClassName}
      searchInputClassName={mergedSearchInputClassName}
      searchButtonClassName={mergedSearchButtonClassName}
      searchButtonAriaLabel={searchButtonAriaLabel}
      canExport={canExport}
      exporting={exporting}
    />
  );
};
