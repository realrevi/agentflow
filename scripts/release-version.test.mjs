import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { bumpAll, FILES, isVersion } from './release-version.mjs';

const root = join(import.meta.dirname, '..');
const texts = Object.fromEntries(
  Object.entries(FILES).map(([key, file]) => [key, readFileSync(join(root, file), 'utf8')]),
);

function versions(t) {
  return {
    packageJson: JSON.parse(t.packageJson).version,
    tauriConf: JSON.parse(t.tauriConf).version,
    cargoToml: /^\[package\][^[]*?^version = "([^"]+)"/m.exec(t.cargoToml)?.[1],
    cargoLock: /^\[\[package\]\]\nname = "agentflow"\nversion = "([^"]+)"/m.exec(t.cargoLock)?.[1],
  };
}

describe('release:version', () => {
  it('starts from one version in all four files', () => {
    const now = versions(texts);
    expect(new Set(Object.values(now)).size).toBe(1);
    expect(isVersion(now.packageJson)).toBe(true);
  });

  it('sets the same version everywhere and touches nothing else', () => {
    const next = bumpAll(texts, '1.2.3');
    expect(versions(next)).toEqual({
      packageJson: '1.2.3',
      tauriConf: '1.2.3',
      cargoToml: '1.2.3',
      cargoLock: '1.2.3',
    });
    for (const key of Object.keys(FILES)) {
      const changed = next[key].split('\n').filter((line, i) => line !== texts[key].split('\n')[i]);
      expect(changed).toHaveLength(1);
    }
    // Dependencies keep their own versions.
    expect(next.cargoToml).toContain('serde = { version = "1"');
    expect(JSON.parse(next.packageJson).dependencies).toEqual(
      JSON.parse(texts.packageJson).dependencies,
    );
  });

  it('rejects anything but a plain x.y.z', () => {
    for (const bad of ['', '1.2', 'v1.2.3', '1.2.3-rc.1', '01.2.3', '1.2.3.4']) {
      expect(() => bumpAll(texts, bad)).toThrow();
    }
  });

  it('fails before writing when a file has no version field', () => {
    expect(() => bumpAll({ ...texts, cargoLock: '# empty\n' }, '1.2.3')).toThrow(/Cargo.lock/);
  });
});
