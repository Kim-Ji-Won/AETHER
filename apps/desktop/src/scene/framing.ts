// Fitted neural silhouette including .07 deformation; local Plasma envelope remains 2.65.
export const sceneFitRadius = 2.25;
export type Quality = 'balanced' | 'high';
export function frameScene(width: number, height: number, deviceDpr: number, quality: Quality, referenceFraming = false) {
  const w = Math.max(1, width), h = Math.max(1, height);
  const fov = 38;
  const vertical = fov * Math.PI / 180;
  const horizontal = 2 * Math.atan(Math.tan(vertical / 2) * w / h);
  // Conservative bounding sphere includes neural branches, deformation and every camera angle.
  const radius = referenceFraming ? 2.65 : sceneFitRadius;
  const distance = radius / Math.sin(Math.min(vertical, horizontal) / 2) * (referenceFraming ? 1.08 : 1.025);
  return { width: w, height: h, aspect: w / h, fov, distance, near: .1, far: distance + 30,
    dpr: Math.max(.5, Math.min(deviceDpr || 1, quality === 'high' ? 1.5 : 1.25)) };
}
export function renderPolicy(visible: boolean, focused: boolean, motion: boolean, reduced: boolean) {
  return { draw: visible && focused, animate: visible && focused && motion && !reduced };
}
