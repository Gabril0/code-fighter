import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { clone as cloneSkinned } from "three/examples/jsm/utils/SkeletonUtils.js";

import { pixelate, toToon } from "./toon";

const MODEL = "/models/fighter.glb?v=3";
const TARGET_HEIGHT = 3.4;
const FACE_MATERIAL = "Face";
const SKIN_FILL = "#ffd3ac";
const GEAR_SHADES = {
  Shirt: 0,
  Glove: 0.16,
  Headgear: -0.3,
  Trunks: -0.16,
};
const FACE_TEXTURE_SIZE = 512;
const DEFAULT_FADE = 0.22;
const GUARD_TURN = 0.17;
const ENGAGE_TURN = 0.42;

const LOOPING = new Set([
  "Idle", "Standing", "Sitting", "Walking", "Cheer", "Dance", "Dance2", "Still",
  "ShadowBox", "Anxious", "LookAround", "Scream", "ArmWave",
]);

const clamp01 = (t) => Math.min(1, Math.max(0, t));
const easeOutCubic = (t) => 1 - (1 - t) ** 3;
const easeInOutQuad = (t) => (t < 0.5 ? 2 * t * t : 1 - (-2 * t + 2) ** 2 / 2);
const MAX_STRIKE_GROWTH = 0.25;
const RAGDOLL_GRAVITY = 17;
const RAGDOLL_BOUNCE = 0.62;
const RAGDOLL_FRICTION = 0.8;
const RAGDOLL_KICK = 3.4;

let sourcePromise = null;

function shaded(hex, amount) {
  const color = new THREE.Color(hex);
  return amount >= 0
    ? color.lerp(new THREE.Color(0xffffff), amount)
    : color.lerp(new THREE.Color(0x000000), -amount);
}

export function loadMiiSource() {
  if (!sourcePromise) {
    sourcePromise = new GLTFLoader().loadAsync(MODEL).then((gltf) => {
      const root = gltf.scene;

      const box = new THREE.Box3().setFromObject(root);
      const size = box.getSize(new THREE.Vector3());
      const scale = TARGET_HEIGHT / (size.y || 1);
      root.scale.setScalar(scale);
      root.position.y = -box.min.y * scale;
      root.position.x = -((box.min.x + box.max.x) / 2) * scale;
      root.position.z = -((box.min.z + box.max.z) / 2) * scale;

      return { scene: root, animations: gltf.animations };
    });
  }
  return sourcePromise;
}

function buildFaceTexture(member) {
  const canvas = document.createElement("canvas");
  canvas.width = FACE_TEXTURE_SIZE;
  canvas.height = FACE_TEXTURE_SIZE;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = SKIN_FILL;
  ctx.fillRect(0, 0, FACE_TEXTURE_SIZE, FACE_TEXTURE_SIZE);

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.flipY = false;
  pixelate(texture);

  if (member?.photo) {
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.onload = () => {
      const side = Math.min(image.width, image.height);
      ctx.drawImage(
        image,
        (image.width - side) / 2,
        (image.height - side) / 2,
        side,
        side,
        0,
        0,
        FACE_TEXTURE_SIZE,
        FACE_TEXTURE_SIZE,
      );
      texture.needsUpdate = true;
    };
    image.src = member.photo;
  }

  return texture;
}

export class Mii {
  constructor({ source, member, color, side, sideSign }) {
    this.side = side;
    this.sideSign = sideSign;
    this.state = null;
    this.timers = [];
    this.move = null;
    this.fade = null;
    this.turn = null;
    this.ragdoll = null;
    this.opacity = 1;

    this.root = new THREE.Group();
    this.pivot = cloneSkinned(source.scene);
    this.root.add(this.pivot);

    this.materials = [];
    this.gearMaterials = [];
    this.bones = {};
    this.strikePulse = null;

    this.pivot.traverse((child) => {
      if (child.isBone) this.bones[child.name] = child;
      if (!child.isMesh) return;
      child.castShadow = true;
      child.receiveShadow = true;
      child.frustumCulled = false;

      const name = child.material?.name ?? "";
      child.material = toToon(child.material);
      this.materials.push(child.material);

      if (name === FACE_MATERIAL) {
        this.faceTexture = buildFaceTexture(member);
        this.faceMaterial = child.material;
        child.material.map = this.faceTexture;
        child.material.needsUpdate = true;
      }

      if (name in GEAR_SHADES) {
        const shade = GEAR_SHADES[name];
        child.material.color.copy(shaded(color, shade));
        this.gearMaterials.push({ material: child.material, shade });
      }
    });

    this.mixer = new THREE.AnimationMixer(this.pivot);
    this.actions = new Map();
    for (const clip of source.animations) {
      const action = this.mixer.clipAction(clip);
      action.clampWhenFinished = true;
      this.actions.set(clip.name, action);
    }

    this.play("Idle");
  }

  play(name, { fade = DEFAULT_FADE } = {}) {
    if (this.state === name) return;
    const next = this.actions.get(name);
    if (!next) return;

    const previous = this.state ? this.actions.get(this.state) : null;
    this.state = name;

    next.reset();
    next.setLoop(LOOPING.has(name) ? THREE.LoopRepeat : THREE.LoopOnce, Infinity);
    next.enabled = true;
    next.setEffectiveWeight(1);
    next.play();

    if (previous && previous !== next) {
      next.crossFadeFrom(previous, fade, false);
    }
  }

  replay(name, options) {
    const action = this.actions.get(name);
    if (!action) return;
    if (this.state !== name) {
      this.play(name, options);
      return;
    }
    action.reset();
    action.play();
  }

  faceCenter() {
    this.turn = null;
    this.root.rotation.y = this.guardAngle();
  }

  turnTo(angle, duration, onDone) {
    this.turn = { from: this.root.rotation.y, to: angle, elapsed: 0, duration, onDone };
  }

  engageAngle() {
    return -this.sideSign * Math.PI * ENGAGE_TURN;
  }

  guardAngle() {
    return -this.sideSign * Math.PI * GUARD_TURN;
  }

  setFace(texture) {
    if (!this.faceMaterial) return;
    this.faceTexture?.dispose();
    this.faceTexture = texture;
    this.faceMaterial.map = texture;
    this.faceMaterial.needsUpdate = true;
  }

  facePoint(target) {
    this.turn = null;
    this.root.rotation.y = Math.atan2(
      target.x - this.root.position.x,
      target.z - this.root.position.z,
    );
  }

  faceCamera() {
    this.turn = null;
    this.root.rotation.y = -this.sideSign * Math.PI * 0.05;
  }

  placeAt(position) {
    this.root.position.copy(position);
  }

  setOpacity(value) {
    this.opacity = value;
    this.materials.forEach((material) => {
      material.transparent = value < 1;
      material.opacity = value;
    });
  }

  setColor(hex) {
    this.gearMaterials.forEach(({ material, shade }) => material.color.copy(shaded(hex, shade)));
  }

  launch(velocity, bounds) {
    this.move = null;
    this.ragdoll = {
      velocity: velocity.clone(),
      spin: new THREE.Vector3(
        (Math.random() - 0.5) * 7.5,
        (Math.random() - 0.5) * 4.4,
        (Math.random() - 0.5) * 7.5,
      ),
      bounds,
      settled: false,
    };
  }

  stopRagdoll() {
    this.ragdoll = null;
    this.pivot.rotation.set(0, 0, 0);
  }

  applyRagdoll(delta) {
    const doll = this.ragdoll;
    if (!doll || doll.settled) return;

    doll.velocity.y -= RAGDOLL_GRAVITY * delta;
    this.root.position.addScaledVector(doll.velocity, delta);

    const { minX, maxX, minZ, maxZ, floor } = doll.bounds;
    if (this.root.position.x < minX || this.root.position.x > maxX) {
      this.root.position.x = THREE.MathUtils.clamp(this.root.position.x, minX, maxX);
      doll.velocity.x *= -RAGDOLL_BOUNCE;
      doll.velocity.y = Math.max(doll.velocity.y, RAGDOLL_KICK);
      doll.spin.multiplyScalar(1.25);
    }
    if (this.root.position.z < minZ || this.root.position.z > maxZ) {
      this.root.position.z = THREE.MathUtils.clamp(this.root.position.z, minZ, maxZ);
      doll.velocity.z *= -RAGDOLL_BOUNCE;
      doll.velocity.y = Math.max(doll.velocity.y, RAGDOLL_KICK);
      doll.spin.multiplyScalar(1.25);
    }

    if (this.root.position.y <= floor) {
      this.root.position.y = floor;
      doll.velocity.y *= -RAGDOLL_BOUNCE;
      doll.velocity.x *= RAGDOLL_FRICTION;
      doll.velocity.z *= RAGDOLL_FRICTION;
      doll.spin.multiplyScalar(RAGDOLL_FRICTION);
      if (doll.velocity.length() < 0.5) {
        doll.settled = true;
        doll.velocity.set(0, 0, 0);
        return;
      }
    }

    this.pivot.rotation.x += doll.spin.x * delta;
    this.pivot.rotation.y += doll.spin.y * delta;
    this.pivot.rotation.z += doll.spin.z * delta;
  }

  moveTo(position, duration, onDone) {
    this.move = {
      from: this.root.position.clone(),
      to: position.clone(),
      elapsed: 0,
      duration,
      onDone,
    };
  }

  fadeTo(target, duration, onDone) {
    this.fade = { from: this.opacity, to: target, elapsed: 0, duration, onDone };
  }

  after(ms, callback) {
    this.timers.push({ remaining: ms, callback });
  }

  strike(names, { peak = 0.5, span = 0.85, amount = MAX_STRIKE_GROWTH } = {}) {
    const bones = names.map((name) => this.bones[name]).filter(Boolean);
    if (!bones.length) return;
    this.strikePulse = {
      bones,
      elapsed: 0,
      peak,
      span,
      amount: Math.min(amount, MAX_STRIKE_GROWTH),
    };
  }

  applyStrikeScale(delta) {
    const pulse = this.strikePulse;
    if (!pulse) return;

    pulse.elapsed += delta;
    const t = clamp01(pulse.elapsed / pulse.span);
    const shape =
      t < pulse.peak
        ? (t / pulse.peak) ** 3
        : 1 - easeOutCubic(clamp01((t - pulse.peak) / (1 - pulse.peak)));
    const scale = 1 + pulse.amount * shape;
    pulse.bones.forEach((bone) => bone.scale.setScalar(scale));

    if (t >= 1) {
      pulse.bones.forEach((bone) => bone.scale.setScalar(1));
      this.strikePulse = null;
    }
  }

  update(delta) {
    this.mixer.update(delta);
    this.applyStrikeScale(delta);
    this.applyRagdoll(delta);

    for (const timer of this.timers) timer.remaining -= delta * 1000;
    const due = this.timers.filter((timer) => timer.remaining <= 0);
    this.timers = this.timers.filter((timer) => timer.remaining > 0);
    due.forEach((timer) => timer.callback());

    if (this.move) {
      this.move.elapsed += delta * 1000;
      const t = clamp01(this.move.elapsed / this.move.duration);
      this.root.position.lerpVectors(this.move.from, this.move.to, easeInOutQuad(t));
      if (t >= 1) {
        const { onDone } = this.move;
        this.move = null;
        onDone?.();
      }
    }

    if (this.turn) {
      this.turn.elapsed += delta * 1000;
      const t = clamp01(this.turn.elapsed / this.turn.duration);
      this.root.rotation.y = THREE.MathUtils.lerp(this.turn.from, this.turn.to, easeInOutQuad(t));
      if (t >= 1) {
        const { onDone } = this.turn;
        this.turn = null;
        onDone?.();
      }
    }

    if (this.fade) {
      this.fade.elapsed += delta * 1000;
      const t = clamp01(this.fade.elapsed / this.fade.duration);
      this.setOpacity(THREE.MathUtils.lerp(this.fade.from, this.fade.to, t));
      if (t >= 1) {
        const { onDone } = this.fade;
        this.fade = null;
        onDone?.();
      }
    }
  }

  dispose() {
    this.mixer.stopAllAction();
    this.faceTexture?.dispose();
    this.materials.forEach((material) => material.dispose());
  }
}
