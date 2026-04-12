import type { ReactNode, Ref } from 'react';

import { WorkbenchPageTemplate } from '@/components/workbench/templates';

import './UiModule02WorkbenchTemplate.css';

type UiModule02WorkbenchTemplateProps = {
  className?: string;
  toolbar?: ReactNode;
  toolbarLeft?: ReactNode;
  toolbarCenter?: ReactNode;
  toolbarRight?: ReactNode;
  header?: ReactNode;
  actionBar?: ReactNode;
  tableContainerRef?: Ref<HTMLDivElement>;
  tableClassName?: string;
  tableContent: ReactNode;
  tableFooter?: ReactNode;
};

export const UiModule02WorkbenchTemplate = ({
  className,
  toolbar,
  toolbarLeft,
  toolbarCenter,
  toolbarRight,
  header,
  actionBar,
  tableContainerRef,
  tableClassName,
  tableContent,
  tableFooter
}: UiModule02WorkbenchTemplateProps) => {
  const tableClassNames = ['order-workbench-table', tableClassName]
    .filter(Boolean)
    .join(' ');

  return (
    <WorkbenchPageTemplate
      className={className}
      toolbar={
        toolbar ? (
          toolbar
        ) : (
          <div className="order-workbench-toolbar">
            {toolbarLeft ? <div className="order-workbench-toolbar-left">{toolbarLeft}</div> : null}
            {toolbarCenter ? (
              <div className="order-workbench-toolbar-center">{toolbarCenter}</div>
            ) : null}
            {toolbarRight ? (
              <div className="order-workbench-toolbar-right">{toolbarRight}</div>
            ) : null}
          </div>
        )
      }
      mainContent={
        <>
          {header ? <div className="order-workbench-header">{header}</div> : null}
          <div className={tableClassNames} ref={tableContainerRef}>
            {tableContent}
            {tableFooter}
          </div>
          {actionBar ? <div className="order-workbench-action">{actionBar}</div> : null}
        </>
      }
    />
  );
};
