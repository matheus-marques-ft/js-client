import type { UnlistenFn } from "@tauri-apps/api/event";
import type { AssetPageType, AssetsResponse, PermedAccount, PermedProtocol, RawAssetData } from "~/types";

import { useUserInfoStore } from "~/store/modules/userInfo";

const LIMIT = 20;
let assetRequestSequence = 0;

// get_assets returns results via a global event; the request ID distinguishes between different menus and different pagination requests.
const createAssetRequestId = (assetType: AssetPageType) => {
  assetRequestSequence += 1;
  return `${assetType}-${Date.now()}-${assetRequestSequence}`;
};

export const useAssetFetcher = (assetType: AssetPageType, scrollRef?: Ref<HTMLElement | null>) => {
  const { t } = useI18n();
  const { componentsConfig } = useAppConfig();

  const toast = useToast();
  const route = useRoute();
  const colorMode = useColorMode();
  const userInfoStore = useUserInfoStore();

  const { deleteUserData } = userInfoStore;
  const { currentSite, orgId } = storeToRefs(userInfoStore);

  const offset = ref(0);
  const hasMore = ref(true);
  const getDetail = ref(false);
  const isLoading = ref(false);
  const rawAssetsList = ref<RawAssetData[]>([]);
  const lastDetailAssetId = ref<string | null>(null);
  const activeRequestId = ref<string | null>(null);
  const subscribeGetAssetsEvent = ref<UnlistenFn | null>(null);
  const subscribeGetAssetFailedEvent = ref<UnlistenFn | null>(null);
  const subscribeGetFavoriteAssetsEvent = ref<UnlistenFn | null>(null);

  const totalCount = ref(0);
  const currentOrder = ref("");
  const currentSearch = ref("");

  let stopResizeObserver: (() => void) | null = null;
  let stopScrollListener: (() => void) | null = null;

  const favoriteSet = ref<Set<string>>(new Set());

  const assetsData = computed(() => {
    const list = transformAssetsData(rawAssetsList.value);
    return list.map((a) => ({ ...a, isFavorite: favoriteSet.value.has(a.id) }));
  });

  const isAppending = computed(() => isLoading.value && rawAssetsList.value.length > 0);

  const isInitialLoading = computed(() => isLoading.value && rawAssetsList.value.length === 0);

  const appendSkeletonCount = computed(() => {
    return 1;
  });

  const scrollbarStyles = computed(() => {
    const isDark = colorMode.value === "dark";
    return {
      "--scrollbar-width": "8px",
      "--scrollbar-track-color": isDark ? "#333" : "#f1f1f1",
      "--scrollbar-thumb-color": isDark
        ? componentsConfig.pages.scrollBarDarkThumbColor
        : componentsConfig.pages.scrollBarLightThumbColor,
      "--scrollbar-thumb-hover-color": isDark
        ? componentsConfig.pages.scrollBarDarkHoverColor
        : componentsConfig.pages.scrollBarLightHoverColor
    };
  });

  watchEffect((onCleanup) => {
    if (hasMore.value && scrollRef?.value) {
      ensureScrollListener();
      clientResizeObserver();
    } else {
      stopScrollListener?.();
      stopResizeObserver?.();

      stopResizeObserver = null;
      stopScrollListener = null;
    }

    onCleanup(() => {
      stopScrollListener?.();
      stopResizeObserver?.();

      stopResizeObserver = null;
      stopScrollListener = null;
    });
  });

  function prefetchToFill() {
    if (!scrollRef?.value) return;

    const el = scrollRef.value;
    const notScrollable = el.scrollHeight <= el.clientHeight + 1;

    // If not scrollable and there's more data, keep requesting the next page
    if (notScrollable && hasMore.value && !isLoading.value) {
      fetchNextPage(currentSearch.value, currentOrder.value);
    }
  }

  function ensureScrollListener() {
    if (!scrollRef?.value) return;
    if (stopScrollListener) return;

    const el = scrollRef.value!;
    let ticking = false;
    const onScroll = () => {
      if (ticking) return;
      ticking = true;

      requestAnimationFrame(() => {
        ticking = false;
        if (!hasMore.value || isLoading.value) return;

        // Total content height of the element - distance scrolled up - visible height of the element
        const distanceToBottom = el.scrollHeight - el.scrollTop - el.clientHeight;

        if (distanceToBottom <= 50) {
          fetchNextPage();
        }
      });
    };

    el.addEventListener("scroll", onScroll, { passive: true });

    stopScrollListener = () => {
      el.removeEventListener("scroll", onScroll);
      stopScrollListener = null;
    };
  }

  function clientResizeObserver() {
    if (!scrollRef?.value || stopResizeObserver) return;

    const el = scrollRef.value;
    const resizeObserver = new ResizeObserver(() => {
      if (!isLoading.value) prefetchToFill();
    });

    resizeObserver.observe(el);

    stopResizeObserver = () => {
      resizeObserver.disconnect();
      stopResizeObserver = null;
    };
  }

  /**
   * @description Start loading
   */
  const beginLoading = () => {
    isLoading.value = true;

    try {
      useEventBus().emit("loading", undefined);
    } catch {}
  };

  /**
   * @description Finish loading
   */
  const endLoading = () => {
    isLoading.value = false;
    nextTick(() => {
      try {
        useEventBus().emit("loaded", undefined);
        prefetchToFill();
      } catch {}
    });
  };

  /**
   * @description Check whether this is the current route
   */
  const isActiveForCurrentRoute = () => {
    const pathLower = route.path.toLowerCase();

    switch (assetType) {
      case "favorite":
        return /\/favorite(?:\/|$)/.test(pathLower);
      case "linux":
        return /\/linux(?:\/|$)/.test(pathLower);
      case "windows":
        return /\/windows(?:\/|$)/.test(pathLower);
      case "windows_ad":
        return /\/windows_ad(?:\/|$)/.test(pathLower);
      case "other":
        return /\/other(?:\/|$)/.test(pathLower);
      case "database":
        return /\/database(?:\/|$)/.test(pathLower);
      case "device":
        return /\/device(?:\/|$)/.test(pathLower);
      case "web":
        return /\/web(?:\/|$)/.test(pathLower);
      default:
        return true;
    }
  };

  /**
   * @description Filter results by asset type
   * @param items
   */
  const filterResultsByAssetType = (items: RawAssetData[]) => {
    switch (assetType) {
      case "favorite":
        return items;
      case "linux":
        return items.filter((it) => {
          const typeValue = it.type?.value?.toLowerCase();
          return typeValue === "linux";
        });
      case "windows":
        return items.filter((it) => {
          const typeValue = it.type?.value?.toLowerCase();
          // The Windows menu merges regular Windows and Windows AD assets.
          return typeValue === "windows" || typeValue === "windows_ad";
        });
      case "windows_ad":
        return items.filter((it) => {
          const typeValue = it.type?.value?.toLowerCase();
          return typeValue === "windows_ad";
        });
      case "other":
        return items.filter((it) => {
          const typeValue = it.type?.value?.toLowerCase();
          // The “Other” page covers the unix / other host types from JumpServer.
          return typeValue === "unix" || typeValue === "other";
        });
      case "database":
        return items.filter((it) => {
          const typeValue = it.category?.value?.toLowerCase();
          return typeValue === "database";
        });
      case "device":
        return items.filter((it) => {
          const typeValue = it.category?.value?.toLowerCase();
          return typeValue === "device";
        });
      default:
        return items;
    }
  };

  /**
   * @description Append paginated data
   * @param pageData
   * @param fetchedCount
   * @param count
   */
  const appendPageData = (pageData: RawAssetData[], fetchedCount: number, count?: number | null) => {
    // If there are more than 20 items, only show 20
    if (pageData.length > LIMIT) pageData = pageData.slice(0, LIMIT);

    rawAssetsList.value.push(...pageData);
    // pageData is the filtered display data; the pagination cursor must advance by the server's raw page item count, to stay consistent with count.
    offset.value += fetchedCount;
    totalCount.value = count ?? offset.value;
    // An empty raw page means pagination made no progress; stop even if count looks abnormally large, to avoid repeatedly requesting the same offset.
    hasMore.value = fetchedCount > 0 && offset.value < totalCount.value;
  };

  /**
   * @description Fetch the next page of asset data
   * @param search
   * @param order
   */
  async function fetchNextPage(search?: string, order?: string) {
    if (isLoading.value || !hasMore.value) return;
    if (!currentSite.value) return;
    if (!orgId.value) {
      console.error("No organization ID available for asset request", {
        orgId: orgId.value,
        currentUser: userInfoStore.currentUser
      });
      toast.add({
        title: t("Asset.GetAssetFailed"),
        description: "Organization information is missing",
        color: "error",
        icon: "line-md:close-circle",
        progress: true,
        duration: 4000
      });
      return;
    }

    const searchParam = search !== undefined ? search : currentSearch.value;
    const orderParam = order !== undefined ? order : currentOrder.value;

    currentSearch.value = searchParam;
    currentOrder.value = orderParam;

    const requestId = createAssetRequestId(assetType);
    activeRequestId.value = requestId;
    beginLoading();

    try {
      await useTauriCoreInvoke("get_assets", {
        requestId,
        favorite: assetType === "favorite",
        query: {
          type: assetType === "favorite" ? undefined : assetType,
          offset: offset.value,
          limit: LIMIT,
          search: searchParam,
          order: orderParam
        }
      });
    } catch (e: any) {
      if (activeRequestId.value !== requestId) return;

      activeRequestId.value = null;
      hasMore.value = false;
      endLoading();
      toast.add({
        title: t("Asset.GetAssetFailed"),
        description: e?.message || "invoke get_assets failed",
        color: "error",
        icon: "line-md:close-circle",
        progress: true,
        duration: 4000
      });
    }
  }

  /**
   * @description Refresh asset data (reset state and refetch)
   * @param search
   * @param order
   */
  async function refreshAssets(search?: string, order?: string) {
    const searchParam = search !== undefined ? search : currentSearch.value;
    const orderParam = order !== undefined ? order : currentOrder.value;

    activeRequestId.value = null;
    isLoading.value = false;
    rawAssetsList.value = [];
    offset.value = 0;
    hasMore.value = true;
    totalCount.value = 0;
    await fetchNextPage(searchParam, orderParam);
  }

  /**
   * @description Listen for Tauri events
   */
  const listenTauriEvent = async () => {
    subscribeGetAssetsEvent.value = await useTauriEventListen("get-asset-success", (event) => {
      interface eventPayload {
        status: number
        data: AssetsResponse
        request_id: string
      }

      const resp = event.payload as eventPayload;

      if (!isLoading.value) return;
      if (!isActiveForCurrentRoute()) return;
      // The global event is received by every menu instance; only the currently active request is allowed to modify this menu's state.
      if (resp.request_id !== activeRequestId.value) return;

      activeRequestId.value = null;
      const pageResults = resp.data.results ?? [];
      const filtered = filterResultsByAssetType(pageResults);

      appendPageData(filtered, pageResults.length, resp.data.count);
      endLoading();
    });

    subscribeGetAssetFailedEvent.value = await useTauriEventListen("get-asset-failure", (event) => {
      interface eventPayload {
        status: number
        request_id: string
      }

      const payload = event.payload as eventPayload;

      if (!isLoading.value) return;
      if (!isActiveForCurrentRoute()) return;
      // A stale request's failure must not end a new request's loading, nor modify hasMore or login state.
      if (payload.request_id !== activeRequestId.value) return;

      activeRequestId.value = null;
      const status = payload.status;

      hasMore.value = false;

      if (status === 401) {
        toast.add({
          title: t("Login.LoginAuthenticationExpired"),
          description: t("Login.LoginAuthenticationExpiredDescription"),
          color: "error",
          icon: "line-md:close-circle",
          progress: true,
          duration: 4000
        });

        nextTick(() => {
          deleteUserData(currentSite.value);
        });
      }

      endLoading();
    });

    subscribeGetFavoriteAssetsEvent.value = await useTauriEventListen("get-favorite-assets-success", async (event) => {
      interface payLoadType {
        status: number
        data: string
      }

      const payload = event.payload as payLoadType;
      const favoriteAssets = JSON.parse(payload.data as string) as Array<{ asset: string }>;

      try {
        const ids = (favoriteAssets || []).map((x) => x?.asset).filter(Boolean) as string[];
        favoriteSet.value = new Set(ids);
      } catch (e) {
        console.warn("Failed to update favorites", e);
      }
    });
  };

  /**
   * @description Stop listening for Tauri events
   */
  const unListenTauriEvent = () => {
    subscribeGetAssetsEvent.value?.();
    subscribeGetAssetFailedEvent.value?.();
    subscribeGetFavoriteAssetsEvent.value?.();

    subscribeGetAssetsEvent.value = null;
    subscribeGetAssetFailedEvent.value = null;
    subscribeGetFavoriteAssetsEvent.value = null;
  };

  let unsubscribeSearch: (() => void) | null = null;
  let unsubscribeSetSort: (() => void) | null = null;
  let unsubscribeRefresh: (() => void) | null = null;
  let unsubscribeClearAssets: (() => void) | null = null;
  let unsubscribeAssetDetailUpdated: (() => void) | null = null;
  let unsubscribeAssetRenamed: (() => void) | null = null;
  let unsubscribeFavoriteChanged: (() => void) | null = null;

  const listenEventBusEvent = () => {
    const { on } = useEventBus();

    unsubscribeSetSort = on(
      "setSort",
      (sortOrder) => {
        refreshAssets(currentSearch.value, sortOrder as string);
      },
      false
    );

    unsubscribeRefresh = on(
      "refresh",
      () => {
        refreshAssets();
      },
      false
    );

    unsubscribeSearch = on(
      "search",
      (search) => {
        refreshAssets(search, currentOrder.value);
      },
      false
    );

    unsubscribeClearAssets = on("clearAssets", () => {
      hasMore.value = true;

      offset.value = 0;
      totalCount.value = 0;
      rawAssetsList.value = [];

      stopScrollListener?.();
      stopScrollListener = null;
    });

    unsubscribeAssetDetailUpdated = on(
      "assetDetailUpdated",
      (payload: { assetId: string, permedAccounts: PermedAccount[], permedProtocols: PermedProtocol[] }) => {
        const idx = rawAssetsList.value.findIndex((a) => a.id === payload.assetId);

        if (idx !== -1) {
          rawAssetsList.value[idx] = {
            ...rawAssetsList.value[idx],
            permedAccounts: payload.permedAccounts || [],
            permedProtocols: payload.permedProtocols || []
          } as RawAssetData;
        }

        // Record the assetID
        lastDetailAssetId.value = payload.assetId;

        nextTick(() => {
          getDetail.value = true;
        });
      },
      false
    );

    unsubscribeAssetRenamed = on(
      "assetRenamed",
      (payload: { assetId: string, name: string }) => {
        const idx = rawAssetsList.value.findIndex((a) => a.id === payload.assetId);

        if (idx !== -1) {
          rawAssetsList.value[idx] = {
            ...rawAssetsList.value[idx],
            name: payload.name
          } as RawAssetData;
        }
      },
      false
    );

    unsubscribeFavoriteChanged = on(
      "favoriteChanged",
      (payload: { assetId: string, favorite: boolean }) => {
        const set = new Set(favoriteSet.value);

        if (payload.favorite) {
          set.add(payload.assetId);
        } else {
          set.delete(payload.assetId);

          if (assetType === "favorite" && isActiveForCurrentRoute()) {
            refreshAssets();
          }
        }

        favoriteSet.value = set;
      },
      false
    );
  };

  const unListenEventBusEvent = () => {
    unsubscribeSearch?.();
    unsubscribeSetSort?.();
    unsubscribeRefresh?.();
    unsubscribeClearAssets?.();
    unsubscribeAssetDetailUpdated?.();
    unsubscribeAssetRenamed?.();
    unsubscribeFavoriteChanged?.();
  };

  onMounted(async () => {
    listenEventBusEvent();
    await listenTauriEvent();
  });

  onBeforeUnmount(() => {
    activeRequestId.value = null;
    unListenTauriEvent();
    unListenEventBusEvent();
    stopScrollListener?.();
    stopScrollListener = null;
  });

  return {
    hasMore,
    getDetail,
    isLoading,
    lastDetailAssetId,

    assetsData,
    isAppending,
    rawAssetsList,
    scrollbarStyles,
    isInitialLoading,
    appendSkeletonCount,

    fetchNextPage,
    refreshAssets
  };
};
