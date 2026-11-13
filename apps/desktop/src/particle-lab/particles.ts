import * as THREE from 'three';
import {forwardGLSL,particleBound} from './depth';
export const particleCounts={balanced:12000,high:20000} as const;
// Stable prefix across quality levels. Static volume buffers, no per-frame CPU geometry updates.
export function particleData(count:number) {
  let seed=3901;
  const random=()=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/4294967296;};
  const positions=new Float32Array(count*3),traits=new Float32Array(count*4);
  for(let i=0;i<count;i++) {
    const z=random()*2-1,a=random()*Math.PI*2,u=random(),r=Math.pow(u,.46)*(.86+.38*random()),xy=Math.sqrt(1-z*z);
    positions.set([r*xy*Math.cos(a),r*z*.96,r*xy*Math.sin(a)],i*3);
    traits.set([random(),random(),random(),random()],i*4);
  }
  return {positions,traits};
}
const vertex=/* glsl */`
attribute vec4 traits;
uniform float uTime; uniform float uLevel; uniform float uThinking; uniform float uDpr; uniform float uDensity;
uniform float uRadius; uniform float uSpeaking; uniform float uCameraDistance;
${forwardGLSL}
varying float vAlpha; varying float vTint; varying float vDepth; varying float vAccent; varying float vRadial;
void main(){
  vec3 n=normalize(position+vec3(.0001));
  float t=uTime, phase=traits.x*6.283185, baseRadius=length(position);
  float local=.2+.8*pow(.5+.5*sin(dot(n,vec3(2.3,-1.8,2.9))+t*(.7+.22*uSpeaking)+phase*.25),2.0);
  // Shared damped radius + nonuniform particle response, not object.scale or opacity fading.
  float radial=1.0+(uRadius-1.0)*(.78+.22*traits.y);
  float response=uLevel*(.025+.095*traits.y)*local;
  vec3 tangent=normalize(cross(n,vec3(.35+traits.x,.8,-.3))+vec3(.0001));
  vec3 drift=vec3(sin(t*.43+phase+position.y*2.0),sin(t*.37+phase*1.7+position.z*2.0),cos(t*.31+phase*1.3+position.x*2.0));
  vec3 p=position*radial+n*response;
  p+=drift*(.012+.022*traits.z+.11*uThinking+.025*uLevel)*(.35+.65*traits.y);
  p+=tangent*sin(t*(.45+traits.y*.4)+phase)*(.012+.18*uThinking+(.03+.025*uSpeaking)*uLevel);
  vec3 focus=vec3(sin(t*.27)*.28,cos(t*.33)*.18,sin(t*.21)*.24);
  p+=uThinking*(focus-position)*(.08+.12*traits.w)*(.5+.5*sin(t*.64+phase));
  vec4 view=modelViewMatrix*vec4(p,1.0);
  // In view space the camera looks down -Z: +Z moves actual vertices toward it.
  // Camera, object scale, base buffers and radial contraction remain unchanged.
  view.z+=uForwardEnabled*safeForward(forwardDistance(traits,position),view);
  gl_Position=projectionMatrix*view;
  float front=clamp(.5+(view.z+uCameraDistance)/2.6,0.0,1.0);
  float medium=step(.72,traits.z),accent=step(.98,traits.z);
  float size=mix(1.2+traits.z*.75,2.3+traits.w*.6,medium)+accent*.9;
  gl_PointSize=clamp(size*uDpr*6.0/max(.2,-view.z)*mix(.8,1.12,front),.65,5.5*uDpr);
  float edge=1.0-.45*smoothstep(.72,1.25,baseRadius);
  vAlpha=mix(.45+.25*traits.w,.8+.18*traits.w,medium)*edge*uDensity;
  vAlpha*=mix(.6,1.0,front);
  vTint=traits.x;vDepth=front;vAccent=accent;vRadial=baseRadius;
}`;
const fragment=/* glsl */`
varying float vAlpha; varying float vTint; varying float vDepth; varying float vAccent; varying float vRadial;
void main(){
  float r=length(gl_PointCoord-.5)*2.0;
  if(r>1.0)discard;
  float soft=pow(1.0-r*r,mix(2.8,1.7,vDepth));
  vec3 color=mix(vec3(.025,.25,.9),vec3(.25,.7,1.0),smoothstep(.15,.85,vRadial)*(.5+.5*vTint));
  color=mix(color,vec3(.62,.9,1.0),vAccent*.7);
  gl_FragColor=vec4(color*1.9,soft*vAlpha);
  #include <tonemapping_fragment>
  #include <colorspace_fragment>
}`;
export function createParticles(quality:'balanced'|'high') {
  const count=particleCounts[quality],data=particleData(count);
  const geometry=new THREE.BufferGeometry();
  geometry.setAttribute('position',new THREE.BufferAttribute(data.positions,3));
  geometry.setAttribute('traits',new THREE.BufferAttribute(data.traits,4));
  geometry.boundingSphere=new THREE.Sphere(new THREE.Vector3(),particleBound);
  const uniforms={uForwardFast:{value:0},uForwardSlow:{value:0},uForwardEnabled:{value:1},uTime:{value:0},uLevel:{value:0},uThinking:{value:0},uRadius:{value:1},uSpeaking:{value:0},uCameraDistance:{value:6.5},uDpr:{value:1},uDensity:{value:12000/count}};
  const material=new THREE.ShaderMaterial({uniforms,vertexShader:vertex,fragmentShader:fragment,
    transparent:true,depthTest:true,depthWrite:false,blending:THREE.AdditiveBlending});
  const points=new THREE.Points(geometry,material);
  let disposed=false;
  return {points,uniforms,count,bytes:data.positions.byteLength+data.traits.byteLength,
    dispose(){if(disposed)return;disposed=true;geometry.dispose();material.dispose();}};
}
