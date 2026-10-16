// TRANSITIONAL (deleted with the JS runtime): the runtime's reactivity on the
// native graph (`crates/nsh-js/src/reactive.rs`), under the module name
// `solid-js` while the runtime and apps still use Solid's API. The graph
// (signals, memos, effects, owners, context) is native; what is left here is
// JS logic over it: the list mappers, the flow components and the props
// helpers, adapted from Solid 1.9 (MIT, Ryan Carniato).

const R = globalThis.__nsh_reactive;

const createSignal = R.createSignal;
const createMemo = R.createMemo;
const createRenderEffect = R.createRenderEffect;
const createEffect = R.createEffect;
const createRoot = R.createRoot;
const untrack = R.untrack;
const batch = R.batch;
const onCleanup = R.onCleanup;
const getOwner = R.getOwner;
const runWithOwner = R.runWithOwner;

const $PROXY = Symbol("solid-proxy");

/** Runs `fn` once, untracked, after the render effects. */
function onMount(fn) {
  createEffect(() => untrack(fn));
}

function createComponent(Comp, props) {
  return untrack(() => Comp(props || {}));
}

function createContext(defaultValue) {
  const id = Symbol("context");
  return { id, Provider: createProvider(id), defaultValue };
}

function useContext(context) {
  const value = R.useContext(context.id);
  return value !== undefined ? value : context.defaultValue;
}

function createProvider(id) {
  return function provider(props) {
    let res;
    createRenderEffect(
      () =>
        (res = untrack(() => {
          R.setContext(id, props.value);
          return children(() => props.children);
        })),
    );
    return res;
  };
}

function children(fn) {
  const c = createMemo(fn);
  const memo = createMemo(() => resolveChildren(c()));
  memo.toArray = () => {
    const v = memo();
    return Array.isArray(v) ? v : v != null ? [v] : [];
  };
  return memo;
}

function resolveChildren(children) {
  if (typeof children === "function" && !children.length) return resolveChildren(children());
  if (Array.isArray(children)) {
    const results = [];
    for (let i = 0; i < children.length; i++) {
      const result = resolveChildren(children[i]);
      if (Array.isArray(result)) {
        for (let j = 0; j < result.length; j++) results.push(result[j]);
      } else {
        results.push(result);
      }
    }
    return results;
  }
  return children;
}

const FALLBACK = Symbol("fallback");

function disposeAll(d) {
  for (let i = 0; i < d.length; i++) d[i]();
}

/** Maps `list()` with `mapFn(item, index)`, one root per item kept while the item stays. */
function mapArray(list, mapFn, options = {}) {
  let items = [],
    mapped = [],
    disposers = [],
    len = 0,
    indexes = mapFn.length > 1 ? [] : null;
  onCleanup(() => disposeAll(disposers));
  return () => {
    let newItems = list() || [],
      newLen = newItems.length,
      i,
      j;
    return untrack(() => {
      let newIndices, newIndicesNext, temp, tempdisposers, tempIndexes, start, end, newEnd, item;
      if (newLen === 0) {
        if (len !== 0) {
          disposeAll(disposers);
          disposers = [];
          items = [];
          mapped = [];
          len = 0;
          indexes && (indexes = []);
        }
        if (options.fallback) {
          items = [FALLBACK];
          mapped[0] = createRoot((disposer) => {
            disposers[0] = disposer;
            return options.fallback();
          });
          len = 1;
        }
      } else if (len === 0) {
        mapped = new Array(newLen);
        for (j = 0; j < newLen; j++) {
          items[j] = newItems[j];
          mapped[j] = createRoot(mapper);
        }
        len = newLen;
      } else {
        temp = new Array(newLen);
        tempdisposers = new Array(newLen);
        indexes && (tempIndexes = new Array(newLen));
        for (start = 0, end = Math.min(len, newLen); start < end && items[start] === newItems[start]; start++);
        for (
          end = len - 1, newEnd = newLen - 1;
          end >= start && newEnd >= start && items[end] === newItems[newEnd];
          end--, newEnd--
        ) {
          temp[newEnd] = mapped[end];
          tempdisposers[newEnd] = disposers[end];
          indexes && (tempIndexes[newEnd] = indexes[end]);
        }
        newIndices = new Map();
        newIndicesNext = new Array(newEnd + 1);
        for (j = newEnd; j >= start; j--) {
          item = newItems[j];
          i = newIndices.get(item);
          newIndicesNext[j] = i === undefined ? -1 : i;
          newIndices.set(item, j);
        }
        for (i = start; i <= end; i++) {
          item = items[i];
          j = newIndices.get(item);
          if (j !== undefined && j !== -1) {
            temp[j] = mapped[i];
            tempdisposers[j] = disposers[i];
            indexes && (tempIndexes[j] = indexes[i]);
            j = newIndicesNext[j];
            newIndices.set(item, j);
          } else disposers[i]();
        }
        for (j = start; j < newLen; j++) {
          if (j in temp) {
            mapped[j] = temp[j];
            disposers[j] = tempdisposers[j];
            if (indexes) {
              indexes[j] = tempIndexes[j];
              indexes[j](j);
            }
          } else mapped[j] = createRoot(mapper);
        }
        mapped = mapped.slice(0, (len = newLen));
        items = newItems.slice(0);
      }
      return mapped;
    });
    function mapper(disposer) {
      disposers[j] = disposer;
      if (indexes) {
        const [s, set] = createSignal(j);
        indexes[j] = set;
        return mapFn(newItems[j], s);
      }
      return mapFn(newItems[j]);
    }
  };
}

/** Maps `list()` by position: `mapFn(item(), index)`, `item` a signal per index. */
function indexArray(list, mapFn, options = {}) {
  let items = [],
    mapped = [],
    disposers = [],
    signals = [],
    len = 0,
    i;
  onCleanup(() => disposeAll(disposers));
  return () => {
    const newItems = list() || [],
      newLen = newItems.length;
    return untrack(() => {
      if (newLen === 0) {
        if (len !== 0) {
          disposeAll(disposers);
          disposers = [];
          items = [];
          mapped = [];
          len = 0;
          signals = [];
        }
        if (options.fallback) {
          items = [FALLBACK];
          mapped[0] = createRoot((disposer) => {
            disposers[0] = disposer;
            return options.fallback();
          });
          len = 1;
        }
        return mapped;
      }
      if (items[0] === FALLBACK) {
        disposers[0]();
        disposers = [];
        items = [];
        mapped = [];
        len = 0;
      }
      for (i = 0; i < newLen; i++) {
        if (i < items.length && items[i] !== newItems[i]) {
          signals[i](() => newItems[i]);
        } else if (i >= items.length) {
          mapped[i] = createRoot(mapper);
        }
      }
      for (; i < items.length; i++) {
        disposers[i]();
      }
      len = signals.length = disposers.length = newLen;
      items = newItems.slice(0);
      return (mapped = mapped.slice(0, len));
    });
    function mapper(disposer) {
      disposers[i] = disposer;
      const [s, set] = createSignal(newItems[i]);
      signals[i] = set;
      return mapFn(s, i);
    }
  };
}

function trueFn() {
  return true;
}

const propTraps = {
  get(_, property, receiver) {
    if (property === $PROXY) return receiver;
    return _.get(property);
  },
  has(_, property) {
    if (property === $PROXY) return true;
    return _.has(property);
  },
  set: trueFn,
  deleteProperty: trueFn,
  getOwnPropertyDescriptor(_, property) {
    return {
      configurable: true,
      enumerable: true,
      get() {
        return _.get(property);
      },
      set: trueFn,
      deleteProperty: trueFn,
    };
  },
  ownKeys(_) {
    return _.keys();
  },
};

function resolveSource(s) {
  return !(s = typeof s === "function" ? s() : s) ? {} : s;
}

function resolveSources() {
  for (let i = 0, length = this.length; i < length; ++i) {
    const v = this[i]();
    if (v !== undefined) return v;
  }
}

function mergeProps(...sources) {
  let proxy = false;
  for (let i = 0; i < sources.length; i++) {
    const s = sources[i];
    proxy = proxy || (!!s && $PROXY in s);
    sources[i] = typeof s === "function" ? ((proxy = true), createMemo(s)) : s;
  }
  if (proxy) {
    return new Proxy(
      {
        get(property) {
          for (let i = sources.length - 1; i >= 0; i--) {
            const v = resolveSource(sources[i])[property];
            if (v !== undefined) return v;
          }
        },
        has(property) {
          for (let i = sources.length - 1; i >= 0; i--) {
            if (property in resolveSource(sources[i])) return true;
          }
          return false;
        },
        keys() {
          const keys = [];
          for (let i = 0; i < sources.length; i++) keys.push(...Object.keys(resolveSource(sources[i])));
          return [...new Set(keys)];
        },
      },
      propTraps,
    );
  }
  const sourcesMap = {};
  const defined = Object.create(null);
  for (let i = sources.length - 1; i >= 0; i--) {
    const source = sources[i];
    if (!source) continue;
    const sourceKeys = Object.getOwnPropertyNames(source);
    for (let k = sourceKeys.length - 1; k >= 0; k--) {
      const key = sourceKeys[k];
      if (key === "__proto__" || key === "constructor") continue;
      const desc = Object.getOwnPropertyDescriptor(source, key);
      if (!defined[key]) {
        defined[key] = desc.get
          ? { enumerable: true, configurable: true, get: resolveSources.bind((sourcesMap[key] = [desc.get.bind(source)])) }
          : desc.value !== undefined
            ? desc
            : undefined;
      } else {
        const list = sourcesMap[key];
        if (list) {
          if (desc.get) list.push(desc.get.bind(source));
          else if (desc.value !== undefined) list.push(() => desc.value);
        }
      }
    }
  }
  const target = {};
  const definedKeys = Object.keys(defined);
  for (let i = definedKeys.length - 1; i >= 0; i--) {
    const key = definedKeys[i],
      desc = defined[key];
    if (desc && desc.get) Object.defineProperty(target, key, desc);
    else target[key] = desc ? desc.value : undefined;
  }
  return target;
}

function splitProps(props, ...keys) {
  const len = keys.length;
  if ($PROXY in props) {
    const blocked = len > 1 ? keys.flat() : keys[0];
    const claimed = new Set();
    const res = keys.map((k) => {
      const owned = k.filter((property) => !claimed.has(property) && (claimed.add(property), true));
      return new Proxy(
        {
          get(property) {
            return owned.includes(property) ? props[property] : undefined;
          },
          has(property) {
            return owned.includes(property) && property in props;
          },
          keys() {
            return owned.filter((property) => property in props);
          },
        },
        propTraps,
      );
    });
    res.push(
      new Proxy(
        {
          get(property) {
            return blocked.includes(property) ? undefined : props[property];
          },
          has(property) {
            return blocked.includes(property) ? false : property in props;
          },
          keys() {
            return Object.keys(props).filter((k) => !blocked.includes(k));
          },
        },
        propTraps,
      ),
    );
    return res;
  }
  const objects = [];
  for (let i = 0; i <= len; i++) objects[i] = {};
  for (const propName of Object.getOwnPropertyNames(props)) {
    let keyIndex = len;
    for (let i = 0; i < keys.length; i++) {
      if (keys[i].includes(propName)) {
        keyIndex = i;
        break;
      }
    }
    const desc = Object.getOwnPropertyDescriptor(props, propName);
    const isDefaultDesc = !desc.get && !desc.set && desc.enumerable && desc.writable && desc.configurable;
    isDefaultDesc ? (objects[keyIndex][propName] = desc.value) : Object.defineProperty(objects[keyIndex], propName, desc);
  }
  return objects;
}

const narrowedError = (name) => `Stale read from <${name}>.`;

function For(props) {
  const fallback = "fallback" in props && { fallback: () => props.fallback };
  return createMemo(mapArray(() => props.each, props.children, fallback || undefined));
}

function Index(props) {
  const fallback = "fallback" in props && { fallback: () => props.fallback };
  return createMemo(indexArray(() => props.each, props.children, fallback || undefined));
}

const truthiness = { equals: (a, b) => !a === !b };

function Show(props) {
  const keyed = props.keyed;
  const conditionValue = createMemo(() => props.when);
  const condition = keyed ? conditionValue : createMemo(conditionValue, undefined, truthiness);
  return createMemo(() => {
    const c = condition();
    if (c) {
      const child = props.children;
      const fn = typeof child === "function" && child.length > 0;
      return fn
        ? untrack(() =>
            child(
              keyed
                ? c
                : () => {
                    if (!untrack(condition)) throw narrowedError("Show");
                    return conditionValue();
                  },
            ),
          )
        : child;
    }
    return props.fallback;
  });
}

function Switch(props) {
  const chs = children(() => props.children);
  const switchFunc = createMemo(() => {
    const ch = chs();
    const mps = Array.isArray(ch) ? ch : [ch];
    let func = () => undefined;
    for (let i = 0; i < mps.length; i++) {
      const index = i;
      const mp = mps[i];
      const prevFunc = func;
      const conditionValue = createMemo(() => (prevFunc() ? undefined : mp.when));
      const condition = mp.keyed ? conditionValue : createMemo(conditionValue, undefined, truthiness);
      func = () => prevFunc() || (condition() ? [index, conditionValue, mp] : undefined);
    }
    return func;
  });
  return createMemo(() => {
    const sel = switchFunc()();
    if (!sel) return props.fallback;
    const [index, conditionValue, mp] = sel;
    const child = mp.children;
    const fn = typeof child === "function" && child.length > 0;
    return fn
      ? untrack(() =>
          child(
            mp.keyed
              ? conditionValue()
              : () => {
                  if (untrack(switchFunc)()?.[0] !== index) throw narrowedError("Match");
                  return conditionValue();
                },
          ),
        )
      : child;
  });
}

function Match(props) {
  return props;
}

export {
  createSignal,
  createMemo,
  createRenderEffect,
  createEffect,
  createRoot,
  untrack,
  batch,
  onCleanup,
  getOwner,
  runWithOwner,
  $PROXY,
  onMount,
  createComponent,
  createContext,
  useContext,
  children,
  mapArray,
  indexArray,
  mergeProps,
  splitProps,
  For,
  Index,
  Show,
  Switch,
  Match,
};
