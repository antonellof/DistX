import { createOpenAI } from "@ai-sdk/openai";
import {
  customProvider,
  extractReasoningMiddleware,
  wrapLanguageModel,
} from "ai";
import { isTestEnvironment } from "../constants";

const THINKING_SUFFIX_REGEX = /-thinking$/;

// Create OpenAI provider instance
const openai = createOpenAI({
  apiKey: process.env.OPENAI_API_KEY,
});

export const myProvider = isTestEnvironment
  ? (() => {
      const {
        artifactModel,
        chatModel,
        reasoningModel,
        titleModel,
      } = require("./models.mock");
      return customProvider({
        languageModels: {
          "chat-model": chatModel,
          "chat-model-reasoning": reasoningModel,
          "title-model": titleModel,
          "artifact-model": artifactModel,
        },
      });
    })()
  : null;

export function getLanguageModel(modelId: string) {
  if (isTestEnvironment && myProvider) {
    return myProvider.languageModel(modelId);
  }

  const isReasoningModel =
    modelId.includes("reasoning") || modelId.endsWith("-thinking");

  // Map model IDs to OpenAI models
  const openaiModelId = mapToOpenAIModel(modelId);

  if (isReasoningModel) {
    return wrapLanguageModel({
      model: openai(openaiModelId),
      middleware: extractReasoningMiddleware({ tagName: "thinking" }),
    });
  }

  return openai(openaiModelId);
}

export function getTitleModel() {
  if (isTestEnvironment && myProvider) {
    return myProvider.languageModel("title-model");
  }
  // Use GPT-4o-mini for title generation (fast and cheap)
  return openai("gpt-4o-mini");
}

export function getArtifactModel() {
  if (isTestEnvironment && myProvider) {
    return myProvider.languageModel("artifact-model");
  }
  // Use GPT-4o-mini for artifacts
  return openai("gpt-4o-mini");
}

/**
 * Map various model IDs to OpenAI model names
 */
function mapToOpenAIModel(modelId: string): string {
  // Remove thinking suffix if present
  const baseId = modelId.replace(THINKING_SUFFIX_REGEX, "");
  
  // Common mappings
  const modelMap: Record<string, string> = {
    // OpenAI models
    "openai/gpt-4o": "gpt-4o",
    "openai/gpt-4o-mini": "gpt-4o-mini",
    "openai/gpt-4-turbo": "gpt-4-turbo",
    "openai/gpt-3.5-turbo": "gpt-3.5-turbo",
    "gpt-4o": "gpt-4o",
    "gpt-4o-mini": "gpt-4o-mini",
    "gpt-4-turbo": "gpt-4-turbo",
    "gpt-3.5-turbo": "gpt-3.5-turbo",
    // Fallback for Anthropic models (use GPT-4o as equivalent)
    "anthropic/claude-haiku-4.5": "gpt-4o-mini",
    "anthropic/claude-sonnet-4": "gpt-4o",
    "anthropic/claude-opus-4": "gpt-4o",
  };

  return modelMap[baseId] || "gpt-4o-mini";
}
