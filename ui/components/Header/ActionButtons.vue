<script setup lang="ts">
import type { DropdownMenuItem } from "@nuxt/ui";
import type { ActionItem } from "~/types/index";

import { LogicalPosition } from "@tauri-apps/api/dpi";

const { t, locale } = useI18n();
const { isLinux, isMacOS } = usePlatform();
const { layouts, sort, setSort, setLayouts } = useSettingManager();

const localePath = useLocalePath();

// Common button config
const commonButtonProps = {
  size: "sm" as const,
  variant: "ghost" as const,
  color: "neutral" as const
};

const showCustomWindowControls = computed(() => !isMacOS.value);

// Window control button config
const windowControlButtons = computed(() => {
  // Show window control buttons on Windows
  return [
    {
      key: "minimize",
      iconName: "i-lucide-minus",
      tooltipLabel: t("ToolTips.Minimize"),
      onClick: async () => {
        await useTauriCoreInvoke("minimize_window");
      }
    },
    {
      key: "maximize",
      iconName: "i-lucide-square",
      tooltipLabel: t("ToolTips.Maximize"),
      onClick: async () => {
        await useTauriCoreInvoke("toggle_maximize_window");
      }
    },
    {
      key: "close",
      iconName: "i-lucide-x",
      tooltipLabel: t("ToolTips.Close"),
      onClick: async () => {
        await useTauriCoreInvoke("close_window");
      }
    }
  ];
});

// Get the style class for window control buttons
const getWindowControlButtonClass = (buttonKey: string) => {
  const isLinuxStyle = isLinux.value;
  const baseClass = isLinuxStyle
    ? "h-9 w-9 rounded-full p-1 flex items-center justify-center transition-colors"
    : "rounded-none w-12 h-13 p-1 flex items-center justify-center";

  switch (buttonKey) {
    case "minimize":
      return isLinuxStyle ? `${baseClass} hover:bg-black/6 dark:hover:bg-white/10` : `${baseClass} `;
    case "maximize":
      return isLinuxStyle ? `${baseClass} hover:bg-black/6 dark:hover:bg-white/10` : `${baseClass} `;
    case "close":
      return isLinuxStyle
        ? `${baseClass} hover:bg-red-500/12 hover:text-red-600 dark:hover:bg-red-500/18 dark:hover:text-red-300`
        : `${baseClass} hover:bg-red-500 hover:text-white active:bg-red-600`;
    default:
      return baseClass;
  }
};

// Button action logic moved over from the Operation component
const actionItems = computed<ActionItem[]>(() => [
  {
    key: "refresh",
    type: "action",
    iconName: "i-lucide-refresh-ccw",
    tooltipLabel: t("ToolTips.Refresh"),
    onClick: () => {
      useEventBus().emit("refresh", undefined);
    }
  },
  {
    key: "sort",
    type: "select",
    iconName: "i-lucide-arrow-down-wide-narrow",
    tooltipLabel: t("ToolTips.Sort"),
    selectItems: [
      {
        icon: "i-lucide-arrow-down-a-z",
        label: t("Sort.A-z"),
        value: "name",
        type: "checkbox" as const,
        checked: sort.value === "name",
        onUpdateChecked: (checked: boolean) => {
          if (checked) {
            setSort("name");
          }
        }
      },
      {
        icon: "i-lucide-arrow-up-z-a",
        label: t("Sort.Z-A"),
        value: "-name",
        type: "checkbox" as const,
        checked: sort.value === "-name",
        onUpdateChecked: (checked: boolean) => {
          if (checked) {
            setSort("-name");
          }
        }
      },
      {
        type: "separator" as const
      },
      {
        icon: "i-lucide-calendar-arrow-down",
        label: t("Sort.NewestToOldest"),
        value: "-date_updated",
        type: "checkbox" as const,
        checked: sort.value === "-date_updated",
        onUpdateChecked: (checked: boolean) => {
          if (checked) {
            setSort("-date_updated");
          }
        }
      },
      {
        icon: "i-lucide-calendar-arrow-up",
        label: t("Sort.OldestToNewest"),
        value: "date_updated",
        type: "checkbox" as const,
        checked: sort.value === "date_updated",
        onUpdateChecked: (checked: boolean) => {
          if (checked) {
            setSort("date_updated");
          }
        }
      }
    ] as DropdownMenuItem[]
  },
  {
    key: "layout",
    type: "select",
    iconName: "i-lucide-layout-grid",
    tooltipLabel: t("ToolTips.Layout"),
    selectItems: [
      {
        icon: "i-lucide-grid-2x2",
        label: t("Layout.Grid"),
        value: "grid",
        type: "checkbox" as const,
        checked: layouts.value === "grid",
        onUpdateChecked: (checked: boolean) => {
          if (checked) {
            setLayouts("grid");
          }
        }
      },
      {
        icon: "i-lucide-table-of-contents",
        label: t("Layout.Table"),
        value: "table",
        type: "checkbox" as const,
        checked: layouts.value === "table",
        onUpdateChecked: (checked: boolean) => {
          if (checked) {
            setLayouts("table");
          }
        }
      }
    ] as DropdownMenuItem[]
  },
  {
    key: "settings",
    type: "action",
    iconName: "i-lucide-settings",
    tooltipLabel: t("ToolTips.Settings"),
    onClick: async () => {
      const label = "secondary";
      const existing = await useTauriWebviewWindowWebviewWindow.getByLabel(label);

      // Bring it to the front if it's already open
      if (existing) {
        try {
          if (await existing.isMinimized()) {
            await existing.unminimize();
          }

          if (!(await existing.isVisible())) {
            await existing.show();
          }

          await existing.setFocus();
        } catch (e) {
          console.error("focus settings window failed", e);
        }
        return;
      }

      // Create the window directly

      const useNativeWindowFrame = isMacOS.value;
      new useTauriWebviewWindowWebviewWindow(label, {
        title: t("Common.ConnectionSettings"),
        url: localePath({ path: "/setting" }),
        height: 675,
        minWidth: 930,
        minHeight: 675,
        maxHeight: 675,
        hiddenTitle: true,
        titleBarStyle: "overlay",
        trafficLightPosition: new LogicalPosition(10, 22),
        decorations: useNativeWindowFrame,
        shadow: useNativeWindowFrame
      });
    }
  }
]);
</script>

<template>
  <section class="flex items-center h-full">
    <div class="flex items-center mr-3 gap-2">
      <template v-for="action of actionItems" :key="action.iconName">
        <template v-if="action.type === 'action'">
          <UButton :icon="action.iconName" v-bind="commonButtonProps" @click="action.onClick" />
        </template>

        <template v-else>
          <UDropdownMenu
            arrow
            size="sm"
            :items="action.selectItems"
            :ui="{
              content: locale === 'en' ? 'w-42' : 'w-48'
            }"
          >
            <UButton
              :icon="action.iconName"
              v-bind="commonButtonProps"
              @click="() => console.log('Dropdown button clicked:', action.key)"
            />
          </UDropdownMenu>
        </template>
      </template>
    </div>

    <!-- Window control buttons -->
    <div v-if="showCustomWindowControls" class="flex items-center" :class="isLinux ? 'gap-1.5' : ''">
      <template v-for="button of windowControlButtons" :key="button.key">
        <UButton
          size="sm"
          variant="ghost"
          color="neutral"
          :icon="button.iconName"
          :class="getWindowControlButtonClass(button.key)"
          :title="button.tooltipLabel"
          @click="button.onClick"
        />
      </template>
    </div>
  </section>
</template>
