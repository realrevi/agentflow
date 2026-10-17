import { describe, expect, it } from 'vitest';
import type { AgentTemplate, InstallPlan, PlannedFile } from '../../../shared/types';
import { groups, missing, newer, openConflicts, prefill } from './answers';

const template: AgentTemplate = {
  schema: 1,
  id: 'ui-visual-tester',
  name: 'UI/Visual Tester',
  description: 'Checks UI.',
  version: '1.0.0',
  instructions: '{{brandColors}} {{fonts}} {{framework}} {{a11y}} {{level}} {{focus}}',
  questions: [
    { id: 'brandColors', label: 'Brand colors', type: 'color', detect: 'colors', group: 'Design' },
    {
      id: 'fonts',
      label: 'Fonts',
      type: 'font',
      detect: 'fonts',
      default: 'Arial',
      group: 'Design',
    },
    { id: 'framework', label: 'Framework', type: 'text', detect: 'uiFramework', required: true },
    { id: 'a11y', label: 'Accessibility', type: 'boolean', default: true, group: 'Design' },
    { id: 'level', label: 'Level', type: 'choice', options: ['low', 'high'], detect: 'spacing' },
    { id: 'focus', label: 'Focus', type: 'multiChoice', options: ['a', 'b'] },
  ],
};

describe('installer answers', () => {
  it('prefills from detection, marks what was detected, then defaults', () => {
    const detection = {
      values: { colors: 'primary #1e40af', uiFramework: 'React', spacing: 'not an option' },
      existing: [],
    };
    const { answers, detected } = prefill(template, detection);
    expect(answers).toEqual({
      brandColors: 'primary #1e40af',
      fonts: 'Arial',
      framework: 'React',
      a11y: true,
      level: '',
      focus: [],
    });
    expect([...detected].sort()).toEqual(['brandColors', 'framework']);
  });

  it('keeps the answers of an earlier install over detection', () => {
    const { answers, detected } = prefill(
      template,
      { values: { colors: '#000' }, existing: [] },
      { brandColors: 'mine #fff' },
    );
    expect(answers['brandColors']).toBe('mine #fff');
    expect(detected.has('brandColors')).toBe(false);
  });

  it('groups questions in order and lists missing required answers', () => {
    expect(groups(template.questions ?? []).map((g) => [g.name, g.questions.length])).toEqual([
      ['Design', 3],
      ['', 3],
    ]);
    expect(missing(template, { framework: '  ' })).toEqual(['Framework']);
    expect(missing(template, { framework: 'Vue' })).toEqual([]);
  });

  it('blocks install until every conflict is confirmed', () => {
    const file = (display: string, conflict: PlannedFile['conflict']): PlannedFile => ({
      target: 'cursor',
      scope: 'project',
      path: display,
      display,
      kind: 'file',
      status: conflict ? 'conflict' : 'create',
      conflict,
      before: null,
      after: '',
      remove: false,
      created: false,
    });
    const plan: InstallPlan = {
      files: [file('a.mdc', 'foreign'), file('b.md', null), file('c.md', 'edited')],
      userScope: false,
    };
    expect(openConflicts(plan, new Set())).toEqual(['a.mdc', 'c.md']);
    expect(openConflicts(plan, new Set(['a.mdc', 'c.md']))).toEqual([]);
  });

  it('compares template versions', () => {
    expect(newer('1.1.0', '1.0.9')).toBe(true);
    expect(newer('1.0.0', '1.0.0')).toBe(false);
    expect(newer('2.0.0', '10.0.0')).toBe(false);
  });
});
