import LRUCache from 'lru-cache';
import { QueryCache } from '../adapter/QueryCache';

export class CompilerCache extends QueryCache {
  // @ts-ignore
  protected readonly queryCache: InstanceType<typeof LRUCache>;

  // @ts-ignore
  protected readonly rbacCache: InstanceType<typeof LRUCache>;

  public constructor({ maxQueryCacheSize, maxQueryCacheAge }) {
    super();

    // @ts-ignore
    this.queryCache = new LRUCache({
      max: maxQueryCacheSize || 10000,
      maxAge: (maxQueryCacheAge * 1000) || 1000 * 60 * 10,
      updateAgeOnGet: true
    });

    // @ts-ignore
    this.rbacCache = new LRUCache({
      max: 10000,
      maxAge: 1000 * 60 * 5, // 5 minutes
    });
  }

  // @ts-ignore
  public getRbacCacheInstance(): InstanceType<typeof LRUCache> {
    return this.rbacCache;
  }

  public getQueryCache(key: unknown): QueryCache {
    const keyString = JSON.stringify(key);

    const exist = this.queryCache.get(keyString);
    if (exist) {
      return exist;
    }

    const result = new QueryCache();
    this.queryCache.set(keyString, result);

    return result;
  }
}
