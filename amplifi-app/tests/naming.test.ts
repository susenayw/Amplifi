import { test } from "node:test";
import assert from "node:assert/strict";
import {
  comparisonClipPath,
  outputFilename,
  parseProgressLine,
  targetDimensions,
} from "../src/naming.ts";

test("batch images with the same stem keep separate output names", () => {
  const png = outputFilename("photo.png", true, "", 4, false, "model", "png");
  const jpg = outputFilename("photo.jpg", true, "", 4, false, "model", "png");
  assert.notEqual(png, jpg);
  assert.equal(png, "photo.png_amplifi_4x_model.png");
});

test("single, double and custom width names describe the output", () => {
  assert.equal(outputFilename("a.jpg", false, "", 4, true, "model", "png"), "a_amplifi_16x_model.png");
  assert.equal(outputFilename("a.jpg", false, "800", 4, false, "model", "webp"), "a_amplifi_800px_model.webp");
});

test("target dimensions reflect scale, double upscale, and custom width", () => {
  assert.deepEqual(targetDimensions(32, 32, 4, false, ""), { width: 128, height: 128 });
  assert.deepEqual(targetDimensions(32, 32, 4, true, ""), { width: 512, height: 512 });
  assert.deepEqual(targetDimensions(100, 50, 4, false, "250"), { width: 250, height: 125 });
  assert.equal(targetDimensions(0, 32, 4, false, ""), null);
});

test("progress parsing and comparison clip paths handle slider and lens modes", () => {
  assert.equal(parseProgressLine("0.00%"), 0);
  assert.equal(parseProgressLine("62.5%"), 62.5);
  assert.equal(parseProgressLine("100.00"), 100);
  assert.equal(parseProgressLine("Pass 1 of 2"), null);
  assert.equal(comparisonClipPath("slider", 40, 120, null), "inset(0 60% 0 0)");
  assert.equal(comparisonClipPath("lens", 40, 120, { x: 10, y: 20 }), "circle(120px at 10px 20px)");
  assert.equal(comparisonClipPath("lens", 40, 120, null), "circle(0px at 0 0)");
});
