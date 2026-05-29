import type { InputHTMLAttributes } from "react";
import { cn } from "../../lib/cn";

type TextFieldProps = InputHTMLAttributes<HTMLInputElement> & {
  label: string;
  hint?: string;
};

export function TextField({ label, hint, className, ...props }: TextFieldProps) {
  return (
    <label className="block space-y-2">
      <span className="text-sm font-bold text-ink">{label}</span>
      <input
        {...props}
        className={cn(
          "h-11 w-full rounded-2xl border border-line bg-white px-4 text-sm font-semibold text-ink outline-none",
          "placeholder:text-sub/55 focus:border-blue/55 focus:ring-4 focus:ring-blue/10",
          className,
        )}
      />
      {hint ? <span className="block text-xs font-semibold text-sub">{hint}</span> : null}
    </label>
  );
}
