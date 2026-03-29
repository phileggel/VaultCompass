import { useCallback } from "react";
import type { CreateAssetDTO, UpdateAssetDTO } from "@/bindings";
import { useAppStore } from "@/lib/store";
import { assetGateway } from "./gateway";

export function useAssets() {
  const assets = useAppStore((state) => state.assets);
  const loading = useAppStore((state) => state.isLoadingAssets);
  const fetchAssets = useAppStore((state) => state.fetchAssets);

  const addAsset = useCallback(async (dto: CreateAssetDTO) => {
    try {
      const res = await assetGateway.createAsset(dto);
      if (res.status === "ok") {
        return res.data;
      }
    } catch (e) {
      console.error("Failed to add asset", e);
    }
    return null;
  }, []);

  const updateAsset = useCallback(async (dto: UpdateAssetDTO) => {
    try {
      const res = await assetGateway.updateAsset(dto);
      if (res.status === "ok") {
        return true;
      }
    } catch (e) {
      console.error("Failed to update asset", e);
    }
    return false;
  }, []);

  const deleteAsset = useCallback(async (id: string) => {
    try {
      const res = await assetGateway.deleteAsset(id);
      if (res.status === "ok") {
        return true;
      }
    } catch (e) {
      console.error("Failed to delete asset", e);
    }
    return false;
  }, []);

  return { assets, loading, addAsset, updateAsset, deleteAsset, fetchAssets };
}
