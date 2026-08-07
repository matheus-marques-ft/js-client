<script setup lang="ts">
import { useUserInfoStore } from "~/store/modules/userInfo";

const userInfoStore = useUserInfoStore();
const { loggedIn, currentUser } = storeToRefs(userInfoStore);

/**
 * @description Window drag
 * @param event mouse event
 */
const handleWindowDrag = async (event: MouseEvent) => {
  // Don't trigger drag if a button or other interactive element was clicked
  const target = event.target as HTMLElement;
  if (
    target.closest("button")
    || target.closest('[role="button"]')
    || target.closest("input")
    || target.closest("select")
  ) {
    return;
  }

  if (event.button !== 0) return;

  try {
    const windows = await useTauriWindowGetAllWindows();

    windows.forEach((window) => {
      if (window.label === "main") {
        window.startDragging();
      }
    });
  } catch (error) {
    console.error(error);
  }
};

const shouldShowOrganizationSelector = computed(() => {
  if (!loggedIn.value) {
    return false;
  }

  return currentUser.value?.xpackLicenseValid !== false;
});
</script>

<template>
  <div class="header-bg flex items-center justify-between pl-4 h-13" @mousedown="handleWindowDrag">
    <section class="flex items-center h-full">
      <div v-if="shouldShowOrganizationSelector">
        <HeaderOrganizationSelector />
      </div>
    </section>

    <HeaderActionButtons />
  </div>
</template>
