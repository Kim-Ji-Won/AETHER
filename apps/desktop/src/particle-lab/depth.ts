// CPU reference + matching GPU formula. Diagnostics/tests only; never rebuild particle buffers per frame.
export const forwardLimit=2.8;
export const particleBound=5.6; // original conservative 2.4 + 2.8 * 1.12 < 5.6, any camera direction
const smooth=(a:number,b:number,x:number)=>{const t=Math.min(1,Math.max(0,(x-a)/(b-a)));return t*t*(3-2*t);};
export function forwardDistance(traits:readonly number[],base:readonly number[],fast:number,slow:number,speaking:number) {
 const group=smooth(.62,.82,traits[3])*(.24+.76*smooth(.82,.98,traits[3]));
 const front=.18+.82*smooth(-.55,.7,base[2]);
 const interior=1-.7*smooth(.35,1.15,Math.hypot(base[0],base[1]));
 const lag=.2+.65*traits[1],response=fast*(1-lag)+slow*lag;
 return forwardLimit*group*front*interior*response*(1+.12*speaking);
}
export function safeForward(distance:number,view:readonly number[],projectionX:number,projectionY:number) {
 const guard=Math.max(1.5,Math.abs(view[0])*projectionX/.86,Math.abs(view[1])*projectionY/.82);
 return Math.min(distance,Math.max(0,-view[2]-guard));
}
export const forwardGLSL=/* glsl */`
uniform float uForwardFast; uniform float uForwardSlow; uniform float uForwardEnabled;
float forwardDistance(vec4 seed,vec3 base){
 float group=smoothstep(.62,.82,seed.w)*(.24+.76*smoothstep(.82,.98,seed.w));
 float front=.18+.82*smoothstep(-.55,.7,base.z);
 float interior=1.0-.7*smoothstep(.35,1.15,length(base.xy));
 float response=mix(uForwardFast,uForwardSlow,.2+.65*seed.y);
 return ${forwardLimit.toFixed(1)}*group*front*interior*response*(1.0+.12*uSpeaking);
}
float safeForward(float distance,vec4 view){
 float guard=max(1.5,max(abs(view.x)*projectionMatrix[0][0]/.86,abs(view.y)*projectionMatrix[1][1]/.82));
 return min(distance,max(0.0,-view.z-guard));
}
`;
