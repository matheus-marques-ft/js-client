import type { ConnectionInfo, PermOrgItem, RdpGraphics, UserData } from "~/types/index";
import { useConnectMethods } from "~/composables/useConnectMethods";

export type SiteUserData = UserData & {
  language?: string
  rdpClientOption?: RdpGraphics
  connectionInfoMap?: Record<string, ConnectionInfo>
};

// This would really be better named accountInfoStore
export const useUserInfoStore = defineStore(
  "userInfo",
  () => {
    const currentSite = ref("");
    const loggedIn = ref(false);

    const currentUser = ref<UserData | null>(null);
    const currentOrganizations = ref<PermOrgItem[]>([]);
    const userMap = ref<Record<string, SiteUserData>>({});
    const currentRdpClientOption = ref<RdpGraphics>({});
    const currentConnectionInfoMap = ref<Record<string, ConnectionInfo>>({});

    const hasUser = computed(() => Object.keys(userMap.value).length > 0);
    const orgId = computed(() => currentUser.value?.org?.id || "");

    /**
     * @description Sync the current frontend session to the Rust request layer
     * @param site
     * @param userData
     */
    const syncApiSession = (site: string, userData: UserData) => {
      if (!site || !userData.bearerToken || !userData.org?.id) return;

      void useTauriCoreInvoke("set_api_session", {
        sessionKey: site,
        origin: site,
        bearerToken: userData.bearerToken,
        orgId: userData.org.id
      }).catch((error) => {
        console.error("sync api session failed", error);
      });
    };

    watch(
      [currentSite, currentUser],
      ([site, userData]) => {
        if (site && userData) syncApiSession(site, userData);
      },
      { immediate: true }
    );

    /**
     * @description Set the user's login state
     * @param l
     */
    const setUserLoggedIn = (l: boolean) => {
      loggedIn.value = l;
    };

    /**
     * @description Get user data
     * @param site
     */
    const getUserData = (site: string) => {
      if (!(site in userMap.value)) {
        return null;
      }

      return userMap.value[site];
    };

    /**
     * @description Set user data
     * @param site
     * @param userData
     */
    const setUserData = (site: string, userData: UserData) => {
      const next = userData as SiteUserData;

      userMap.value[site] = next;
      currentUser.value = next;
      currentSite.value = site;
      syncApiSession(site, next);

      // Initialize the current site's connection info map and RDP client options
      currentConnectionInfoMap.value = next.connectionInfoMap || {};
      currentRdpClientOption.value = next.rdpClientOption || {};

      // Fetch connect methods after login
      const { fetchConnectMethods } = useConnectMethods();
      nextTick(async () => {
        try {
          await fetchConnectMethods();
        } catch (error) {
          console.debug("Failed to fetch connect methods on login:", error);
        }
      });
    };

    /**
     * @description Delete user data
     * @param site
     */
    const deleteUserData = (site: string) => {
      // Immediately request cleanup of its cookie when logging out of the current site
      useTauriCoreInvoke("logout", {
        name: "main",
        site
      });

      if (!(site in userMap.value)) {
        return;
      }

      delete userMap.value[site];

      // Switch to the next user if any remain
      if (hasUser.value) {
        const nextUser = Object.values(userMap.value)[0] as SiteUserData | undefined;

        if (nextUser) {
          userMap.value[nextUser.site] = nextUser;
          currentUser.value = nextUser;
          currentSite.value = nextUser.site;
          syncApiSession(nextUser.site, nextUser);

          // Sync the connection info map and RDP client options
          currentConnectionInfoMap.value = nextUser.connectionInfoMap || {};
          currentRdpClientOption.value = nextUser.rdpClientOption || {};
          currentOrganizations.value = nextUser.availableOrgs || [];

          loggedIn.value = true;

          nextTick(() => {
            useEventBus().emit("refresh", undefined);
          });
        }
      } else {
        currentSite.value = "";
        loggedIn.value = false;
        currentUser.value = null;

        userMap.value = {};
        currentRdpClientOption.value = {};
        currentConnectionInfoMap.value = {};
        currentOrganizations.value = [];

        nextTick(() => {
          useEventBus().emit("clearAssets", undefined);
        });
      }
    };

    /**
     * @description Set the current site
     * @param site
     */
    const setCurrentSite = (site: string) => {
      currentSite.value = site;

      // Also update the current organization list when switching sites
      const userData = getUserData(site);

      if (userData) {
        userMap.value[site] = userData as SiteUserData;
        currentUser.value = userData as SiteUserData;
        currentOrganizations.value = (userData as SiteUserData).availableOrgs || [];
        syncApiSession(site, userData);

        // Sync the current site's connection info map and RDP client options
        currentConnectionInfoMap.value = (userData as SiteUserData).connectionInfoMap || {};
        currentRdpClientOption.value = (userData as SiteUserData).rdpClientOption || {};
      } else {
        currentConnectionInfoMap.value = {};
        currentRdpClientOption.value = {};
      }
    };

    /**
     * @description Set the current organization list
     * @param orgs
     */
    const setOrganizations = (orgs: PermOrgItem[]) => {
      currentOrganizations.value = orgs;

      if (currentUser.value && currentSite.value) {
        const updatedUserData = {
          ...currentUser.value,
          availableOrgs: orgs
        };

        userMap.value[currentSite.value] = updatedUserData as SiteUserData;
        currentUser.value = updatedUserData;
      }
    };

    /**
     * @description Set the current organization
     * @param org
     */
    const setCurrentOrg = (org: PermOrgItem) => {
      if (!currentUser.value || !currentSite.value) {
        console.error("No current user or site when setting organization");
        return;
      }

      const updatedUserData = {
        ...currentUser.value,
        org
      };

      currentUser.value = updatedUserData as UserData;
      userMap.value[currentSite.value] = updatedUserData as SiteUserData;

      void useTauriCoreInvoke("set_api_org", {
        orgId: org.id
      }).catch((error) => {
        console.error("sync api org failed", error);
      });
    };

    /**
     * @description Set the user's connection info
     * @param connectionInfo
     */
    const setConnectionInfoToUser = (connectionInfo: ConnectionInfo) => {
      if (!currentUser.value) {
        return;
      }

      currentUser.value.connectionInfo = connectionInfo;
    };

    /**
     * @description Get an asset's connection info
     * @param assetId asset ID
     */
    const getConnectionInfoForAsset = (assetId: string) => {
      if (!currentSite.value) return null;

      const siteData = userMap.value[currentSite.value];
      return siteData?.connectionInfoMap?.[assetId] || null;
    };

    /**
     * @description Set an asset's connection info
     * @param assetId
     * @param connectionInfo
     */
    const setConnectionInfoForAsset = (assetId: string, connectionInfo: ConnectionInfo) => {
      if (!currentSite.value) return;
      const site = currentSite.value;
      const siteData = userMap.value[site];

      if (!siteData) return;

      if (!siteData.connectionInfoMap) {
        siteData.connectionInfoMap = {};
      }

      const existing = siteData.connectionInfoMap[assetId];
      const incomingProtocols = (connectionInfo.availableProtocols || [])
        .map((p) => (typeof p === "string" ? p.trim() : ""))
        .filter((p) => p.length > 0);

      const mergedProtocols
        = incomingProtocols.length > 0 ? Array.from(new Set(incomingProtocols)) : existing?.availableProtocols;

      siteData.connectionInfoMap[assetId] = {
        ...(existing || {}),
        ...connectionInfo,
        ...(mergedProtocols && mergedProtocols.length > 0 ? { availableProtocols: mergedProtocols } : {})
      };

      currentConnectionInfoMap.value = { ...siteData.connectionInfoMap };
    };

    /**
     * @description Set the RDP client option
     * @param rdpClientOption
     */
    const setRdpClientOption = (rdpClientOption: RdpGraphics) => {
      currentRdpClientOption.value = rdpClientOption;

      // Sync into the current site's user data, so it persists / restores after switching sites
      if (currentSite.value && userMap.value[currentSite.value]) {
        const site = currentSite.value;
        const siteData = userMap.value[site] as SiteUserData;

        userMap.value[site] = {
          ...siteData,
          rdpClientOption
        } as SiteUserData;
      }
    };

    return {
      orgId,
      userMap,
      loggedIn,
      currentSite,
      currentUser,
      currentOrganizations,
      currentRdpClientOption,
      currentConnectionInfoMap,

      setUserData,
      getUserData,
      setCurrentOrg,
      setCurrentSite,
      deleteUserData,
      setUserLoggedIn,
      setOrganizations,
      setRdpClientOption,
      setConnectionInfoToUser,
      getConnectionInfoForAsset,
      setConnectionInfoForAsset
    };
  },
  {
    persist: {
      key: "userInfo",
      storage: localStorage,
      pick: [
        "userMap",
        "loggedIn",
        "currentUser",
        "currentSite",
        "currentOrganizations",
        "currentRdpClientOption",
        "currentConnectionInfoMap"
      ]
    }
  }
);
