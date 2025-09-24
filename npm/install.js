#!/usr/bin/env node

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const { pipeline } = require("stream");
const { promisify } = require("util");

const streamPipeline = promisify(pipeline);

// Version should match your Rust crate version
const VERSION = "0.1.0";
const BINARY_NAME = "iconmate";

function getPlatformInfo() {
  const platform = process.platform;
  const arch = process.arch;

  // Map Node.js platform/arch to Rust target triples
  const platformMap = {
    darwin: {
      x64: "x86_64-apple-darwin",
      arm64: "aarch64-apple-darwin",
    },
    linux: {
      x64: "x86_64-unknown-linux-gnu",
      arm64: "aarch64-unknown-linux-gnu",
    },
    win32: {
      x64: "x86_64-pc-windows-msvc",
    },
  };

  if (!platformMap[platform]) {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  if (!platformMap[platform][arch]) {
    throw new Error(`Unsupported architecture: ${arch} on ${platform}`);
  }

  const target = platformMap[platform][arch];
  const extension = platform === "win32" ? ".zip" : ".tar.xz";
  const binaryName = platform === "win32" ? `${BINARY_NAME}.exe` : BINARY_NAME;

  return {
    target,
    extension,
    binaryName,
    filename: `${BINARY_NAME}-${target}${extension}`,
    url: `https://github.com/Blankeos/iconmate/releases/download/v${VERSION}/${BINARY_NAME}-${target}${extension}`,
  };
}

async function downloadFile(url, dest) {
  console.log(`Downloading ${url}...`);

  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(dest);

    https
      .get(url, (response) => {
        if (response.statusCode === 302 || response.statusCode === 301) {
          // Handle redirect
          return downloadFile(response.headers.location, dest)
            .then(resolve)
            .catch(reject);
        }

        if (response.statusCode !== 200) {
          reject(
            new Error(
              `Failed to download: ${response.statusCode} ${response.statusMessage}`,
            ),
          );
          return;
        }

        response.pipe(file);

        file.on("finish", () => {
          file.close();
          resolve();
        });

        file.on("error", (err) => {
          fs.unlink(dest, () => {}); // Clean up on error
          reject(err);
        });
      })
      .on("error", reject);
  });
}

function extractArchive(archivePath, extractDir, platformInfo) {
  console.log("Extracting binary...");

  if (platformInfo.extension === ".zip") {
    // For Windows .zip files
    try {
      // Try using PowerShell on Windows
      execSync(
        `powershell -command "Expand-Archive -Path '${archivePath}' -DestinationPath '${extractDir}' -Force"`,
        { stdio: "inherit" },
      );
    } catch {
      // Fallback: try using unzip if available
      execSync(`unzip -o "${archivePath}" -d "${extractDir}"`, {
        stdio: "inherit",
      });
    }
  } else {
    // For .tar.xz files
    execSync(`tar -xf "${archivePath}" -C "${extractDir}"`, {
      stdio: "inherit",
    });
  }
}

async function install() {
  try {
    const platformInfo = getPlatformInfo();
    const binDir = path.join(__dirname, "bin");
    const archivePath = path.join(__dirname, platformInfo.filename);
    const binaryPath = path.join(binDir, platformInfo.binaryName);

    // Create bin directory
    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    // Download the archive
    await downloadFile(platformInfo.url, archivePath);

    // Extract the binary
    extractArchive(archivePath, __dirname, platformInfo);

    // Move binary to bin directory and make executable
    const extractedBinaryPath = path.join(__dirname, platformInfo.binaryName);
    if (fs.existsSync(extractedBinaryPath)) {
      fs.renameSync(extractedBinaryPath, binaryPath);
    } else {
      // Binary might be in a subdirectory after extraction
      const subdirPath = path.join(
        __dirname,
        `${BINARY_NAME}-${platformInfo.target}`,
        platformInfo.binaryName,
      );
      if (fs.existsSync(subdirPath)) {
        fs.renameSync(subdirPath, binaryPath);
        // Clean up extracted directory
        fs.rmSync(path.dirname(subdirPath), { recursive: true, force: true });
      } else {
        throw new Error("Binary not found after extraction");
      }
    }

    // Make binary executable on Unix systems
    if (process.platform !== "win32") {
      fs.chmodSync(binaryPath, "755");
    }

    // Clean up archive
    fs.unlinkSync(archivePath);

    // Clean up any remaining extracted files
    ["README.md"].forEach((file) => {
      const filePath = path.join(__dirname, file);
      if (fs.existsSync(filePath)) {
        fs.unlinkSync(filePath);
      }
    });

    console.log(`✅ iconmate v${VERSION} installed successfully!`);
  } catch (error) {
    console.error("❌ Installation failed:", error.message);
    console.error("\nYou can install iconmate directly using:");
    console.error(
      'curl --proto "=https" --tlsv1.2 -LsSf https://github.com/Blankeos/iconmate/releases/latest/download/iconmate-installer.sh | sh',
    );
    process.exit(1);
  }
}

// Only run install if this script is executed directly
if (require.main === module) {
  install();
}
