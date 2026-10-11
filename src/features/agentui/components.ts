// Heavy part of the agent UI, imported by lazy panels only (see index.ts).
export { AgentOverlay, type AgentOverlayHandle } from './overlay/AgentOverlay';
export { createPromptHistory } from './prompt/history';
export { playPacket } from './prompt/packet';
export { PromptBar, type PromptBarHandle, type SentPrompt } from './prompt/PromptBar';
