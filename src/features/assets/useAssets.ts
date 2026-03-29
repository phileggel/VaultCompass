import { useCallback, useMemo } from "react";
import type { CreateAssetDTO, UpdateAssetDTO } from "@/bindings";
import { logger } from "@/lib/logger";
import { useAppStore } from "@/lib/store";
import { assetGateway } from "./gateway";

export function useAssets() {
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
          return { data: res.data, error: null };
        }
        return { data: null, error: res.error };
      } catch (e) {
        logger.error("Failed to add asset", { error: e });
        return { data: null, error: String(e) };
      }
    },
    [fetchAssets],
  );

  const updateAsset = useCallback(
    async (dto: UpdateAssetDTO) => {
      try {
        const res = await assetGateway.updateAsset(dto);
        if (res.status === "ok") {
          await fetchAssets();
          return { data: res.data, error: null };
        }
        return { data: null, error: res.error };
      } catch (e) {
        logger.error("Failed to update asset", { error: e });
        return { data: null, error: String(e) };
      }
    },
    [fetchAssets],
  );

  const archiveAsset = useCallback(
    async (id: string) => {
      try {
        const res = await assetGateway.archiveAsset(id);
        if (res.status === "ok") {
          await fetchAssets();
          return { error: null };
        }
        return { error: res.error };
      } catch (e) {
        logger.error("Failed to archive asset", { error: e });
        return { error: String(e) };
      }
    },
    [fetchAssets],
  );

  const unarchiveAsset = useCallback(
    async (id: string) => {
      try {
        const res = await assetGateway.unarchiveAsset(id);
        if (res.status === "ok") {
          await fetchAssets();
          return { error: null };
        }
        return { error: res.error };
      } catch (e) {
        logger.error("Failed to unarchive asset", { error: e });
        return { error: String(e) };
      }
    },
    [fetchAssets],
  );

  const deleteAsset = useCallback(
    async (id: string) => {
      try {
        const res = await assetGateway.deleteAsset(id);
        if (res.status === "ok") {
          await fetchAssets();
          return { error: null };
        }
        return { error: res.error };
      } catch (e) {
        logger.error("Failed to delete asset", { error: e });
        return { error: String(e) };
      }
    },
    [fetchAssets],
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
