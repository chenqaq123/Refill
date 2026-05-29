import type { ButtonHTMLAttributes, ReactNode } from "react";
import { cn } from "../../lib/cn";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  icon?: ReactNode;
  variant?: "primary" | "soft" | "ghost" | "danger";
};

export function Button({ className, icon, variant = "soft", children, ...props }: ButtonProps) {
  return (
    <button
      {...props}
      className={cn(
        "pressable inline-flex h-9 items-center justify-center gap-2 rounded-full px-3.5 text-sm font-bold outline-none",
        "focus-visible:ring-2 focus-visible:ring-blue/35 disabled:pointer-events-none disabled:opacity-45",
        variant === "primary" && "bg-blue text-white shadow-[0_8px_18px_rgba(35,120,238,0.18)] hover:bg-[#1f6fd7]",
        variant === "soft" && "bg-blue/10 text-blue hover:bg-blue/15",
        variant === "ghost" && "bg-transparent text-sub hover:bg-black/5",
        variant === "danger" && "bg-red/10 text-red hover:bg-red/15",
        className,
      )}
    >
      {icon}
      {children}
    </button>
  );
}
