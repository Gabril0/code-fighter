import * as THREE from "three";

const BEAM_TOP = 13.5;
const FLOOR_LIGHT_Y = 1.36;
const RIM_INTENSITY = 24;
const RIM_ANGLE = Math.PI / 5;
const RIM_SPREAD = 4.6;
const RIM_TOP = 7.2;
const RIM_BACK = -8.4;
const RIM_AIM = 1.5;

export const THEME = {
  brand: 0x10b981,
  brandLight: 0x6ee7b7,
  brandDeep: 0x047857,
  impact: 0xf43f5e,
  gold: 0xffc447,
  white: 0xffffff,
};

const CORNER_WHITENESS = 0.32;

export function softLight(hex) {
  return new THREE.Color(hex).lerp(new THREE.Color(0xffffff), CORNER_WHITENESS);
}

const QUANTISE = 5;
const stepped = (value) => Math.ceil(value * QUANTISE) / QUANTISE;

let starTexture = null;
let sparkTexture = null;

const TRAIL_SAMPLES = 5;
const SIZE_DECAY_START = 0.85;
const CONE_SPREAD = Math.PI / 3.4;

function inCone(direction, spread) {
  const axis = direction.clone().normalize();
  const cosLimit = Math.cos(spread);
  const cosTheta = cosLimit + Math.random() * (1 - cosLimit);
  const sinTheta = Math.sqrt(Math.max(0, 1 - cosTheta * cosTheta));
  const phi = Math.random() * Math.PI * 2;

  const helper = Math.abs(axis.x) > 0.9 ? new THREE.Vector3(0, 1, 0) : new THREE.Vector3(1, 0, 0);
  const right = new THREE.Vector3().crossVectors(helper, axis).normalize();
  const up = new THREE.Vector3().crossVectors(axis, right);

  return new THREE.Vector3()
    .addScaledVector(right, Math.cos(phi) * sinTheta)
    .addScaledVector(up, Math.sin(phi) * sinTheta)
    .addScaledVector(axis, cosTheta);
}
const TRAIL_STEP = 0.028;

function sparkSprite() {
  if (sparkTexture) return sparkTexture;
  const size = 8;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  const art = [
    "00011000",
    "00111100",
    "01111110",
    "11111111",
    "11111111",
    "01111110",
    "00111100",
    "00011000",
  ];
  ctx.fillStyle = "#ffffff";
  art.forEach((row, y) => {
    [...row].forEach((cell, x) => {
      if (cell === "1") ctx.fillRect(x, y, 1, 1);
    });
  });

  sparkTexture = new THREE.CanvasTexture(canvas);
  sparkTexture.magFilter = THREE.NearestFilter;
  sparkTexture.minFilter = THREE.NearestFilter;
  sparkTexture.generateMipmaps = false;
  return sparkTexture;
}

export function noiseTexture(size = 96, existing = null) {
  const canvas = existing?.image ?? document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  const frame = ctx.createImageData(size, size);
  for (let i = 0; i < frame.data.length; i += 4) {
    const value = Math.random() < 0.5 ? Math.random() * 70 : 140 + Math.random() * 115;
    frame.data[i] = value;
    frame.data[i + 1] = value;
    frame.data[i + 2] = value;
    frame.data[i + 3] = 255;
  }
  ctx.putImageData(frame, 0, 0);

  if (existing) {
    existing.needsUpdate = true;
    return existing;
  }
  const texture = new THREE.CanvasTexture(canvas);
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  texture.generateMipmaps = false;
  texture.flipY = false;
  return texture;
}
let pixelSheet = null;

const PIXEL_FRAMES = 6;
const PIXEL_CELL = 48;
const PIXEL_GRID = 12;

function pixelSheetTexture() {
  if (pixelSheet) return pixelSheet;
  const unit = PIXEL_CELL / PIXEL_GRID;
  const canvas = document.createElement("canvas");
  canvas.width = PIXEL_CELL * PIXEL_FRAMES;
  canvas.height = PIXEL_CELL;
  const ctx = canvas.getContext("2d");
  const centre = PIXEL_GRID / 2;

  for (let frame = 0; frame < PIXEL_FRAMES; frame += 1) {
    const t = frame / (PIXEL_FRAMES - 1);
    const radius = 0.9 + t * 4.8;
    const band = Math.max(0.7, 2.1 - t * 1.5);
    for (let gy = 0; gy < PIXEL_GRID; gy += 1) {
      for (let gx = 0; gx < PIXEL_GRID; gx += 1) {
        const dx = gx + 0.5 - centre;
        const dy = gy + 0.5 - centre;
        const distance = Math.hypot(dx, dy);
        const spoke = Math.abs(Math.sin(Math.atan2(dy, dx) * 4));
        if (Math.abs(distance - radius) > band * (0.4 + spoke * 0.6)) continue;
        ctx.fillStyle = t < 0.35 || spoke > 0.65 ? "#ffffff" : "#bfe6ff";
        ctx.fillRect(frame * PIXEL_CELL + gx * unit, gy * unit, unit, unit);
      }
    }
  }

  pixelSheet = new THREE.CanvasTexture(canvas);
  pixelSheet.magFilter = THREE.NearestFilter;
  pixelSheet.minFilter = THREE.NearestFilter;
  pixelSheet.generateMipmaps = false;
  return pixelSheet;
}

function starSprite() {
  if (starTexture) return starTexture;
  const size = 32;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#ffffff";
  ctx.beginPath();
  for (let i = 0; i < 10; i += 1) {
    const radius = i % 2 === 0 ? size * 0.46 : size * 0.19;
    const angle = (i / 10) * Math.PI * 2 - Math.PI / 2;
    const x = size / 2 + Math.cos(angle) * radius;
    const y = size / 2 + Math.sin(angle) * radius;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.closePath();
  ctx.fill();

  starTexture = new THREE.CanvasTexture(canvas);
  starTexture.magFilter = THREE.NearestFilter;
  starTexture.minFilter = THREE.NearestFilter;
  return starTexture;
}

const PRESETS = {
  punch: {
    ring: { inner: 0.7, outer: 1.05, span: 0.75, growth: 2.4, opacity: 0.95 },
    burst: { count: 110, size: 0.26, speed: [3.5, 8.5], lift: 0.8, gravity: 9, span: 0.9 },
    flash: { intensity: 42, life: 0.18 },
  },
  knockout: {
    ring: { inner: 0.8, outer: 1.4, span: 1.25, growth: 4.2, opacity: 1 },
    burst: { count: 210, size: 0.34, speed: [4.5, 13], lift: 1.1, gravity: 11, span: 1.5 },
    flash: { intensity: 95, life: 0.32 },
    stars: true,
  },
  victory: {
    ring: { inner: 0.6, outer: 1.0, span: 1.6, growth: 3.4, opacity: 0.8 },
    burst: { count: 240, size: 0.3, speed: [2.5, 7], lift: 2.6, gravity: 5.2, span: 2.4 },
    flash: { intensity: 58, life: 0.26 },
    confetti: true,
  },
  hit: {
    ring: { inner: 0.4, outer: 0.7, span: 0.55, growth: 2.2, opacity: 0.7 },
    burst: { count: 80, size: 0.22, speed: [3, 7.5], lift: 0.9, gravity: 10, span: 0.7 },
    flash: { intensity: 30, life: 0.14 },
  },
  solve: {
    ring: { inner: 0.5, outer: 0.85, span: 0.9, growth: 2.9, opacity: 0.85 },
    burst: { count: 90, size: 0.24, speed: [2.5, 6], lift: 1.4, gravity: 6, span: 0.9 },
    flash: { intensity: 36, life: 0.16 },
  },
};

const PALETTES = {
  punch: (teamColor) => [teamColor, THEME.white, THEME.brandLight],
  knockout: () => [THEME.impact, THEME.gold, THEME.white],
  victory: () => [THEME.gold, THEME.brand, THEME.brandLight, THEME.white],
  solve: () => [THEME.brand, THEME.brandLight, THEME.white],
  hit: (teamColor) => [THEME.white, teamColor, THEME.gold],
};

export function buildSpotRig(scene, { colors, fight, corner }) {
  const rig = { spots: [], beams: [], rims: [] };

  const addSpot = (origin, target, color, intensity, angle, beamRadius) => {
    const spot = new THREE.SpotLight(color, intensity, 70, angle, 0.34, 1.05);
    spot.position.copy(origin);
    spot.target.position.copy(target);
    spot.castShadow = true;
    spot.shadow.mapSize.set(1024, 1024);
    spot.shadow.bias = -0.0008;
    scene.add(spot, spot.target);
    rig.spots.push(spot);

    if (beamRadius) {
      const axis = new THREE.Vector3().subVectors(origin, target);
      const height = axis.length();
      const beam = new THREE.Mesh(
        new THREE.ConeGeometry(beamRadius, height, 24, 1, true),
        new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: 0.035,
          blending: THREE.AdditiveBlending,
          depthWrite: false,
          side: THREE.BackSide,
        }),
      );
      beam.position.copy(origin).add(target).multiplyScalar(0.5);
      beam.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), axis.clone().normalize());
      scene.add(beam);
      rig.beams.push(beam);
    } else {
      rig.beams.push(null);
    }
    return spot;
  };

  const addRim = (origin, target, color) => {
    const rim = new THREE.SpotLight(color, RIM_INTENSITY, 46, RIM_ANGLE, 0.62, 1.1);
    rim.position.copy(origin);
    rim.target.position.copy(target);
    scene.add(rim, rim.target);
    rig.rims.push(rim);
    return rim;
  };

  const v = (x, y, z) => new THREE.Vector3(x, y, z);

  addSpot(v(fight.left.x - 0.9, BEAM_TOP, 2.2), fight.left.clone().setY(FLOOR_LIGHT_Y), 0xfff2dd, 74, Math.PI / 13, 1.05);
  addSpot(v(fight.right.x + 0.9, BEAM_TOP, 2.2), fight.right.clone().setY(FLOOR_LIGHT_Y), 0xfff2dd, 74, Math.PI / 13, 1.05);
  addSpot(v(corner.left.x - 1.2, 8.5, corner.left.z + 2.6), corner.left.clone().setY(1.1), softLight(colors.left), 56, Math.PI / 6, 0);
  addSpot(v(corner.right.x + 1.2, 8.5, corner.right.z + 2.6), corner.right.clone().setY(1.1), softLight(colors.right), 56, Math.PI / 6, 0);

  addRim(
    v(fight.left.x - RIM_SPREAD, RIM_TOP, RIM_BACK),
    fight.left.clone().setY(FLOOR_LIGHT_Y + RIM_AIM),
    softLight(colors.left),
  );
  addRim(
    v(fight.right.x + RIM_SPREAD, RIM_TOP, RIM_BACK),
    fight.right.clone().setY(FLOOR_LIGHT_Y + RIM_AIM),
    softLight(colors.right),
  );

  return rig;
}

export function floorTexture(repeat = 26) {
  const size = 64;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");

  ctx.fillStyle = "#0d1220";
  ctx.fillRect(0, 0, size, size);

  const grain = ctx.getImageData(0, 0, size, size);
  for (let i = 0; i < grain.data.length; i += 4) {
    const shift = (Math.random() - 0.5) * 16;
    grain.data[i] = Math.max(0, grain.data[i] + shift);
    grain.data[i + 1] = Math.max(0, grain.data[i + 1] + shift);
    grain.data[i + 2] = Math.max(0, grain.data[i + 2] + shift);
  }
  ctx.putImageData(grain, 0, 0);

  ctx.strokeStyle = "rgba(120, 150, 205, 0.09)";
  ctx.lineWidth = 1;
  ctx.strokeRect(0.5, 0.5, size - 1, size - 1);

  const texture = new THREE.CanvasTexture(canvas);
  texture.wrapS = THREE.RepeatWrapping;
  texture.wrapT = THREE.RepeatWrapping;
  texture.repeat.set(repeat, repeat);
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  texture.generateMipmaps = false;
  texture.colorSpace = THREE.SRGBColorSpace;
  return texture;
}

export function buildFloorBounce(radius = 15) {
  const size = 256;
  const canvas = document.createElement("canvas");
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext("2d");
  const centre = size / 2;
  const wash = ctx.createRadialGradient(centre, centre, 0, centre, centre, centre);
  wash.addColorStop(0, "rgba(150, 176, 226, 0.5)");
  wash.addColorStop(0.45, "rgba(96, 122, 176, 0.19)");
  wash.addColorStop(1, "rgba(60, 78, 122, 0)");
  ctx.fillStyle = wash;
  ctx.fillRect(0, 0, size, size);

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;

  const bounce = new THREE.Mesh(
    new THREE.PlaneGeometry(radius * 2, radius * 2),
    new THREE.MeshBasicMaterial({
      map: texture,
      transparent: true,
      opacity: 0.55,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    }),
  );
  bounce.rotation.x = -Math.PI / 2;
  bounce.position.y = 0.02;
  return bounce;
}

export class ImpactFx {
  constructor(scene) {
    this.scene = scene;
    this.active = [];

    this.flash = new THREE.PointLight(0xffffff, 0, 40);
    this.flash.position.set(0, 3.2, 0);
    scene.add(this.flash);
    this.flashLife = 0;
    this.flashPeak = 0;
    this.flashSpan = 0.22;
  }

  shockwave(position, color, preset = PRESETS.punch.ring) {
    const mesh = new THREE.Mesh(
      new THREE.RingGeometry(preset.inner, preset.outer, 24),
      new THREE.MeshBasicMaterial({
        color,
        transparent: true,
        opacity: preset.opacity,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
        side: THREE.DoubleSide,
      }),
    );
    mesh.position.copy(position);
    mesh.rotation.x = -Math.PI / 2;
    this.scene.add(mesh);
    this.active.push({
      mesh,
      life: 0,
      span: preset.span,
      growth: preset.growth,
      peak: preset.opacity,
      kind: "ring",
    });
  }

  particles(position, palette, preset, { upward = false, direction = null } = {}) {
    const { count, size, speed, lift, gravity, span } = preset;
    const stride = TRAIL_SAMPLES + 1;
    const total = count * stride;
    const geometry = new THREE.BufferGeometry();
    const positions = new Float32Array(total * 3);
    const colors = new Float32Array(total * 3);
    const velocities = [];
    const tint = new THREE.Color();

    for (let i = 0; i < count; i += 1) {
      tint.setHex(palette[i % palette.length]);
      for (let sample = 0; sample < stride; sample += 1) {
        const index = (i * stride + sample) * 3;
        positions[index] = position.x;
        positions[index + 1] = position.y;
        positions[index + 2] = position.z;
        const fade = 1 - (sample / stride) * 0.85;
        colors[index] = tint.r * fade;
        colors[index + 1] = tint.g * fade;
        colors[index + 2] = tint.b * fade;
      }

      const magnitude = speed[0] + Math.random() * (speed[1] - speed[0]);
      if (direction) {
        velocities.push(inCone(direction, CONE_SPREAD).multiplyScalar(magnitude));
      } else {
        const angle = Math.random() * Math.PI * 2;
        const rise = upward
          ? lift * (0.6 + Math.random())
          : Math.sin(Math.random() * 0.9 + 0.1) * lift;
        velocities.push(
          new THREE.Vector3(
            Math.cos(angle) * magnitude,
            rise * magnitude * 0.5,
            Math.sin(angle) * magnitude,
          ),
        );
      }
    }

    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setAttribute("color", new THREE.BufferAttribute(colors, 3));

    const points = new THREE.Points(
      geometry,
      new THREE.PointsMaterial({
        size,
        map: sparkSprite(),
        vertexColors: true,
        transparent: true,
        alphaTest: 0.45,
        opacity: 1,
        depthWrite: false,
      }),
    );
    this.scene.add(points);
    this.active.push({
      mesh: points,
      life: 0,
      span,
      velocities,
      gravity,
      stride,
      trailClock: 0,
      baseSize: size,
      kind: "points",
    });
  }

  stars(position) {
    const count = 6;
    const group = new THREE.Group();
    group.position.copy(position);
    for (let i = 0; i < count; i += 1) {
      const sprite = new THREE.Sprite(
        new THREE.SpriteMaterial({
          map: starSprite(),
          color: THEME.gold,
          transparent: true,
          depthWrite: false,
        }),
      );
      sprite.scale.setScalar(0.42);
      group.add(sprite);
    }
    this.scene.add(group);
    this.active.push({ mesh: group, life: 0, span: 2.6, kind: "stars", radius: 0.95 });
  }

  pixel(position, color, scale = 1.6, span = 0.42) {
    const texture = pixelSheetTexture().clone();
    texture.needsUpdate = true;
    texture.repeat.set(1 / PIXEL_FRAMES, 1);
    texture.offset.set(0, 0);

    const sprite = new THREE.Sprite(
      new THREE.SpriteMaterial({
        map: texture,
        color,
        transparent: true,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
      }),
    );
    sprite.position.copy(position);
    sprite.scale.setScalar(scale);
    this.scene.add(sprite);
    this.active.push({ mesh: sprite, life: 0, span, kind: "pixel", texture });
  }

  burst(kind, position, teamColor = THEME.brand, { direction = null } = {}) {
    const preset = PRESETS[kind] ?? PRESETS.punch;
    const palette = (PALETTES[kind] ?? PALETTES.punch)(teamColor);

    this.shockwave(position, palette[0], preset.ring);
    this.particles(position, palette, preset.burst, {
      upward: Boolean(preset.confetti),
      direction: preset.confetti ? null : direction,
    });
    if (preset.stars) this.stars(position.clone().setY(position.y + 0.9));

    this.flash.position.copy(position).setY(position.y + 1);
    this.flash.color.setHex(palette[0]);
    this.flashPeak = preset.flash.intensity;
    this.flashSpan = preset.flash.life;
    this.flashLife = preset.flash.life;
  }

  punch(position, color) {
    this.burst("punch", position, color);
  }

  update(delta) {
    if (this.flashLife > 0) {
      this.flashLife = Math.max(0, this.flashLife - delta);
      const t = this.flashLife / this.flashSpan;
      this.flash.intensity = stepped(t * t) * this.flashPeak;
    } else {
      this.flash.intensity = 0;
    }

    this.active = this.active.filter((fx) => {
      fx.life += delta;
      const t = fx.life / fx.span;
      if (t >= 1) {
        this.scene.remove(fx.mesh);
        if (fx.kind === "stars") {
          fx.mesh.children.forEach((sprite) => sprite.material.dispose());
        } else if (fx.kind === "pixel") {
          fx.mesh.material.dispose();
          fx.texture.dispose();
        } else {
          fx.mesh.geometry.dispose();
          fx.mesh.material.dispose();
        }
        return false;
      }

      if (fx.kind === "pixel") {
        const frame = Math.min(PIXEL_FRAMES - 1, Math.floor(t * PIXEL_FRAMES));
        fx.texture.offset.x = frame / PIXEL_FRAMES;
        fx.mesh.material.opacity = t > 0.82 ? 0.4 : 1;
      } else if (fx.kind === "points") {
        const attr = fx.mesh.geometry.getAttribute("position");
        const array = attr.array;
        const stride = fx.stride;

        fx.trailClock += delta;
        const shift = fx.trailClock >= TRAIL_STEP;
        if (shift) fx.trailClock = 0;

        for (let i = 0; i < fx.velocities.length; i += 1) {
          const head = i * stride * 3;
          if (shift) {
            for (let sample = stride - 1; sample > 0; sample -= 1) {
              const to = head + sample * 3;
              const from = head + (sample - 1) * 3;
              array[to] = array[from];
              array[to + 1] = array[from + 1];
              array[to + 2] = array[from + 2];
            }
          }
          const velocity = fx.velocities[i];
          array[head] += velocity.x * delta;
          array[head + 1] += velocity.y * delta;
          array[head + 2] += velocity.z * delta;
          velocity.y -= fx.gravity * delta;
        }
        attr.needsUpdate = true;
        const decay = t <= SIZE_DECAY_START ? 1 : 1 - (t - SIZE_DECAY_START) / (1 - SIZE_DECAY_START);
        fx.mesh.material.size = fx.baseSize * Math.max(0, decay);
        fx.mesh.material.opacity = t <= SIZE_DECAY_START ? 1 : Math.max(0, decay);
      } else if (fx.kind === "stars") {
        const spin = fx.life * 2.4;
        fx.mesh.children.forEach((sprite, index) => {
          const angle = spin + (index / fx.mesh.children.length) * Math.PI * 2;
          sprite.position.set(
            Math.cos(angle) * fx.radius,
            Math.sin(angle * 2) * 0.16,
            Math.sin(angle) * fx.radius,
          );
          sprite.material.opacity = stepped(1 - t);
        });
      } else {
        fx.mesh.scale.setScalar(1 + t * fx.growth);
        fx.mesh.material.opacity = stepped(fx.peak * (1 - t) ** 1.1);
      }
      return true;
    });
  }
}
