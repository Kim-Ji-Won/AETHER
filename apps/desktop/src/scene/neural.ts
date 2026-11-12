import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import type { VisualSnapshot } from './visual-state';

// Shared art-direction values for later Plasma / DOM integration. No runtime ownership.
export const neuralPalette = { ice: 0xa6eaff, cyan: 0x45bccf, violet: 0x8988b3, white: 0xd8f6ff };
export const neuralEnvelope = { radius: 2.65, plasmaRadius: 1.45, maxDisplacement: .07 };

const vertexShader = /* glsl */`
  uniform float uTime;
  attribute float pathPhase;
  varying vec2 vUv;
  varying float vPhase;
  varying vec3 vNormal;
  varying vec3 vPosition;
  void main() {
    // Continuous field shared by every curve and node keeps junctions connected.
    vec3 p = position;
    p += .035 * vec3(sin(p.y*1.6+uTime*.31), sin(p.z*1.4+uTime*.27), sin(p.x*1.3+uTime*.23));
    vPosition = p;
    vNormal = normalize(normalMatrix * normal);
    vUv = uv; vPhase = pathPhase;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.0);
  }
`;
const fragmentShader = /* glsl */`
  uniform float uTime;
  uniform float uEnergy;
  uniform float uSpeed;
  uniform float uNode;
  uniform vec3 uColor;
  varying vec2 vUv;
  varying float vPhase;
  varying vec3 vNormal;
  varying vec3 vPosition;
  void main() {
    float phase = fract(uTime*uSpeed + vPhase);
    float wave = exp(-pow((vUv.x-phase)*16.0, 2.0));
    float activation = .5 + .5*sin(uTime*.85 + vPhase*6.283);
    float signal = mix(wave, activation*.65, uNode);
    float key = .34 + .66*max(0.0, dot(normalize(vNormal), normalize(vec3(-.4,.6,1.0))));
    float center = exp(-length(vPosition)*1.15);
    // Rear connections are dimmer; opaque depth-tested tubes retain crossings/occlusion.
    float depth = clamp(.7 + vPosition.z*.23, .32, 1.0);
    vec3 color = uColor*(key*.52 + center*.65)*depth;
    color += mix(uColor, vec3(.8,.96,1.0), .6)*signal*uEnergy*(.35+center)*depth;
    gl_FragColor = vec4(color, 1.0);
    #include <tonemapping_fragment>
    #include <colorspace_fragment>
  }
`;

export interface NeuralResources {
  geometries: THREE.BufferGeometry[];
  materials: THREE.Material[];
}

// Stable topology, no random regeneration on resize or frame updates.
export function createNeuralStructure(parent: THREE.Group, owned: NeuralResources) {
  const startGeometry = owned.geometries.length;
  const startMaterial = owned.materials.length;
  const layers: THREE.Vector3[][] = [];
  const batches: THREE.BufferGeometry[][] = [[], [], [], []];
  const uniforms = { uTime: { value: 0 }, uEnergy: { value: .7 }, uSpeed: { value: .12 } };
  let connections = 0, disposed = false;
  const hash = (i: number) => { const n=Math.sin(i*127.1+311.7)*43758.5453; return n-Math.floor(n); };
  function phase(g: THREE.BufferGeometry, value: number) {
    g.setAttribute('pathPhase', new THREE.Float32BufferAttribute(new Float32Array(g.attributes.position.count).fill(value), 1));
    return g;
  }
  function connect(a: THREE.Vector3, b: THREE.Vector3, layer: number, seed: number) {
    const delta=b.clone().sub(a);
    const bend=new THREE.Vector3(delta.y, -delta.x, delta.x*.35).normalize().multiplyScalar(delta.length()*(.16+hash(seed)*.12));
    const curve=new THREE.CatmullRomCurve3([a, a.clone().lerp(b,.3).add(bend),
      a.clone().lerp(b,.7).addScaledVector(bend,.65), b]);
    batches[Math.min(layer,2)].push(phase(new THREE.TubeGeometry(curve, 24, layer===0 ? .012 : layer===1 ? .009 : .0055, 5, false), hash(seed+7)));
    connections++;
  }
  try {
    for (let layer=0; layer<4; layer++) {
      const count=[12,30,48,24][layer], radius=[.46,1.04,1.72,2.15][layer];
      const points: THREE.Vector3[]=[];
      for (let i=0; i<count; i++) {
        const y=1-2*(i+.5)/count, angle=i*2.399963+layer*.71;
        const r=Math.sqrt(1-y*y), variation=.88+hash(i+layer*51)*.17;
        const p=new THREE.Vector3(Math.cos(angle)*r*radius*1.04, y*radius*.96, Math.sin(angle)*r*radius*.8).multiplyScalar(variation);
        points.push(p);
        const node=phase(new THREE.SphereGeometry(layer===0 ? .035 : layer===1 ? .025 : .017, 8, 6), hash(i+layer*100));
        node.translate(p.x,p.y,p.z); batches[3].push(node);
      }
      layers.push(points);
      points.forEach((p,i) => {
        const candidates=(layer ? layers[layer-1] : points).filter(q=>q!==p).sort((a,b)=>p.distanceToSquared(a)-p.distanceToSquared(b));
        connect(candidates[0],p,Math.max(0,layer-1),i+layer*100);
        if (layer<3 && i%2===0) connect(candidates[1],p,Math.max(0,layer-1),i+layer*173);
        if (layer===1 || layer===2) {
          const neighbor=points.filter(q=>q!==p).sort((a,b)=>p.distanceToSquared(a)-p.distanceToSquared(b))[1];
          if (i%2===0) connect(p,neighbor,layer,i+layer*211);
        }
      });
    }
    const colors=[neuralPalette.ice,neuralPalette.cyan,neuralPalette.violet,neuralPalette.white];
    batches.forEach((parts,i) => {
      const geometry=mergeGeometries(parts);
      if (!geometry) throw new Error('Neural geometry 결합 실패');
      owned.geometries.push(geometry);
      const material=new THREE.ShaderMaterial({ uniforms: { ...uniforms, uColor: { value: new THREE.Color(colors[i]) }, uNode: { value: i===3 ? 1 : 0 } }, vertexShader, fragmentShader });
      owned.materials.push(material);
      const mesh=new THREE.Mesh(geometry,material); mesh.frustumCulled=false; parent.add(mesh);
    });
  } finally { batches.forEach(parts=>parts.forEach(g=>g.dispose())); }
  const geometries=owned.geometries.slice(startGeometry), materials=owned.materials.slice(startMaterial);
  const bufferBytes=geometries.reduce((sum,g)=>sum+Object.values(g.attributes).reduce((n,a)=>n+a.array.byteLength,0)+(g.index?.array.byteLength??0),0);
  return {
    nodeCount: layers.reduce((sum,l)=>sum+l.length,0), connections, bufferBytes,
    triangles: geometries.reduce((sum,g)=>sum+(g.index?.count??g.attributes.position.count)/3,0),
    update(delta: number) { if (!disposed) uniforms.uTime.value+=Math.max(0,Math.min(delta,.06)); },
    setVisual(snapshot: VisualSnapshot) {
      if (disposed) return;
      const preview=snapshot.source==='preview';
      uniforms.uEnergy.value=snapshot.illuminationEnabled ? 1.05 : .7;
      uniforms.uSpeed.value=preview && snapshot.phase==='thinking' ? .2 : preview && snapshot.phase==='speaking' ? .16 : .12;
      if (preview && (snapshot.phase==='error' || snapshot.phase==='awaiting_approval')) uniforms.uEnergy.value=.35;
    },
    // Renderer owns actual resource disposal; this only invalidates future updates.
    invalidate() { disposed=true; },
    inspection: { geometries, materials, uniforms },
  };
}
