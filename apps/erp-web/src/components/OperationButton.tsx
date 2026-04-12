import { Button, Tooltip } from 'antd';
import type { ButtonProps } from 'antd';

import type { PlatformCode } from '@/features/platform';
import type { OperationKey } from '@/features/operations';
import { useOperationAccess } from '@/features/operations';

type OperationButtonProps = ButtonProps & {
  operation: OperationKey;
  platform: PlatformCode;
};

export const OperationButton = ({
  operation,
  platform,
  disabled,
  ...rest
}: OperationButtonProps) => {
  const decision = useOperationAccess(operation, platform);
  const isDisabled = Boolean(disabled) || !decision.allowed;
  return (
    <Tooltip title={decision.allowed ? undefined : decision.reason}>
      <span>
        <Button {...rest} disabled={isDisabled} />
      </span>
    </Tooltip>
  );
};
