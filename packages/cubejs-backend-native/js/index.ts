/* eslint-disable import/no-dynamic-require,global-require */
import fs from 'fs';
import path from 'path';
import { Writable } from 'stream';
import type { Request as ExpressRequest } from 'express';
import { CubeStoreResultWrapper } from './CubeStoreResultWrapper';

export * from './CubeStoreResultWrapper';

export interface BaseMeta {
  // postgres or mysql
  protocol: string,
  // always sql
  apiType: string,
  // Application name, for example Metabase
  appName?: string,
}

export interface LoadRequestMeta extends BaseMeta {
  // Security Context switching
  changeUser?: string,
}

export interface Request<Meta> {
  id: string,
  meta: Meta,
}

export interface CheckAuthResponse {
  password: string | null,
  superuser: boolean,
  securityContext: any,
  skipPasswordCheck?: boolean,
}

export interface CheckAuthPayload {
  request: Request<undefined>,
  user: string | null,
  password: string | null,
}

export interface SessionContext {
  user: string | null,
  superuser: boolean,
  securityContext: any,
}

export interface LoadPayload {
  request: Request<LoadRequestMeta>,
  session: SessionContext,
  query: any,
}

export interface SqlPayload {
  request: Request<LoadRequestMeta>,
  session: SessionContext,
  query: any,
  memberToAlias: Record<string, string>,
  expressionParams: string[],
}

export interface SqlApiLoadPayload {
  request: Request<LoadRequestMeta>,
  session: SessionContext,
  query: any,
  queryKey: any,
  sqlQuery: any,
  streaming: boolean,
}

export interface LogLoadEventPayload {
  request: Request<LoadRequestMeta>,
  session: SessionContext,
  event: string,
  properties: any,
}

export interface MetaPayload {
  request: Request<undefined>,
  session: SessionContext,
  onlyCompilerId?: boolean,
}

export interface CanSwitchUserPayload {
  session: SessionContext,
  user: string,
}

export type SQLInterfaceOptions = {
  pgPort?: number,
  checkAuth: (payload: CheckAuthPayload) => CheckAuthResponse | Promise<CheckAuthResponse>,
  load: (payload: LoadPayload) => unknown | Promise<unknown>,
  sql: (payload: SqlPayload) => unknown | Promise<unknown>,
  meta: (payload: MetaPayload) => unknown | Promise<unknown>,
  stream: (payload: LoadPayload) => unknown | Promise<unknown>,
  sqlApiLoad: (payload: SqlApiLoadPayload) => unknown | Promise<unknown>,
  logLoadEvent: (payload: LogLoadEventPayload) => unknown | Promise<unknown>,
  sqlGenerators: (paramsJson: string) => unknown | Promise<unknown>,
  canSwitchUserForSession: (payload: CanSwitchUserPayload) => unknown | Promise<unknown>,
  // gateway options
  gatewayPort?: number,
};

export type DBResponsePrimitive =
  null |
  boolean |
  number |
  string;

export type TransformDataResponse = {
  members: string[],
  dataset: DBResponsePrimitive[][]
} | {
  [member: string]: DBResponsePrimitive
}[];

export type TransformDataResponseNative = {
  result: string
};

export function loadNative() {
  // Development version
  if (fs.existsSync(path.join(__dirname, '/../../index.node'))) {
    return require(path.join(__dirname, '/../../index.node'));
  }

  if (fs.existsSync(path.join(__dirname, '/../../native/index.node'))) {
    return require(path.join(__dirname, '/../../native/index.node'));
  }

  throw new Error(
    `Unable to load @cubejs-backend/native, probably your system (${process.arch}-${process.platform}) with Node.js ${process.version} is not supported.`,
  );
}

export function isSupported(): boolean {
  return fs.existsSync(path.join(__dirname, '/../../index.node')) || fs.existsSync(path.join(__dirname, '/../../native/index.node'));
}

function wrapNativeFunctionWithChannelCallback(
  fn: (extra: any) => unknown | Promise<unknown>,
) {
  return async (extra: any, channel: any) => {
    try {
      const result = await fn(JSON.parse(extra));

      if (process.env.CUBEJS_NATIVE_INTERNAL_DEBUG) {
        console.debug('[js] channel.resolve', {
          result,
        });
      }

      if (!result) {
        channel.resolve('');
      } else {
        channel.resolve(JSON.stringify(result));
      }
    } catch (e: any) {
      if (process.env.CUBEJS_NATIVE_INTERNAL_DEBUG) {
        console.debug('[js] channel.reject', {
          e,
        });
      }
      try {
        channel.reject(e.message || 'Unknown JS exception');
      } catch (rejectErr: unknown) {
        if (process.env.CUBEJS_NATIVE_INTERNAL_DEBUG) {
          console.debug('[js] channel.reject exception', {
            e: rejectErr,
          });
        }
      }

      // throw e;
    }
  };
}

function wrapRawNativeFunctionWithChannelCallback(
  fn: (extra: any) => unknown | Promise<unknown>,
) {
  return async (extra: any, channel: any) => {
    try {
      const result = await fn(extra);

      if (process.env.CUBEJS_NATIVE_INTERNAL_DEBUG) {
        console.debug('[js] channel.resolve', {
          result,
        });
      }
      channel.resolve(result);
    } catch (e: any) {
      if (process.env.CUBEJS_NATIVE_INTERNAL_DEBUG) {
        console.debug('[js] channel.reject', {
          e,
        });
      }
      try {
        channel.reject(e.message || e.toString());
      } catch (error) {
        if (process.env.CUBEJS_NATIVE_INTERNAL_DEBUG) {
          console.debug('[js] channel.reject exception', {
            e: error,
          });
        }
      }

      // throw e;
    }
  };
}

const errorString = (err: any) => err.error ||
  err.message ||
  err.stack?.toString() ||
  (typeof err === 'string' ? err.toString() : JSON.stringify(err));

// TODO: Refactor - define classes
function wrapNativeFunctionWithStream(
  fn: (extra: any) => unknown | Promise<unknown>,
) {
  const chunkLength = parseInt(
    process.env.CUBEJS_DB_QUERY_STREAM_HIGH_WATER_MARK || '8192',
    10,
  );
  return async (extra: any, writerOrChannel: any) => {
    let response: any;
    try {
      response = await fn(JSON.parse(extra));
      if (response && response.stream) {
        writerOrChannel.start();

        let chunkBuffer: any[] = [];
        const writable = new Writable({
          objectMode: true,
          highWaterMark: chunkLength,
          write(row: any, encoding: BufferEncoding, callback: (error?: (Error | null)) => void) {
            chunkBuffer.push(row);
            if (chunkBuffer.length < chunkLength) {
              callback(null);
            } else {
              const toSend = chunkBuffer;
              chunkBuffer = [];
              writerOrChannel.chunk(toSend, callback);
            }
          },
          final(callback: (error?: (Error | null)) => void) {
            const end = (err: any) => {
              if (err) {
                callback(err);
              } else {
                writerOrChannel.end(callback);
              }
            };
            if (chunkBuffer.length > 0) {
              const toSend = chunkBuffer;
              chunkBuffer = [];
              writerOrChannel.chunk(toSend, end);
            } else {
              end(null);
            }
          },
          destroy(error: Error | null, callback: (error: (Error | null)) => void) {
            if (error) {
              writerOrChannel.reject(errorString(error));
            }
            callback(null);
          },
        });
        response.stream.pipe(writable);
        response.stream.on('error', (err: any) => {
          writable.destroy(err);
        });
      } else if (response.error) {
        writerOrChannel.reject(errorString(response));
      } else {
        // TODO remove JSON.stringify()
        writerOrChannel.resolve(JSON.stringify(response));
      }
    } catch (e: any) {
      if (!!response && !!response.stream) {
        response.stream.destroy(e);
      }
      writerOrChannel.reject(errorString(e));
    }
  };
}

type LogLevel = 'error' | 'warn' | 'info' | 'debug' | 'trace';

export const setupLogger = (logger: (extra: any) => unknown, logLevel: LogLevel): void => {
  const native = loadNative();
  native.setupLogger({ logger: wrapNativeFunctionWithChannelCallback(logger), logLevel });
};

export const isFallbackBuild = (): boolean => {
  const native = loadNative();
  return native.isFallbackBuild();
};

export type SqlInterfaceInstance = { __typename: 'sqlinterfaceinstance' };

export const registerInterface = async (options: SQLInterfaceOptions): Promise<SqlInterfaceInstance> => {
  if (typeof options !== 'object' && options == null) {
    throw new Error('Argument options must be an object');
  }

  if (typeof options.checkAuth !== 'function') {
    throw new Error('options.checkAuth must be a function');
  }

  if (typeof options.load !== 'function') {
    throw new Error('options.load must be a function');
  }

  if (typeof options.meta !== 'function') {
    throw new Error('options.meta must be a function');
  }

  if (typeof options.stream !== 'function') {
    throw new Error('options.stream must be a function');
  }

  if (typeof options.sqlApiLoad !== 'function') {
    throw new Error('options.sqlApiLoad must be a function');
  }

  if (typeof options.sqlGenerators !== 'function') {
    throw new Error('options.sqlGenerators must be a function');
  }

  if (typeof options.sql !== 'function') {
    throw new Error('options.sql must be a function');
  }

  const native = loadNative();
  return native.registerInterface({
    ...options,
    checkAuth: wrapNativeFunctionWithChannelCallback(options.checkAuth),
    load: wrapNativeFunctionWithChannelCallback(options.load),
    sql: wrapNativeFunctionWithChannelCallback(options.sql),
    meta: wrapNativeFunctionWithChannelCallback(options.meta),
    stream: wrapNativeFunctionWithStream(options.stream),
    sqlApiLoad: wrapNativeFunctionWithStream(options.sqlApiLoad),
    sqlGenerators: wrapRawNativeFunctionWithChannelCallback(options.sqlGenerators),
    logLoadEvent: wrapRawNativeFunctionWithChannelCallback(options.logLoadEvent),
    canSwitchUserForSession: wrapRawNativeFunctionWithChannelCallback(options.canSwitchUserForSession),
  });
};

export type ShutdownMode = 'fast' | 'semifast' | 'smart';

export const shutdownInterface = async (instance: SqlInterfaceInstance, shutdownMode: ShutdownMode): Promise<void> => {
  const native = loadNative();

  await native.shutdownInterface(instance, shutdownMode);
};

export const execSql = async (instance: SqlInterfaceInstance, sqlQuery: string, stream: any, securityContext?: any): Promise<void> => {
  const native = loadNative();

  await native.execSql(instance, sqlQuery, stream, securityContext ? JSON.stringify(securityContext) : null);
};

export const buildSqlAndParams = (cubeEvaluator: any): String => {
  const native = loadNative();

  return native.buildSqlAndParams(cubeEvaluator);
};

export type ResultRow = Record<string, string>;

export const parseCubestoreResultMessage = async (message: ArrayBuffer): Promise<CubeStoreResultWrapper> => {
  const native = loadNative();

  const msg = await native.parseCubestoreResultMessage(message);
  return new CubeStoreResultWrapper(msg);
};

export const getCubestoreResult = (ref: CubeStoreResultWrapper): ResultRow[] => {
  const native = loadNative();

  return native.getCubestoreResult(ref);
};

export const transformData = (transformDataObj: Object, rows: any): Promise<TransformDataResponseNative> => {
  const native = loadNative();

  return native.transformQueryData(transformDataObj, rows);
};

export const getFinalQueryResult = (transformDataObj: Object, rows: any, resultData: Object): Promise<ArrayBuffer> => {
  const native = loadNative();

  return native.getFinalQueryResult(transformDataObj, rows, resultData);
};

export const getFinalQueryResultArray = (transformDataArr: Object[], rows: any[], resultDataArr: Object[]): Promise<ArrayBuffer> => {
  const native = loadNative();

  return native.getFinalQueryResultArray(transformDataArr, rows, resultDataArr);
};

export const getFinalQueryResultMulti = (transformDataArr: Object[], rows: any[], responseData: Object): Promise<ArrayBuffer> => {
  const native = loadNative();

  return native.getFinalQueryResultMulti(transformDataArr, rows, responseData);
};

export interface PyConfiguration {
  repositoryFactory?: (ctx: unknown) => Promise<unknown>,
  logger?: (msg: string, params: Record<string, any>) => void,
  checkAuth?: (req: unknown, authorization: string) => Promise<{ 'security_context'?: unknown }>
  extendContext?: (req: unknown) => Promise<unknown>
  queryRewrite?: (query: unknown, ctx: unknown) => Promise<unknown>
  contextToApiScopes?: () => Promise<string[]>
  scheduledRefreshContexts?: (ctx: unknown) => Promise<string[]>
  scheduledRefreshTimeZones?: (ctx: unknown) => Promise<string[]>
  contextToRoles?: (ctx: unknown) => Promise<string[]>
}

function simplifyExpressRequest(req: ExpressRequest) {
  // Req is a large object, let's simplify it
  // Important: Dont pass circular references
  return {
    url: req.url,
    method: req.method,
    headers: req.headers,
    ip: req.ip,

    // req.securityContext is an extension of request done by api-gateway
    // But its typings currently live in api-gateway package, which has native-backend (this package) as it's dependency
    // TODO extract typings to separate package and drop as any
    ...(Object.hasOwn(req, 'securityContext') ? { securityContext: (req as any).securityContext } : {}),
  };
}

export const pythonLoadConfig = async (content: string, options: { fileName: string }): Promise<PyConfiguration> => {
  if (isFallbackBuild()) {
    throw new Error('Python is not supported in fallback build');
  }

  const native = loadNative();
  const config = await native.pythonLoadConfig(content, options);

  if (config.checkAuth) {
    const nativeCheckAuth = config.checkAuth;
    config.checkAuth = async (req: ExpressRequest, authorization: string) => {
      const nativeResult = await nativeCheckAuth(
        simplifyExpressRequest(req),
        authorization,
      );
      const securityContext = nativeResult?.security_context;
      return {
        ...(securityContext ? { security_context: securityContext } : {})
      };
    };
  }

  if (config.extendContext) {
    const nativeExtendContext = config.extendContext;
    config.extendContext = async (req: ExpressRequest) => nativeExtendContext(
      simplifyExpressRequest(req),
    );
  }

  if (config.repositoryFactory) {
    const nativeRepositoryFactory = config.repositoryFactory;
    config.repositoryFactory = (ctx: any) => ({
      dataSchemaFiles: async () => nativeRepositoryFactory(
        ctx,
      ),
    });
  }

  if (config.logger) {
    const nativeLogger = config.logger;
    config.logger = (msg: string, params: Record<string, any>) => {
      nativeLogger(msg, params).catch((e: any) => {
        console.error(e);
      });
    };
  }

  return config;
};

export type PythonCtx = {
  __type: 'PythonCtx'
} & {
  filters: Record<string, Function>
  functions: Record<string, Function>
  variables: Record<string, any>
};

export type JinjaEngineOptions = {
  debugInfo?: boolean,
  filters: Record<string, Function>,
  workers: number
};

export interface JinjaEngine {
  loadTemplate(templateName: string, templateContent: string): void;

  renderTemplate(templateName: string, context: unknown, pythonContext: Record<string, any> | null): Promise<string>;
}

export class NativeInstance {
  protected native: any | null = null;

  protected getNative(): any {
    if (this.native) {
      return this.native;
    }

    this.native = loadNative();

    return this.native;
  }

  public newJinjaEngine(options: JinjaEngineOptions): JinjaEngine {
    return this.getNative().newJinjaEngine(options);
  }

  public loadPythonContext(fileName: string, content: unknown): Promise<PythonCtx> {
    if (isFallbackBuild()) {
      throw new Error(
        'Python (loadPythonContext) is not supported because you are using the fallback build of native extension. Read more: ' +
        'https://github.com/cube-js/cube/blob/master/packages/cubejs-backend-native/README.md#supported-architectures-and-platforms',
      );
    }

    return this.getNative().pythonLoadModel(fileName, content);
  }
}
