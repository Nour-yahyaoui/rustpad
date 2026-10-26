export interface AppError {
  title: string;
  message: string;
  retryable: boolean;
}

/**
 * Converts unknown runtime or network failures into calm, actionable human messages
 * without exposing raw stack traces or internal exception strings.
 */
export function toAppError(err: unknown): AppError {
  if (typeof navigator !== 'undefined' && navigator.onLine === false) {
    return {
      title: 'Connection unavailable',
      message:
        'Your browser appears to be offline, so the specimen file could not be fetched. Reconnect and retry.',
      retryable: true,
    };
  }

  if (err instanceof Response || (typeof err === 'object' && err !== null && 'status' in err)) {
    const status = Number((err as { status?: number }).status ?? 0);
    if (status === 404) {
      return {
        title: 'Specimen file not found',
        message: 'The requested document specimen is missing from the static bundle.',
        retryable: false,
      };
    }
    if (status >= 500) {
      return {
        title: 'Server temporarily unavailable',
        message: 'The static asset host returned an unexpected error. Wait a moment and retry.',
        retryable: true,
      };
    }
  }

  return {
    title: 'Could not inspect specimen bytes',
    message:
      'The binary header preview did not finish loading. You can retry the inspection or download the file directly.',
    retryable: true,
  };
}
