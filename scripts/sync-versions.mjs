#!/usr/bin/env node
/**
 * Sync / verify versions across Cargo workspace, Python, and npm packages.
 * Usage: node scripts/sync-versions.mjs [--check]
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const checkOnly = process.argv.includes("--check");

function readFile(p) {
  return fs.readFileSync(p, "utf8");
}

function cargoWorkspaceVersion() {
  const t = readFile(path.join(root, "Cargo.toml"));
  const m = t.match(/\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("Could not parse [workspace.package] version in Cargo.toml");
  return m[1];
}

function pyprojectVersion() {
  const t = readFile(path.join(root, "python", "pyproject.toml"));
  const m = t.match(/\[project\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("Could not parse [project] version in python/pyproject.toml");
  return m[1];
}

function jsonVersion(pkgRel) {
  const j = JSON.parse(readFile(path.join(root, pkgRel)));
  return j.version;
}

function setJsonVersion(pkgRel, ver) {
  const p = path.join(root, pkgRel);
  const j = JSON.parse(readFile(p));
  j.version = ver;
  fs.writeFileSync(p, `${JSON.stringify(j, null, 2)}\n`);
}

function main() {
  const cargoVer = cargoWorkspaceVersion();
  const pyVer = pyprojectVersion();
  const coreVer = jsonVersion("packages/core/package.json");
  const jupyterVer = jsonVersion("packages/jupyter/package.json");
  const e2eVer = jsonVersion("packages/e2e/package.json");
  const examplesVer = jsonVersion("examples/web/package.json");

  const rows = [
    ["Cargo.toml [workspace.package]", cargoVer],
    ["python/pyproject.toml", pyVer],
    ["packages/core/package.json", coreVer],
    ["packages/jupyter/package.json", jupyterVer],
    ["packages/e2e/package.json", e2eVer],
    ["examples/web/package.json", examplesVer],
  ];

  const uniq = [...new Set(rows.map((r) => r[1]))];
  if (uniq.length !== 1) {
    console.error("Version mismatch:\n");
    for (const [name, v] of rows) console.error(`  ${name}: ${v}`);
    process.exit(1);
  }

  const version = uniq[0];
  console.log(`All versions agree: ${version}`);

  if (!checkOnly) {
    console.log("(Use --check for verification only; writing is not implemented — bump versions with release tooling.)");
  }
}

main();
