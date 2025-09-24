#!/usr/bin/env node

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const binaryName = process.platform === "win32" ? "iconmate.exe" : "iconmate";
const binaryPath = path.join(__dirname, "bin", binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error("❌ iconmate binary not found. Please reinstall:");
  console.error("npm uninstall -g iconmate && npm install -g iconmate");
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), { stdio: "inherit" });

child.on("error", (err) => {
  console.error("❌ Failed to start iconmate:", err.message);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  process.exit(signal ? 1 : code || 0);
});
