import type { Event } from "@tauri-apps/api/event";
import type { Theme } from "@tauri-apps/api/window";
import { nextTick } from "vue";

export const useThemeAdapter = () => {
  const currentOSTheme = ref<Theme>("light");

  const uiColorMode = useColorMode();
  const {
    theme: userTheme,
    themeMode,
    followSystem,
    hydrationPromise,
    isHydrated,
    setTheme,
    setThemeMode,
    setFollowSystem
  } = useSettingManager();

  const currentWindow = useTauriWindowGetCurrentWindow();

  const broadcastThemeChange = (mode: Theme | "withSystem") => {
    void useTauriEventEmit("theme-changed", { mode }).catch((err) => {
      console.debug("broadcast theme change failed", err);
    });
  };

  const waitHydration = async () => {
    if (isHydrated.value) return;

    // Wait for useSettingManager to finish initializing
    if (!hydrationPromise.value) {
      await nextTick();
    }

    const promise = hydrationPromise.value;

    if (promise) {
      try {
        await promise;
      } catch (err) {
        console.error("wait hydration failed", err);
      }
    }
  };

  /**
   * @description The app defaults to the OS theme on first load
   */
  const initialTheme = async () => {
    await waitHydration();

    const savedMode = (themeMode.value || "") as string;
    const modeIsWithSystem = savedMode === "withSystem";
    const modeIsManual = savedMode === "dark" || savedMode === "light";

    const follow = modeIsWithSystem ? true : modeIsManual ? false : followSystem.value;
    const savedTheme = (modeIsManual ? savedMode : userTheme.value) as Theme | "";

    const osTheme = await currentWindow.theme();

    if (!osTheme) {
      if (savedTheme) {
        uiColorMode.preference = savedTheme;
      }
      return;
    }

    currentOSTheme.value = osTheme;

    if (follow) {
      if (themeMode.value !== "withSystem") {
        setThemeMode("withSystem");
      }
      if (!followSystem.value) {
        setFollowSystem(true);
      }
      uiColorMode.preference = osTheme;
      setTheme(osTheme);
      return;
    }

    if (followSystem.value) {
      setFollowSystem(false);
    }

    if (savedTheme) {
      uiColorMode.preference = savedTheme;
      return;
    }

    uiColorMode.preference = osTheme;
    setTheme(osTheme);
  };

  const manualSetTheme = (theme: Theme) => {
    setFollowSystem(false);
    setThemeMode(theme as any);
    uiColorMode.preference = theme;
    setTheme(theme);
    broadcastThemeChange(theme);
  };

  const enableFollowSystem = async () => {
    setFollowSystem(true);
    setThemeMode("withSystem");

    const osTheme = (await currentWindow.theme()) || currentOSTheme.value;

    if (osTheme) {
      currentOSTheme.value = osTheme;
      uiColorMode.preference = osTheme;
      setTheme(osTheme);
    }

    broadcastThemeChange("withSystem");
  };

  const applyThemePreference = (theme: Theme) => {
    uiColorMode.preference = theme;
  };

  const applySystemThemePreference = async () => {
    const osTheme = (await currentWindow.theme()) || currentOSTheme.value;

    if (osTheme) {
      currentOSTheme.value = osTheme;
      uiColorMode.preference = osTheme;
    }
  };

  const listenOSThemeChange = () => {
    // Listen for OS theme changes
    currentWindow.onThemeChanged((event: Event<Theme>) => {
      currentOSTheme.value = event.payload;

      if (themeMode.value === "withSystem" || followSystem.value) {
        uiColorMode.preference = event.payload;
        setTheme(event.payload);
      }
    });
  };

  return {
    userTheme,
    themeMode,
    followSystem,

    initialTheme,
    manualSetTheme,
    enableFollowSystem,
    listenOSThemeChange,
    applyThemePreference,
    applySystemThemePreference
  };
};
