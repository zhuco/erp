import { isValidElement, type ReactElement } from 'react';

import {
  OrderReviewWorkbenchTemplate,
  type OrderReviewWorkbenchTemplateProps,
} from '@/components/workbench/templates/OrderReviewWorkbenchTemplate';

import { WorkbenchToolbar, type WorkbenchToolbarProps } from './WorkbenchToolbar';

type WorkbenchToolbarElement = ReactElement<WorkbenchToolbarProps, typeof WorkbenchToolbar>;

export type WorkbenchPageTemplateProps = Omit<OrderReviewWorkbenchTemplateProps, 'toolbar'> & {
  toolbar?: WorkbenchToolbarElement | null;
};

const isWorkbenchToolbarElement = (toolbar: WorkbenchPageTemplateProps['toolbar']) =>
  toolbar == null || (isValidElement(toolbar) && toolbar.type === WorkbenchToolbar);

export const WorkbenchPageTemplate = ({ toolbar, ...rest }: WorkbenchPageTemplateProps) => {
  if (!isWorkbenchToolbarElement(toolbar)) {
    throw new Error('WorkbenchPageTemplate ? toolbar ???? WorkbenchToolbar??????????????????????????????');
  }

  return <OrderReviewWorkbenchTemplate {...rest} toolbar={toolbar} />;
};
