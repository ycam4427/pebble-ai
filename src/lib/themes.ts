// Theme + persona registries used by Settings and the animated background.

export interface ThemeDef {
  key: string;
  label: string;
  desc: string;
  dark: boolean;
}

export const THEMES: ThemeDef[] = [
  { key: "pebble", label: "Pebble", desc: "Soft pink, grey & white — the cozy default", dark: false },
  { key: "matcha", label: "Matcha", desc: "Gentle sage & warm cream", dark: false },
  { key: "cloud", label: "Cloud", desc: "Clean, airy soft greys", dark: false },
  { key: "stars", label: "Stars", desc: "A calm night sky that gently twinkles", dark: true },
  {
    key: "liquidglass",
    label: "Liquid Glass",
    desc: "Dark blue frosted glass with floating orbs",
    dark: true,
  },
];

export interface PersonaDef {
  key: string;
  label: string;
  emoji: string;
  desc: string;
}

export const PERSONAS: PersonaDef[] = [
  { key: "cozy", label: "Cozy & warm", emoji: "🤍", desc: "Soft, comforting, friend-next-door" },
  { key: "cheerful", label: "Cheerful & bubbly", emoji: "✨", desc: "Upbeat and encouraging" },
  { key: "calm", label: "Calm & gentle", emoji: "🌿", desc: "Serene and reassuring" },
  { key: "playful", label: "Playful & witty", emoji: "🎈", desc: "Light, fun, a little cheeky" },
];

export const DEFAULT_THEME = "pebble";
