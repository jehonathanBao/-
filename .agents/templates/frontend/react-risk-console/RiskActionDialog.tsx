import * as AlertDialog from "@radix-ui/react-alert-dialog";
import { useState } from "react";

type RiskActionDialogProps = {
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => Promise<void>;
  children: React.ReactNode;
};

export function RiskActionDialog({
  title,
  description,
  confirmLabel,
  onConfirm,
  children,
}: RiskActionDialogProps) {
  const [pending, setPending] = useState(false);

  async function handleConfirm() {
    if (pending) return;
    setPending(true);
    try {
      await onConfirm();
    } finally {
      setPending(false);
    }
  }

  return (
    <AlertDialog.Root>
      <AlertDialog.Trigger asChild>{children}</AlertDialog.Trigger>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="fixed inset-0 bg-black/40" />
        <AlertDialog.Content className="fixed left-1/2 top-1/2 w-[min(92vw,480px)] -translate-x-1/2 -translate-y-1/2 rounded-xl bg-white p-6 shadow-xl">
          <AlertDialog.Title className="text-lg font-semibold">{title}</AlertDialog.Title>
          <AlertDialog.Description className="mt-2 text-sm text-slate-600">
            {description}
          </AlertDialog.Description>
          <div className="mt-6 flex justify-end gap-3">
            <AlertDialog.Cancel className="rounded-md border px-3 py-2">Cancel</AlertDialog.Cancel>
            <AlertDialog.Action
              className="rounded-md bg-red-600 px-3 py-2 text-white disabled:opacity-60"
              disabled={pending}
              onClick={(event) => {
                event.preventDefault();
                void handleConfirm();
              }}
            >
              {pending ? "Working..." : confirmLabel}
            </AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}
