export const DISCORD_URL = "https://discord.gg/ATXTFSmX6N";

// What each model is good at / struggles with — shown next to the picker so you
// know why you might upgrade (the tiny default is a chatter, not an organizer).
export interface ModelBlurb {
  name: string;
  tier: string;
  good: string;
  limit: string;
}

export const MODEL_BLURBS: ModelBlurb[] = [
  {
    name: "llama3.2:3b",
    tier: "Featherweight",
    good: "Quick chats, simple finds & answers. Runs on almost anything (~2 GB).",
    limit: "Weak at multi-step organizing and following exact instructions.",
  },
  {
    name: "llama3.1:8b",
    tier: "Balanced",
    good: "Good at organizing, following steps, and finding things. ~6 GB.",
    limit: "Wants a midrange GPU or 16 GB+ RAM.",
  },
  {
    name: "qwen2.5:7b",
    tier: "Balanced",
    good: "Strong all-rounder — instructions & tidy-ups. ~5–6 GB.",
    limit: "A little heavier than the default.",
  },
  {
    name: "qwen2.5:14b",
    tier: "Powerful",
    good: "Great at multi-step planning & precise organizing. ~10–12 GB VRAM.",
    limit: "Needs a strong GPU.",
  },
  {
    name: "deepseek-r1:8b",
    tier: "Reasoning",
    good: "Careful step-by-step thinking. ~6 GB.",
    limit: "A bit slower to reply.",
  },
];

export function blurbFor(name: string): ModelBlurb | undefined {
  return MODEL_BLURBS.find((b) => b.name === name);
}

/// A rough "this is a small/default model" check, to decide whether to nudge an upgrade.
export function isSmallModel(name: string): boolean {
  return name.trim() === "" || /(:0\.5b|:1\.5b|:1b|:2b|:3b|^llama3\.2$|phi|gemma:2b)/i.test(name);
}
