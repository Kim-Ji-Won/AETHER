import * as THREE from 'three';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';
import type { VisualSnapshot } from './visual-state';

// Shared art-direction values for later Plasma / DOM integration. No runtime ownership.
export const neuralPalette = { ice: 0x79d8ef, cyan: 0x45bccf, violet: 0x8988b3, white: 0xd8f6ff };
export const neuralEnvelope = { radius: 2.65, plasmaRadius: 1.45, maxDisplacement: .07 };

const vertexShader = /* glsl */`
  uniform float uTime;
  attribute vec2 signalRoute;
  varying vec2 vRoute;
  varying vec3 vNormal;
  varying vec3 vPosition;
  void main() {
    // Shared continuous field preserves branch junctions, with no whole-Core rotation.
    vec3 p = position;
    p += .04 * vec3(sin(p.y*1.8+uTime*.36), sin(p.z*1.6+uTime*.29), sin(p.x*1.4+uTime*.33));
    vPosition = p;
    vNormal = normalize(normalMatrix * normal);
    vRoute = signalRoute;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.0);
  }
`;
const fragmentShader = /* glsl */`
  uniform float uTime;
  uniform float uEnergy;
  uniform float uNode;
  uniform vec3 uColor;
  varying vec2 vRoute;
  varying vec3 vNormal;
  varying vec3 vPosition;
  void main() {
    // Eight staggered root channels; travel time is continuous across primary junctions.
    float age = mod(uTime - vRoute.x - vRoute.y*.79 + 80.0, 8.0);
    float signal = smoothstep(0.0, .12, age) * exp(-max(0.0,age-.12)*5.0);
    float key = .38 + .62*max(0.0, dot(normalize(vNormal), normalize(vec3(-.4,.6,1.0))));
    float center = exp(-dot(vPosition,vPosition)*.7);
    float depth = clamp(.73 + vPosition.z*.18, .38, 1.0);
    vec3 color = uColor*(key*.61 + center*.3)*depth;
    color += mix(uColor, vec3(.65,.92,1.0), .32)*signal*uEnergy*(.8+uNode*.25)*depth;
    gl_FragColor = vec4(color, 1.0);
    #include <tonemapping_fragment>
    #include <colorspace_fragment>
  }
`;

export interface NeuralResources {
  geometries: THREE.BufferGeometry[];
  materials: THREE.Material[];
}

interface NeuralNode { position: THREE.Vector3; arrival: number; channel: number; layer: number }

// Fixed topology: nucleus (8 roots + 40 inner nodes), 42 intermediate, 24 peripheral.
// This redistributes the existing 114 nodes rather than adding density everywhere.
export function createNeuralStructure(parent: THREE.Group, owned: NeuralResources) {
  const startGeometry = owned.geometries.length;
  const startMaterial = owned.materials.length;
  const layers: NeuralNode[][] = [];
  const batches: THREE.BufferGeometry[][] = [[], [], [], []];
  const uniforms = { uTime: { value: 0 }, uEnergy: { value: 1 }, uSpeed: { value: .85 } };
  const primaryRoutes: { from: NeuralNode; to: NeuralNode }[] = [];
  const edgeKeys = new Set<string>();
  const nodes: NeuralNode[] = [];
  const layerConnections = [0,0,0];
  let disposed = false;
  const hash = (i: number) => { const n=Math.sin(i*127.1+311.7)*43758.5453; return n-Math.floor(n); };
  function route(g: THREE.BufferGeometry, a: NeuralNode, b: NeuralNode, node=false) {
    const uv=g.attributes.uv;
    const values=new Float32Array(uv.count*2);
    for(let i=0;i<uv.count;i++) {
      values[i*2]=node ? b.arrival : THREE.MathUtils.lerp(a.arrival,b.arrival,uv.getX(i));
      values[i*2+1]=b.channel;
    }
    g.setAttribute('signalRoute',new THREE.Float32BufferAttribute(values,2));
    return g;
  }
  function connect(a: NeuralNode, b: NeuralNode, layer: number, seed: number) {
    const ids=[nodes.indexOf(a),nodes.indexOf(b)].sort((x,y)=>x-y);
    const key=ids.join(':'); if(edgeKeys.has(key))return;
    edgeKeys.add(key); layerConnections[layer]++;
    const delta=b.position.clone().sub(a.position);
    // Small alternating 3D bends avoid the previous common planar swirl.
    const bend=new THREE.Vector3(hash(seed)-.5,hash(seed+9)-.5,hash(seed+21)-.5)
      .normalize().multiplyScalar(delta.length()*.13);
    const curve=new THREE.CatmullRomCurve3([a.position,
      a.position.clone().lerp(b.position,.33).add(bend),
      a.position.clone().lerp(b.position,.7).addScaledVector(bend,.65),b.position]);
    batches[layer].push(route(new THREE.TubeGeometry(curve,24,[.013,.009,.005][layer],5,false),a,b));
  }
  try {
    for(let layer=0;layer<4;layer++) {
      const count=[8,40,42,24][layer];
      const points: NeuralNode[]=[];
      for(let i=0;i<count;i++) {
        const y=1-2*(i+.5)/count, angle=i*2.399963+layer*.47;
        const r=Math.sqrt(1-y*y);
        const radius=layer===0 ? .16 : layer===1 ? .25+.75*Math.pow(((i*17)%count+.5)/count,.65)
          : layer===2 ? 1.36+hash(i+73)*.16 : 1.9+hash(i+191)*.16;
        const position=new THREE.Vector3(Math.cos(angle)*r*radius, y*radius*1.02, Math.sin(angle)*r*radius*.92);
        const nearest=layer ? [...layers[layer-1]].sort((a,b)=>position.distanceToSquared(a.position)-position.distanceToSquared(b.position))[0] : null;
        const n: NeuralNode={position, layer, channel: nearest?.channel??i,
          arrival: nearest ? nearest.arrival+nearest.position.distanceTo(position)*.72+.08 : 0};
        points.push(n); nodes.push(n);
        if(nearest)primaryRoutes.push({from:nearest,to:n});
        const sphere=new THREE.SphereGeometry(layer<2 ? .024 : layer===2 ? .02 : .012,8,6);
        sphere.translate(position.x,position.y,position.z);
        batches[3].push(route(sphere,n,n,true));
      }
      layers.push(points);
    }
    for(const {from,to} of primaryRoutes)connect(from,to,Math.max(0,to.layer-1),nodes.indexOf(to));
    // Compact nucleus cross-links bridge several depths, without filling the outer shell.
    layers[0].forEach((n,i)=>connect(n,layers[0][(i+1)%8],0,i+400));
    layers[1].forEach((n,i)=>{
      const nearest=layers[1].filter(p=>p!==n).sort((a,b)=>n.position.distanceToSquared(a.position)-n.position.distanceToSquared(b.position));
      for(const other of nearest.slice(0,2)) {
        const [a,b]=[n,other].sort((x,y)=>x.arrival-y.arrival);
        connect(a,b,0,i+500);
      }
    });
    layers[2].forEach((n,i)=>{
      if(i%3!==0)return;
      const other=layers[1].filter(p=>p.channel===n.channel).sort((a,b)=>n.position.distanceToSquared(a.position)-n.position.distanceToSquared(b.position))[1];
      if(other)connect(other,n,1,i+600);
    });
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
    nodeCount: nodes.length, connections: edgeKeys.size, bufferBytes,
    triangles: geometries.reduce((sum,g)=>sum+(g.index?.count??g.attributes.position.count)/3,0),
    update(delta: number) { if (!disposed) uniforms.uTime.value+=Math.max(0,Math.min(delta,.06))*uniforms.uSpeed.value; },
    setVisual(snapshot: VisualSnapshot) {
      if (disposed) return;
      const preview=snapshot.source==='preview';
      uniforms.uEnergy.value=snapshot.illuminationEnabled ? 1.3 : 1;
      uniforms.uSpeed.value=preview && snapshot.phase==='thinking' ? 1.35 : preview && snapshot.phase==='speaking' ? 1.1 : .85;
      if(preview && snapshot.phase==='thinking')uniforms.uEnergy.value*=1.2;
      if (preview && (snapshot.phase==='error' || snapshot.phase==='awaiting_approval')) uniforms.uEnergy.value=.35;
    },
    invalidate() { disposed=true; },
    inspection: { geometries, materials, uniforms, nodes, primaryRoutes, layerConnections },
  };
}
