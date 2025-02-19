import type { LRUCache } from 'lru-cache';
import { QueryCache } from '../adapter/QueryCache';

export class CompilerCache extends QueryCache {
  // @ts-ignore
  protected readonly queryCache: LRUCache<string, QueryCache>;

  // @ts-ignore
  protected readonly rbacCache: LRUCache<string, any>;

  public constructor({ maxQueryCacheSize, maxQueryCacheAge }) {
    super();

    // @ts-ignore
    this.queryCache = new LRUCache({
      max: maxQueryCacheSize || 10000,
      // @ts-ignore
      maxAge: (maxQueryCacheAge * 1000) || 1000 * 60 * 10,
      updateAgeOnGet: true
    });

    // @ts-ignore
    this.rbacCache = new LRUCache({
      max: 10000,
      // @ts-ignore
      maxAge: 1000 * 60 * 5, // 5 minutes
    });
  }

  // @ts-ignore
  public getRbacCacheInstance(): LRUCache<string, any> {
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
