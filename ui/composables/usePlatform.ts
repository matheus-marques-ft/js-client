/**
 * Platform detection composable
 * Provides cross-platform platform-detection functionality
 */
export const usePlatform = () => {
  const platform = ref<string>("unknown");
  const isLoading = ref(true);

  // Computed: check whether this is macOS
  const isMacOS = computed(() => platform.value === "darwin" || platform.value === "macos");

  // Computed: check whether this is Windows
  const isWindows = computed(() => platform.value === "win32" || platform.value === "windows");

  // Computed: check whether this is Linux
  const isLinux = computed(() => platform.value === "linux");

  const detectPlatformFromUserAgent = () => {
    if (typeof navigator === "undefined") return "unknown";

    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes("windows")) return "win32";
    if (ua.includes("mac os") || ua.includes("macintosh")) return "darwin";
    if (ua.includes("linux")) return "linux";

    return "unknown";
  };

  // Get platform info; fall back to browser-environment detection if the Tauri OS plugin isn't available yet
  const getPlatform = async () => {
    try {
      isLoading.value = true;
      const currentPlatform = await useTauriOsPlatform();
      platform.value = currentPlatform || detectPlatformFromUserAgent();
    } catch {
      platform.value = detectPlatformFromUserAgent();
    } finally {
      isLoading.value = false;
    }
  };

  // Automatically fetch platform info when the component mounts
  onMounted(() => {
    getPlatform();
  });

  return {
    isMacOS,
    isLinux,
    isWindows,
    getPlatform,
    platform: readonly(platform),
    isLoading: readonly(isLoading)
  };
};
