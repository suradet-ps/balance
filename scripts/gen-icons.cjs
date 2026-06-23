#!/usr/bin/env node
/**
 * gen-icons.cjs — Tauri app-icon generator (uses @resvg/resvg-js)
 *
 * Reads icon-master.svg from project root and generates all Tauri icon sizes.
 *
 * Usage:
 *   node scripts/gen-icons.cjs          # Normal mode
 *   node scripts/gen-icons.cjs --silent # Quiet mode
 */

"use strict";

const { Resvg } = require("@resvg/resvg-js");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const ROOT = path.resolve(__dirname, "..");
const ICONS_DIR = path.join(ROOT, "src-tauri", "icons");
const SOURCE_SVG = path.join(ROOT, "icon-master.svg");
const SOURCE_PNG = path.join(ICONS_DIR, "icon.png");
const IS_SILENT = process.argv.includes("--silent");

const log = (msg) => !IS_SILENT && console.log(msg);
const error = (msg) => console.error(msg);

(async () => {
  try {
    // 1. Read source SVG
    if (!fs.existsSync(SOURCE_SVG)) {
      error(`Source SVG not found: ${SOURCE_SVG}`);
      process.exit(1);
    }

    const svgContent = fs.readFileSync(SOURCE_SVG, "utf-8");
    log(`Read ${path.basename(SOURCE_SVG)} (${svgContent.length} bytes)`);

    // 2. Render SVG → PNG (1024×1024)
    log("Rendering SVG → icon.png (1024×1024)…");

    const resvg = new Resvg(svgContent, {
      fitTo: { mode: "width", value: 1024 },
      imageRendering: 1,
      shapeRendering: 2,
      textRendering: 2,
    });

    const pngBuffer = resvg.render().asPng();

    fs.mkdirSync(ICONS_DIR, { recursive: true });
    fs.writeFileSync(SOURCE_PNG, pngBuffer);
    log(`Saved ${path.basename(SOURCE_PNG)} (${Math.round(pngBuffer.length / 1024)} KB)`);

    // 3. Run tauri icon generator
    log("Running tauri icon generator…");
    execSync(`npx tauri icon "${SOURCE_PNG}"`, {
      cwd: ROOT,
      stdio: IS_SILENT ? "pipe" : "inherit",
      timeout: 120_000,
    });

    log("All icons generated successfully in src-tauri/icons/");
    log("Rebuild the app to apply: npm run tauri build");
  } catch (err) {
    error("Process failed:");
    error(err.message || err);
    process.exit(1);
  }
})();
