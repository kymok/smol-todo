import type { CollectionColor } from "../api/types";

// Tailwind text-color class for each collection color. Full class strings (not
// interpolated) so Tailwind's scanner picks them up. gray maps to neutral.
export const COLLECTION_COLOR_CLASS: Record<CollectionColor, string> = {
  gray: "text-neutral-500",
  red: "text-red-500",
  orange: "text-orange-500",
  yellow: "text-yellow-500",
  green: "text-emerald-500",
  blue: "text-sky-500",
  purple: "text-violet-500",
};
