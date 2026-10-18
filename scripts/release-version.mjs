#!/usr/bin/env node
// `pnpm release:version <x.y.z>`: sets the app version in package.json, src-tauri/Cargo.toml,
// src-tauri/Cargo.lock and src-tauri/tauri.conf.json together. Edits the text in place (no
// reformatting) and refuses to write anything unless every file had its version field.
import { readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

/** Plain x.y.z: Windows MSI installers reject pre-release suffixes like -rc.1. */
const VERSION = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

export const FILES = {
  packageJson: 'package.json',
  cargoToml: 'src-tauri/Cargo.toml',
  cargoLock: 'src-tauri/Cargo.lock',
  tauriConf: 'src-tauri/tauri.conf.json',
};

export function isVersion(value) {
  return VERSION.test(value);
}

/** Replaces the first match of `pattern` (groups: before, after) or throws. */
function replaceOnce(text, pattern, version, what) {
  if (!pattern.test(text)) throw new Error(`no version field found in ${what}`);
  return text.replace(pattern, (_match, before, after) => `${before}${version}${after}`);
}

/** The top-level "version" of a JSON file (the first one, as both files keep it on top). */
export function setJsonVersion(text, version, what) {
  return replaceOnce(text, /^(\s*"version":\s*")[^"]*(")/m, version, what);
}

export function setCargoTomlVersion(text, version) {
  return replaceOnce(text, /^(\[package\][^[]*?^version\s*=\s*")[^"]*(")/m, version, 'Cargo.toml');
}

export function setCargoLockVersion(text, version) {
  return replaceOnce(
    text,
    /^(\[\[package\]\]\r?\nname = "agentflow"\r?\nversion = ")[^"]*(")/m,
    version,
    'Cargo.lock',
  );
}

/** Every file's new text, or an error before anything is written. */
export function bumpAll(texts, version) {
  if (!isVersion(version)) throw new Error(`"${version}" is not a plain x.y.z version`);
  return {
    packageJson: setJsonVersion(texts.packageJson, version, 'package.json'),
    cargoToml: setCargoTomlVersion(texts.cargoToml, version),
    cargoLock: setCargoLockVersion(texts.cargoLock, version),
    tauriConf: setJsonVersion(texts.tauriConf, version, 'tauri.conf.json'),
  };
}

function main() {
  const version = process.argv[2] ?? '';
  const root = process.cwd();
  const texts = {};
  for (const [key, file] of Object.entries(FILES)) {
    texts[key] = readFileSync(join(root, file), 'utf8');
  }
  const next = bumpAll(texts, version);
  for (const [key, file] of Object.entries(FILES)) {
    writeFileSync(join(root, file), next[key]);
  }
  console.log(`Version ${version}: ${Object.values(FILES).join(', ')}`);
  console.log('Next: add a CHANGELOG.md section, commit, then tag v' + version + '.');
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    main();
  } catch (err) {
    console.error(`release:version: ${err instanceof Error ? err.message : String(err)}`);
    process.exit(1);
  }
}
