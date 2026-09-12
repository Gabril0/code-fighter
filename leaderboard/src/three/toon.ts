import * as THREE from "three";

let gradient = null;

export function toonGradient(steps = 4) {
  if (!gradient) {
    const data = new Uint8Array(steps);
    for (let i = 0; i < steps; i += 1) {
      data[i] = Math.round((i / (steps - 1)) * 255);
    }
    gradient = new THREE.DataTexture(data, steps, 1, THREE.RedFormat);
    gradient.minFilter = THREE.NearestFilter;
    gradient.magFilter = THREE.NearestFilter;
    gradient.generateMipmaps = false;
    gradient.needsUpdate = true;
  }
  return gradient;
}

export function pixelate(texture) {
  if (!texture) return texture;
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestMipmapNearestFilter;
  texture.generateMipmaps = true;
  texture.anisotropy = 1;
  if (texture.image) texture.needsUpdate = true;
  return texture;
}

export function toToon(source) {
  const toon = new THREE.MeshToonMaterial({
    color: source.color ? source.color.clone() : new THREE.Color(0xffffff),
    map: pixelate(source.map) ?? null,
    gradientMap: toonGradient(),
    transparent: source.transparent ?? false,
    opacity: source.opacity ?? 1,
    side: source.side ?? THREE.FrontSide,
    alphaTest: source.alphaTest ?? 0,
  });
  toon.name = source.name;
  return toon;
}

export function toonify(root) {
  root.traverse((child) => {
    if (!child.isMesh || !child.material) return;
    const materials = Array.isArray(child.material) ? child.material : [child.material];
    const converted = materials.map((material) =>
      material.isMeshToonMaterial ? material : toToon(material),
    );
    child.material = Array.isArray(child.material) ? converted : converted[0];
  });
}
