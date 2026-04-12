import type { ReactNode } from 'react';

export type WorkbenchPageTemplateProps = {
  className?: string;
  toolbar?: ReactNode;
  leftPanel?: ReactNode;
  mainContent: ReactNode;
  rightPanel?: ReactNode;
  bottomPanel?: ReactNode;
  footer?: ReactNode;
};

const hasOwn = Object.prototype.hasOwnProperty;

export const WorkbenchPageTemplate = (props: WorkbenchPageTemplateProps) => {
  const { className, toolbar, leftPanel, mainContent, rightPanel, bottomPanel, footer } = props;
  const rootClassName = ['order-workbench-page', className].filter(Boolean).join(' ');
  const shouldRenderLeft = hasOwn.call(props, 'leftPanel');
  const shouldRenderRight = hasOwn.call(props, 'rightPanel');
  const shouldRenderBottom = hasOwn.call(props, 'bottomPanel');
  const shouldRenderFooter = hasOwn.call(props, 'footer');

  return (
    <div className={rootClassName}>
      {toolbar}
      <div className="order-workbench-body">
        {shouldRenderLeft ? <div className="order-workbench-left">{leftPanel}</div> : null}
        <div className="order-workbench-main">{mainContent}</div>
        {shouldRenderRight ? <div className="order-workbench-right">{rightPanel}</div> : null}
      </div>
      {shouldRenderBottom ? <div className="order-workbench-bottom">{bottomPanel}</div> : null}
      {shouldRenderFooter ? <div className="order-workbench-footer">{footer}</div> : null}
    </div>
  );
};
