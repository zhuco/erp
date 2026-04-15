const DEFAULT_TRANSIENT_PATTERNS = [
  /502\s+Bad Gateway/i,
  /503\s+Service Unavailable/i,
  /504\s+Gateway Timeout/i,
  /upstream request failed/i,
  /temporarily unavailable/i,
  /ECONNRESET/i,
  /EAI_AGAIN/i,
  /ETIMEDOUT/i,
];

function toCombinedOutput(result) {
  const stdout = result?.stdout ?? '';
  const stderr = result?.stderr ?? '';
  return `${stdout}\n${stderr}`.trim();
}

function isTransientFailure(text, patterns = DEFAULT_TRANSIENT_PATTERNS) {
  const source = String(text ?? '');
  return patterns.some((pattern) => pattern.test(source));
}

const wait = async (delayMs) => new Promise((resolve) => {
  if (!delayMs || delayMs <= 0) {
    resolve();
    return;
  }
  setTimeout(resolve, delayMs);
});

async function runWithRetry({
  execute,
  maxRetries = 2,
  delayMs = 0,
  shouldRetry = isTransientFailure,
  onAttempt = () => {},
}) {
  if (typeof execute !== 'function') {
    throw new Error('execute must be a function');
  }
  if (!Number.isInteger(maxRetries) || maxRetries < 0) {
    throw new Error('maxRetries must be a non-negative integer');
  }

  const maxAttempts = maxRetries + 1;
  let attempt = 0;
  let lastResult = null;
  let retried = false;

  while (attempt < maxAttempts) {
    attempt += 1;
    const result = await execute(attempt);
    const combinedOutput = toCombinedOutput(result);
    const code = Number.isInteger(result?.code) ? result.code : 1;
    const ok = code === 0;
    const transient = !ok && shouldRetry(combinedOutput);
    const hasNext = attempt < maxAttempts;

    onAttempt({
      attempt,
      maxAttempts,
      result,
      ok,
      transient,
      willRetry: transient && hasNext,
    });

    lastResult = result;
    if (ok || !transient || !hasNext) {
      return {
        ok,
        attempts: attempt,
        maxAttempts,
        retried,
        lastResult,
        transient,
      };
    }

    retried = true;
    await wait(delayMs);
  }

  return {
    ok: false,
    attempts: attempt,
    maxAttempts,
    retried,
    lastResult,
    transient: isTransientFailure(toCombinedOutput(lastResult)),
  };
}

export {
  DEFAULT_TRANSIENT_PATTERNS,
  isTransientFailure,
  runWithRetry,
};
