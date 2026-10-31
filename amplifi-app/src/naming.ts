/** A batch includes the input extension so photo.jpg and photo.png remain distinct. */
export function outputFilename(
  filename: string,
  batch: boolean,
  width: string,
  scale: number,
  doubleUpscale: boolean,
  model: string,
  format: string,
): string {
  const name = batch ? filename : filename.replace(/\.[^.]+$/, "");
  const size = width ? `${width}px` : `${doubleUpscale ? scale * scale : scale}x`;
  return `${name}_amplifi_${size}_${model}.${format}`;
}

export function targetDimensions(
  inputWidth: number,
  inputHeight: number,
  scale: number,
  doubleUpscale: boolean,
  customWidth: string,
): { width: number; height: number } | null {
  if (inputWidth <= 0 || inputHeight <= 0) return null;
  const parsedWidth = Number(customWidth);
  if (customWidth.trim() !== "" && Number.isFinite(parsedWidth) && parsedWidth > 0) {
    const width = Math.round(parsedWidth);
    const height = Math.max(1, Math.round(width * (inputHeight / inputWidth)));
    return { width, height };
  }
  const factor = doubleUpscale ? scale * scale : scale;
  return { width: inputWidth * factor, height: inputHeight * factor };
}

export function parseProgressLine(line: string): number | null {
  const match = /^(\d+(?:\.\d+)?)%?$/.exec(line.trim());
  if (!match) return null;
  return Math.min(100, Math.max(0, Number(match[1])));
}

export function comparisonClipPath(
  viewMode: "slider" | "lens",
  position: number,
  lensSize: number,
  lensPoint: { x: number; y: number } | null,
): string {
  if (viewMode === "slider") return `inset(0 ${100 - position}% 0 0)`;
  return lensPoint ? `circle(${lensSize}px at ${lensPoint.x}px ${lensPoint.y}px)` : "circle(0px at 0 0)";
}
