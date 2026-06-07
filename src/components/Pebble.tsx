import pebble from "../assets/pebble.png";

// The mascot. Source is 128×128 — never request a size above that so he stays crisp.
export default function Pebble({
  size = 28,
  float = false,
  className = "",
}: {
  size?: number;
  float?: boolean;
  className?: string;
}) {
  const px = Math.min(size, 128);
  return (
    <img
      src={pebble}
      alt="Pebble"
      width={px}
      height={px}
      draggable={false}
      className={`${float ? "pebble-float " : ""}${className}`}
      style={{ width: px, height: px }}
    />
  );
}
