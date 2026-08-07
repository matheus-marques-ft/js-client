<script setup lang="ts">
import type { AssetItem, AssetPageType, ConnectionInfo, PermedAccount, PermedProtocol } from "~/types/index";
import EditForm from "~/components/EditForm/editForm.vue";

const props = defineProps<{
  assetType?: AssetPageType
}>();

const { t, locale } = useI18n();
const { getAssetDetail } = useAssetAction();

const open = ref(false);
const currentAsset = ref<AssetItem | null>(null);

const draftProtocol = ref<string>("");
const draftAccount = ref<string>("");
const draftManualUsername = ref<string>("");
const draftManualPassword = ref<string>("");
const draftDynamicPassword = ref<string>("");
const draftRememberSecret = ref<boolean>(false);
const draftConnectMethod = ref<string>("");

let pendingResolve: ((info: any) => void) | null = null;
let pendingReject: ((reason?: any) => void) | null = null;

const modalTitle = computed(() => {
  const name = currentAsset.value?.name || "";
  return `${t("EditModal.ModifyConnectionInfo")} - ${name}`;
});

/**
 * @description Initialize the Form
 * @param asset
 */
const initDraft = (asset: AssetItem, preferredProtocol?: string) => {
  const saved: ConnectionInfo | undefined = asset.savedConnection;

  const protocols = sortPermedProtocols(asset.permedProtocols || ([] as PermedProtocol[]));
  const accounts = asset.permedAccounts || ([] as PermedAccount[]);

  // Protocol default: saved protocol -> first protocol -> empty
  draftProtocol.value = protocols.some((protocol) => protocol.name === preferredProtocol)
    ? preferredProtocol!
    : saved?.protocol || protocols[0]?.name || "";

  // Account default: saved username -> first managed account -> dynamic account (@USER) -> manual input (@INPUT) -> empty
  if (saved?.username) {
    draftAccount.value = saved!.username;
  } else {
    const hosted = accounts.find((a) => a?.alias && !a.alias.startsWith("@"));

    if (hosted) {
      draftAccount.value = hosted.name;
    } else {
      const dynamic = accounts.find((a) => a.alias === "@USER");
      const manual = accounts.find((a) => a.alias === "@INPUT");

      if (dynamic) {
        draftAccount.value
          = locale.value === "zh" ? `${dynamic.name}(${dynamic.username})` : `Dynamic user(${dynamic.username})`;
      } else if (manual) {
        draftAccount.value = locale.value === "zh" ? manual.name : "Manual input";
      } else {
        draftAccount.value = "";
      }
    }
  }

  draftManualUsername.value = saved?.manualUsername || "";
  draftManualPassword.value = saved?.manualPassword || "";
  draftDynamicPassword.value = saved?.dynamicPassword || "";
  draftRememberSecret.value = saved?.rememberSecret || false;
  draftConnectMethod.value
    = !preferredProtocol || preferredProtocol === saved?.protocol ? saved?.connectMethod || "" : "";
};

/**
 * @description Assemble the connection info
 */
const normalizeProtocols = () => {
  return sortPermedProtocols(currentAsset.value?.permedProtocols || [])
    .map((p) => (p?.name ? p.name.trim() : ""))
    .filter((name) => name.length > 0);
};

const buildConnectionInfo = () => {
  let accountMode: "hosted" | "dynamic" | "manual" | "anonymous" = "hosted";
  let normalizedAccount = draftAccount.value || "";
  let accountId: string | undefined;

  const v = draftAccount.value || "";

  if (v === "手动输入" || v === "Manual input") accountMode = "manual";
  if (v.includes("@ANON") || v === "匿名账号" || v === "Anonymous") {
    accountMode = "anonymous";
    normalizedAccount = "@ANON";
  }
  if (v.includes("同名账号") || v.includes("Dynamic user")) {
    accountMode = "dynamic";

    const accs = currentAsset.value?.permedAccounts || [];
    const dynamicAcc = accs.find((a) => a.alias === "@USER");

    if (dynamicAcc) normalizedAccount = dynamicAcc.name;
    else normalizedAccount = v.replace(/\(.+\)/, "");
  }

  if (accountMode === "hosted") {
    const accs = currentAsset.value?.permedAccounts || [];
    const matched = accs.find(
      (a) => a.name === normalizedAccount || a.username === normalizedAccount || a.alias === normalizedAccount
    );
    accountId = matched?.id;
  }

  return {
    protocol: draftProtocol.value || "",
    account: normalizedAccount,
    accountId,
    accountMode,
    manualUsername: draftManualUsername.value || "",
    manualPassword: draftManualPassword.value || "",
    dynamicPassword: draftDynamicPassword.value || "",
    rememberSecret: !!draftRememberSecret.value,
    connectMethod: draftConnectMethod.value || "",
    availableProtocols: normalizeProtocols()
  };
};

/**
 * @description Close the modal
 */
const close = () => {
  open.value = false;
  currentAsset.value = null;
};

/**
 * @description Click confirm
 */
const onConfirm = () => {
  const info = buildConnectionInfo();

  pendingResolve?.(info);
  pendingResolve = null;
  pendingReject = null;
  close();
};

/**
 * @description Click cancel
 */
const onCancel = () => {
  pendingReject?.("cancelled");
  pendingResolve = null;
  pendingReject = null;
  close();
};

async function ensureDetails(asset: AssetItem) {
  const noAccounts = !asset.permedAccounts || asset.permedAccounts.length === 0;
  const noProtocols = !asset.permedProtocols || asset.permedProtocols.length === 0;

  if (!noAccounts && !noProtocols) return asset;

  const detailsReady = new Promise<AssetItem>((resolve) => {
    const unsubscribe = useEventBus().once(
      "assetDetailUpdated",
      (payload: { assetId: string, permedAccounts: PermedAccount[], permedProtocols: PermedProtocol[] }) => {
        if (payload.assetId !== asset.id) return;

        currentAsset.value = {
          ...(currentAsset.value || asset),
          permedAccounts: payload.permedAccounts || [],
          permedProtocols: payload.permedProtocols || []
        } as AssetItem;

        resolve(currentAsset.value!);
      }
    );

    void unsubscribe;
  });

  await getAssetDetail(asset.id);
  const updated = await detailsReady;
  return updated;
}

/**
 * @description Open the Modal
 * @param asset
 */
async function openModal(asset: AssetItem, preferredProtocol?: string): Promise<any> {
  currentAsset.value = asset;
  await ensureDetails(asset);
  initDraft(currentAsset.value!, preferredProtocol);
  open.value = true;

  return new Promise((resolve, reject) => {
    pendingResolve = resolve;
    pendingReject = reject;
  });
}

defineExpose({ open: openModal, close });
</script>

<template>
  <Modal
    :open="open"
    :title="modalTitle"
    :description="t('EditModal.Description')"
    @confirm="onConfirm"
    @update:open="(v) => (v ? (open = true) : onCancel())"
  >
    <EditForm
      v-if="currentAsset"
      v-model:protocol="draftProtocol"
      v-model:account="draftAccount"
      v-model:manual-username="draftManualUsername"
      v-model:manual-password="draftManualPassword"
      v-model:dynamic-password="draftDynamicPassword"
      v-model:remember-secret="draftRememberSecret"
      v-model:connect-method="draftConnectMethod"
      :accounts="currentAsset.permedAccounts || []"
      :protocols="currentAsset.permedProtocols || []"
      :asset-type="props.assetType"
    />
  </Modal>
</template>
