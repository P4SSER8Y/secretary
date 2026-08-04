export type Err<T> = {
  v: T | undefined;
  err: null | Error;
};

export async function h<T>(promise: Promise<T>): Promise<Err<T>> {
  try {
    const v = await promise;
    return { v: v, err: null };
  } catch (err) {
    return { v: undefined, err: err instanceof Error ? err : new Error(String(err)) };
  }
}
