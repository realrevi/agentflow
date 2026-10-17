// Pixel characters (core feature): sprite data, renderer, shared animation loop, avatars
// and the agent state heuristic. Imports only shared/ and other core features.
export { animatorStats, reducedMotion, startAnimation, type Animation } from './animator';
import './characters.css';
export { Avatar, type AvatarProps } from './Avatar';
export { CharacterPicker } from './CharacterPicker';
export {
  characterById,
  characters,
  isCharacterRef,
  loadCharacters,
  pickCharacter,
} from './catalog';
export { AgentStateTracker, spriteFor, type AgentState } from './stateMachine';
export type { CharacterRef, SpriteSheet, SpriteState } from './types';
