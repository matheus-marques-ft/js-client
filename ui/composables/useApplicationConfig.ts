import type { AppConfigType } from "~/types";

export const useApplicationConfig = () => {
  const { t } = useI18n();
  const toast = useToast();
  const { setAppConfig, appConfig, hydrationPromise } = useSettingManager();

  const isValidAppConfig = (cfg: any): cfg is AppConfigType => {
    return (
      !!cfg
      && Array.isArray(cfg.terminal)
      && Array.isArray(cfg.remotedesktop)
      && Array.isArray(cfg.filetransfer)
      && Array.isArray(cfg.databases)
    );
  };

  const formatExecutableNotFound = (raw: string) => {
    const path = raw.replace(/^\s*executable not found:\s*/i, "").trim();
    return path ? `${t("Setting.ExecutableNotFound")}\n${path}` : t("Setting.ExecutableNotFound");
  };

  const getConfig = async () => {
    const config = await useTauriCoreInvoke("get_config");

    if (config) {
      setAppConfig(config as AppConfigType);
    }
  };

  onMounted(async () => {
    // Only fetch config in the main window; other windows read the result directly
    const cur = await useTauriWebviewWindowGetCurrentWebviewWindow();

    if (cur && cur.label !== "main") {
      if (hydrationPromise.value) {
        try {
          await hydrationPromise.value;
        } catch {}
      }

      if (!isValidAppConfig(appConfig.value)) {
        await getConfig();
      }

      return;
    }

    await getConfig();
  });

  const selectClient = async (category: keyof AppConfigType, protocol: string, name: string, enabled = true) => {
    try {
      const updated = await useTauriCoreInvoke("update_config_selection", {
        category,
        protocol,
        name,
        enabled
      });

      if (updated) {
        setAppConfig(updated as AppConfigType);
      }
    } catch (error) {
      const message = String(error ?? "");
      const description = message.toLowerCase().includes("executable not found")
        ? formatExecutableNotFound(message)
        : message || t("Common.OperationFailed");

      toast.add({
        title: t("Setting.EnableFailed"),
        description,
        color: "error",
        icon: "line-md:close-circle",
        actions: [
          {
            label: t("Common.Copy"),
            icon: "i-lucide-copy",
            color: "neutral",
            variant: "soft",
            onClick: () => {
              void useTauriClipboardManagerWriteText(`${t("Setting.EnableFailed")}\n${description}`);
            }
          }
        ],
        progress: true,
        duration: 4000
      });

      // Refresh so path_exists reflects the current filesystem state.
      await getConfig();
    }
  };

  return {
    appConfig,
    selectClient
  };
};
