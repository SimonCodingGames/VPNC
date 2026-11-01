/** Global UI state: confirm dialog + toasts (no secrets stored here). */

export type ConfirmOptions = {
  title: string;
  message?: string;
  confirmLabel?: string;
  danger?: boolean;
};

type ConfirmRequest = ConfirmOptions & { resolve: (ok: boolean) => void };

export type Toast = { id: number; kind: 'success' | 'error' | 'info'; text: string };

export const ui = $state<{ confirm: ConfirmRequest | null; toasts: Toast[] }>({
  confirm: null,
  toasts: []
});

/** Promise-based replacement for window.confirm(). */
export function confirmAction(opts: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    ui.confirm?.resolve(false);
    ui.confirm = { ...opts, resolve };
  });
}

export function settleConfirm(ok: boolean) {
  const req = ui.confirm;
  ui.confirm = null;
  req?.resolve(ok);
}

let nextId = 1;
export function toast(text: string, kind: Toast['kind'] = 'success', ms = 3200) {
  const id = nextId++;
  ui.toasts = [...ui.toasts, { id, kind, text }];
  setTimeout(() => dismissToast(id), ms);
}

export function dismissToast(id: number) {
  ui.toasts = ui.toasts.filter((t) => t.id !== id);
}
