import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

type ChipProps = {
  children: ReactNode;
  tone?: "blue" | "green" | "teal" | "amber" | "red" | "gray";
  solid?: boolean;
  icon?: ReactNode;
};

const tones = {
  blue: "bg-blue/10 text-blue",
  green: "bg-green/12 text-green",
  teal: "bg-teal/10 text-teal",
  amber: "bg-amber/12 text-amber",
  red: "bg-red/10 text-red",
  gray: "bg-black/5 text-sub",
};

const solidTones = {
  blue: "bg-blue text-white",
  green: "bg-green text-white",
  teal: "bg-teal text-white",
  amber: "bg-amber text-white",
  red: "bg-red text-white",
  gray: "bg-ink/100 text-white",
};

export function Chip({ children, tone = "gray", solid, icon }: ChipProps) {
  return (
    <span
      className={cn(
        "inline-flex h-6 items-center gap-1.5 rounded-full px-2.5 text-[11px] font-black leading-none",
        solid ? solidTones[tone] : tones[tone],
      )}
    >
      {icon}
      {children}
    </span>
  );
}
