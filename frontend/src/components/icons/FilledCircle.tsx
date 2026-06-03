import type { SVGProps } from "react";

interface FilledCircleProps extends Omit<SVGProps<SVGSVGElement>, "width" | "height"> {
  size?: number | string;
}

// A solid disc. Lucide ships only outline icons, so this fills the gap for a
// filled circle. Uses fill="currentColor" so the surrounding text-color class
// drives its color, matching how Lucide icons are tinted.
export function FilledCircle({ size = 24, ...props }: FilledCircleProps) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
      {...props}
    >
      <circle cx="12" cy="12" r="6" />
    </svg>
  );
}
