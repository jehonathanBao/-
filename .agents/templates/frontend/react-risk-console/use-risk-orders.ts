import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { riskApi } from "./api";

export type RiskLevel = "low" | "medium" | "high" | "critical" | "data_insufficient";

export type RiskOrder = {
  orderId: string;
  buyerId?: string;
  shopId: string;
  riskLevel: RiskLevel;
  riskScore: number;
  createdAt: string;
};

export type RiskOrderFilters = {
  page: number;
  pageSize: number;
  riskLevel?: RiskLevel;
  shopId?: string;
};

export function useRiskOrders(filters: RiskOrderFilters) {
  return useQuery({
    queryKey: ["risk-orders", filters],
    queryFn: async () => {
      const response = await riskApi.get<{ items: RiskOrder[]; total: number }>("/orders/risk", {
        params: filters,
      });
      return response.data;
    },
  });
}

export function useReleaseOrder() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (orderId: string) => {
      await riskApi.post(`/orders/${encodeURIComponent(orderId)}/release`);
    },
    onSettled: () => queryClient.invalidateQueries({ queryKey: ["risk-orders"] }),
  });
}
