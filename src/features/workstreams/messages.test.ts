import { describe, expect, it } from 'vitest';
import { managerInstructions } from './messages';

describe('manager instructions', () => {
  it('lists workers with their kind and the AgentFlow agent they run', () => {
    const text = managerInstructions('Shop', [
      { name: 'Pip', kind: 'API agent (glm-5.3)', role: 'Code Reviewer' },
      { name: 'Rusty', kind: 'Claude Code' },
    ]);
    expect(text).toContain('- Pip (API agent (glm-5.3)): runs the "Code Reviewer" agent\n');
    expect(text).toContain('- Rusty (Claude Code)\n');
  });
});
