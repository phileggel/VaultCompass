import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { CreateAssetDTO, UpdateAssetDTO } from "@/bindings";
import { logger } from "@/lib/logger";
import { useSnackbar } from "@/lib/snackbarStore";
import { useAppStore } from "@/lib/store";
import { assetGateway } from "./gateway";

export function useAssets() {
  const { t } = useTranslation();
  const showSnackbar = useSnackbar();
  const assets = useAppStore((state) => state.assets);
  const loading = useAppStore((state) => state.isLoadingAssets);
  const fetchError = useAppStore((state) => state.assetsError);
  const fetchAssets = useAppStore((state) => state.fetchAssets);

  const addAsset = useCallback(
    async (dto: CreateAssetDTO) => {
      try {
        const res = await assetGateway.createAsset(dto);
        if (res.status === "ok") {
          await fetchAssets();
          showSnackbar(t("asset.success_created"), "success");
          return { data: res.data, error: null };
        }
        return { data: null, error: res.error };
      } catch (e) {
        logger.error("Failed to add asset", { error: e });
        return { data: null, error: String(e) };
      }
    },
    [fetchAssets, showSnackbar, t],
  );

  const updateAsset = useCallback(
    async (dto: UpdateAssetDTO) => {
      try {
        const res = await assetGateway.updateAsset(dto);
        if (res.status === "ok") {
          await fetchAssets();
          showSnackbar(t("asset.success_updated"), "success");
          return { data: res.data, error: null };
        }
        return { data: null, error: res.error };
      } catch (e) {
        logger.error("Failed to update asset", { error: e });
        return { data: null, error: String(e) };
      }
    },
    [fetchAssets, showSnackbar, t],
  );

  const archiveAsset = useCallback(
    async (id: string) => {
      try {
        const res = await assetGateway.archiveAsset(id);
        if (res.status === "ok") {
          await fetchAssets();
          showSnackbar(t("asset.success_archived"), "success");
          return { error: null };
        }
        return { error: res.error };
      } catch (e) {
        logger.error("Failed to archive asset", { error: e });
        return { error: String(e) };
      }
    },
    [fetchAssets, showSnackbar, t],
  );

  const unarchiveAsset = useCallback(
    async (id: string) => {
      try {
        const res = await assetGateway.unarchiveAsset(id);
        if (res.status === "ok") {
          await fetchAssets();
          showSnackbar(t("asset.success_unarchived"), "success");
          return { error: null };
        }
        return { error: res.error };
      } catch (e) {
        logger.error("Failed to unarchive asset", { error: e });
        return { error: String(e) };
      }
    },
    [fetchAssets, showSnackbar, t],
  );

  const deleteAsset = useCallback(
    async (id: string) => {
      try {
        const res = await assetGateway.deleteAsset(id);
        if (res.status === "ok") {
          await fetchAssets();
          showSnackbar(t("asset.success_deleted"), "info");
          return { error: null };
        }
        return { error: res.error };
      } catch (e) {
        logger.error("Failed to delete asset", { error: e });
        return { error: String(e) };
      }
    },
    [fetchAssets, showSnackbar, t],
  );

  const activeCount = useMemo(() => assets.filter((a) => !a.is_archived).length, [assets]);

  return {
    assets,
    activeCount,
    loading,
    fetchError,
    addAsset,
    updateAsset,
    archiveAsset,
    unarchiveAsset,
    deleteAsset,
    fetchAssets,
  };
}
