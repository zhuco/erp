import { TagDirectoryWorkbench } from './TagDirectoryWorkbench';

type OrderTagDefinitionsPageProps = {
  showTitle?: boolean;
};

export const OrderTagDefinitionsPage = ({ showTitle = true }: OrderTagDefinitionsPageProps) => (
  <TagDirectoryWorkbench fixedTab="orders" showTabs={false} showDirectoryTitle={showTitle} />
);
