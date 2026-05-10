import { TagDirectoryWorkbench } from './TagDirectoryWorkbench';

type SkuTagDefinitionsPageProps = {
  showTitle?: boolean;
};

export const SkuTagDefinitionsPage = ({ showTitle = true }: SkuTagDefinitionsPageProps) => (
  <TagDirectoryWorkbench fixedTab="skus" showTabs={false} showDirectoryTitle={showTitle} />
);
