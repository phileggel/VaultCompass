import { useEffect, useState } from "react";
// import { commands } from "../../bindings";

export interface AssetSnapshot {
  id: string;
  asset_name: string;
  ticker: string | null;
  quantity: number;
  purchase_price: number;
  price: number;
  currency: string;
  date: string;
  value_eur: number;
  performance_eur: number;
}

export interface AccountSnapshot {
  date: string;
  snapshots: AssetSnapshot[];
}

export function useAccountAssetDetails(accountId: string | null, _dates: string[]) {
  const [data, setData] = useState<AccountSnapshot[]>([]);
  const [loading, _setLoading] = useState(false);
  const [error, _setError] = useState<string | null>(null);

  useEffect(() => {
    if (!accountId) {
      setData([]);
      return;
    }

    // getAccountDetails was removed during simplification
    /*
    const fetchData = async () => {
      setLoading(true);
      setError(null);
      try {
        const result = await commands.getAccountDetails(accountId, dates);
        if (result.status === "ok") {
          setData(result.data);
        } else {
          setError(result.error);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to fetch data");
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    */
  }, [accountId]);

  return { data, loading, error };
}
