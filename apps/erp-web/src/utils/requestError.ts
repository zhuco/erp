import { message } from 'antd';

import { getErrorMessage } from '@/utils/errors';

type ErrorReporter = (message: string) => void;

let reportError: ErrorReporter = (content: string) => {
  message.error(content);
};

export const setRequestErrorReporter = (reporter: ErrorReporter) => {
  reportError = reporter;
};

export const notifyRequestError = (error: unknown) => {
  reportError(getErrorMessage(error));
};
