import { RiskActionDialog } from "./RiskActionDialog";
import { RiskBadge } from "./RiskBadge";
import { useReleaseOrder, useRiskOrders, type RiskOrderFilters } from "./use-risk-orders";

export function RiskOrdersTable({ filters }: { filters: RiskOrderFilters }) {
  const orders = useRiskOrders(filters);
  const releaseOrder = useReleaseOrder();

  if (orders.isLoading) return <div>Loading risk orders...</div>;
  if (orders.isError) return <div role="alert">Unable to load risk orders.</div>;

  return (
    <table className="w-full table-fixed border-collapse text-sm">
      <thead>
        <tr className="border-b text-left">
          <th className="p-2">Order</th>
          <th className="p-2">Shop</th>
          <th className="p-2">Risk</th>
          <th className="p-2">Score</th>
          <th className="p-2">Action</th>
        </tr>
      </thead>
      <tbody>
        {orders.data?.items.map((order) => (
          <tr className="border-b" key={order.orderId}>
            <td className="truncate p-2">{order.orderId}</td>
            <td className="truncate p-2">{order.shopId}</td>
            <td className="p-2">
              <RiskBadge level={order.riskLevel} />
            </td>
            <td className="p-2">{order.riskScore.toFixed(2)}</td>
            <td className="p-2">
              <RiskActionDialog
                title="Release order?"
                description="Manual release is audited and prevents this order from being auto-blocked by rules."
                confirmLabel="Release"
                onConfirm={() => releaseOrder.mutateAsync(order.orderId)}
              >
                <button className="rounded-md border px-2 py-1" type="button">
                  Release
                </button>
              </RiskActionDialog>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
