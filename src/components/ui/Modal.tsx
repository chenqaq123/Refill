import type { ReactNode } from "react";
import { X } from "lucide-react";
import { Button } from "./Button";

type ModalProps = {
  title: string;
  description?: string;
  open: boolean;
  onClose: () => void;
  children: ReactNode;
};

export function Modal({ title, description, open, onClose, children }: ModalProps) {
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/16 p-6 backdrop-blur-sm">
      <div className="w-full max-w-xl rounded-3xl border border-line bg-panel p-5 shadow-soft">
        <div className="mb-5 flex items-start justify-between gap-4">
          <div>
            <h2 className="text-xl font-bold text-ink">{title}</h2>
            {description ? <p className="mt-1 text-sm font-medium text-sub">{description}</p> : null}
          </div>
          <Button variant="ghost" className="h-9 w-9 px-0" onClick={onClose} aria-label="关闭">
            <X size={17} />
          </Button>
        </div>
        {children}
      </div>
    </div>
  );
}
