// Decides whether Pebble is ready to chat, and what to tell the user if not.
// Shared by the chat banner (ChatView) and the send guard (Composer) so they
// never disagree.

import type { Config, ModelStatus, OllamaStatus } from "./types";

export type ReadinessKind = "ollama" | "no_model" | "wrong_model";

export interface Readiness {
  canSend: boolean;
  banner: null | {
    kind: ReadinessKind;
    tone: "error" | "warn";
    title: string;
    body: string;
  };
}

function baseName(m: string): string {
  return m.split(":")[0];
}

export function chatReadiness(
  ollama: OllamaStatus | null,
  models: ModelStatus[],
  modelsLoaded: boolean,
  config: Config | null,
): Readiness {
  // Status not checked yet — don't block; the backend will give a clear error.
  if (!ollama) return { canSend: true, banner: null };

  if (!ollama.running) {
    return {
      canSend: false,
      banner: {
        kind: "ollama",
        tone: "error",
        title: "Ollama isn't running",
        body: "Pebble thinks with a local Ollama model. Start Ollama (launch the app, or run `ollama serve` in a terminal) and I'll wake right up. 🪨",
      },
    };
  }

  // Running, but the model list isn't back yet — wait quietly instead of flashing.
  if (!modelsLoaded) return { canSend: true, banner: null };

  if (models.length === 0) {
    return {
      canSend: false,
      banner: {
        kind: "no_model",
        tone: "error",
        title: "No model installed yet",
        body: "Pebble needs a model to think with. Pull one in Settings → AI Model — llama3.2 is a friendly first pick — then we're good to go. 🪨",
      },
    };
  }

  const model = (config?.model ?? "").trim();
  const installed = models.map((m) => m.name);
  const have =
    !!model &&
    (installed.includes(model) || installed.some((n) => baseName(n) === baseName(model)));

  if (model && !have) {
    return {
      // Let the backend try and return a precise message; just warn here.
      canSend: true,
      banner: {
        kind: "wrong_model",
        tone: "warn",
        title: `"${model}" isn't installed`,
        body: "Your chosen model isn't here yet. Pull it in Settings → AI Model, or switch to one you already have.",
      },
    };
  }

  return { canSend: true, banner: null };
}
