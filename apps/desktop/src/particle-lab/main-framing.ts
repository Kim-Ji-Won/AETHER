// Main-only, layout-dependent projection. No voice/time input or changes to the
// camera position, look target, point-size formula, depth guard, or Lab framing.
const smooth = (a:number,b:number,x:number) => {
  const t=Math.min(1,Math.max(0,(x-a)/(b-a)));
  return t*t*(3-2*t);
};
export function mainProjectionZoom(width:number,height:number) {
  if(!Number.isFinite(width)||!Number.isFinite(height))return 1;
  return 1+.12*smooth(480,740,width)*smooth(300,480,height);
}
