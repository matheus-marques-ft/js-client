import type { AssetItem } from "~/types/index";

export function useAssetManagement() {
  const { getAssetDetail } = useAssetAction();

  // Selected card state
  const selectedCardIndex = ref<number | null>(null);
  const currentSelectedCardInfo = ref<AssetItem | null>(null);

  /**
   * Handle card click
   */
  const handleCardClick = (index: number, e: MouseEvent) => {
    e.stopPropagation();
    selectedCardIndex.value = index;
  };

  /**
   * Clear the selected card
   */
  const clearSelectedCard = () => {
    selectedCardIndex.value = null;
  };

  /**
   * Set the currently selected asset info
   */
  const setCurrentAsset = (asset: AssetItem) => {
    currentSelectedCardInfo.value = asset;
  };

  /**
   * Handle fetching asset details
   */
  const handleAssetDetail = (assetId: string) => {
    getAssetDetail(assetId);
  };

  return {
    // State
    selectedCardIndex,
    currentSelectedCardInfo,

    // Methods
    handleCardClick,
    clearSelectedCard,
    setCurrentAsset,
    handleAssetDetail
  };
}
