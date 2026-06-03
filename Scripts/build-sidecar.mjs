// Build the `taskpond` CLI and stage it as a Tauri sidecar:
//   target/<profile>/taskpond  ->  src-tauri/binaries/taskpond-<host-triple>
// Usage: node Scripts/build-sidecar.mjs [debug|release]   (default: debug)
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, chmodSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const profile = process.argv[2] === "release" ? "release" : "debug";

// 1. Build the CLI crate.
const buildArgs = ["build", "-p", "taskpond-cli"];
if (profile === "release") buildArgs.push("--release");
execFileSync("cargo", buildArgs, { cwd: repoRoot, stdio: "inherit" });

// 2. Read the host target triple from `rustc -vV` (the "host: <triple>" line).
const rustcOut = execFileSync("rustc", ["-vV"], { cwd: repoRoot, encoding: "utf8" });
const hostLine = rustcOut.split("\n").find((l) => l.startsWith("host:"));
if (!hostLine) {
  throw new Error("could not determine host target triple from `rustc -vV`");
}
const triple = hostLine.slice("host:".length).trim();

// 3. Copy target/<profile>/taskpond -> src-tauri/binaries/taskpond-<triple> (executable).
const src = join(repoRoot, "target", profile, "taskpond");
const destDir = join(repoRoot, "src-tauri", "binaries");
const dest = join(destDir, `taskpond-${triple}`);
mkdirSync(destDir, { recursive: true });
copyFileSync(src, dest);
chmodSync(dest, 0o755);

console.log(`sidecar staged: ${dest}`);
