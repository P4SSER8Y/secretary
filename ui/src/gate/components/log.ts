export enum LEVEL {
  INFO,
  WARNING,
  SUCCESS,
  ERROR,
}

export interface info_t {
  level: LEVEL;
  msg: string;
  timeout: number;
}
