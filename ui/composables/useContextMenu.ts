import type { AssetItem } from "~/types/index";

export function useContextMenu() {
  // Context menu state
  const contextMenuVisible = ref(false);
  const contextMenuPosition = ref({ x: 0, y: 0 });
  const contextMenuAsset = ref<AssetItem | null>(null);

  /**
   * Show the context menu
   */
  const showContextMenu = (asset: AssetItem, event?: MouseEvent) => {
    contextMenuAsset.value = asset;

    if (event) {
      const menuWidth = 200; // menu width
      const menuHeight = 200; // menu height
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      let x = event.clientX;
      let y = event.clientY;

      // Check whether this came from a table button (by inspecting the target element)
      const target = event.target as HTMLElement;
      const isTableButton
        = target?.hasAttribute("data-table-context-button")
          || target?.closest("[data-table-context-button]")
          || target?.closest(".UTable");

      // If this is a table button, prefer showing it on the left
      if (isTableButton) {
        x = event.clientX - menuWidth;
        // Show it on the right if there isn't enough room on the left
        if (x < 10) {
          x = event.clientX;
        }
      } else {
        // For other cases (e.g. right-click menu), show it on the left if it would overflow the right edge
        if (x + menuWidth > viewportWidth) {
          x = event.clientX - menuWidth;
        }
      }

      // Adjust upward if the menu would overflow the bottom edge
      if (y + menuHeight > viewportHeight) {
        y = event.clientY - menuHeight;
      }

      // Make sure it doesn't overflow the left or top edges
      x = Math.max(10, x);
      y = Math.max(10, y);

      contextMenuPosition.value = { x, y };
    }

    contextMenuVisible.value = true;
  };

  /**
   * Hide the context menu
   */
  const hideContextMenu = () => {
    contextMenuVisible.value = false;
    contextMenuAsset.value = null;
  };

  return {
    // State
    contextMenuVisible,
    contextMenuPosition,
    contextMenuAsset,

    // Methods
    showContextMenu,
    hideContextMenu
  };
}
