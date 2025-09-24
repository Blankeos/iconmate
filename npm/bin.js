#!/usr/bin/env node

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

function getBinaryPath() {
  const platform = process.platform;
  const binaryName = platform === "win32" ? "iconmate.exe" : "iconmate";
  const binaryPath = path.join(__dirname, "bin", binaryName);

  if (!fs.existsSync(binaryPath)) {
    console.error(
      "❌ iconmate binary not found. Please reinstall the package:",
    );
    console.error("npm uninstall iconmate && npm install iconmate");
    process.exit(1);
  }

  return binaryPath;
}

function runBinary() {
  const binaryPath = getBinaryPath();
  const args = process.argv.slice(2);

  const child = spawn(binaryPath, args, {
    stdio: "inherit",
    windowsHide: false,
  });

  child.on("error", (error) => {
    console.error("❌ Failed to start iconmate:", error.message);
    process.exit(1);
  });

  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code || 0);
    }
  });
}

runBinary();
