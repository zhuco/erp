import { isValidElement, type ReactElement } from 'react';

import {
  IndustrialWorkbenchTemplate,
  type IndustrialWorkbenchTemplateProps,
} from './IndustrialWorkbenchTemplate';

import { WorkbenchToolbar, type WorkbenchToolbarProps } from './WorkbenchToolbar';

type WorkbenchToolbarElement = ReactElement<WorkbenchToolbarProps, typeof WorkbenchToolbar>;

export type WorkbenchPageTemplateProps = Omit<IndustrialWorkbenchTemplateProps, 'toolbar'> & {
  toolbar?: WorkbenchToolbarElement | null;
};

const isWorkbenchToolbarElement = (toolbar: WorkbenchPageTemplateProps['toolbar']) =>
  toolbar == null || (isValidElement(toolbar) && toolbar.type === WorkbenchToolbar);

export const WorkbenchPageTemplate = ({ toolbar, ...rest }: WorkbenchPageTemplateProps) => {
  if (!isWorkbenchToolbarElement(toolbar)) {
    throw new Error('WorkbenchPageTemplate 的 toolbar 必须使用 WorkbenchToolbar，以保持工作台操作区一致。');
  }

  return <IndustrialWorkbenchTemplate {...rest} toolbar={toolbar} />;
};
