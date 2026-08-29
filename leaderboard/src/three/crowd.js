import * as THREE from "three";

import {
  createLabel,
  disposeLabel,
  fadeLabel,
  LABEL_ASPECT,
  outlineOf,
  sayOnLabel,
  tintOutline,
} from "./labels";
import { toonGradient } from "./toon";

const SEAT_ROWS = 6;
const SEAT_FRONT_Z = -8.2;
const SEAT_ROW_DEPTH = 2.35;
const SEAT_ROW_RISE = 0.86;
const SEAT_SPACING = 1.95;
const SEAT_BASE_COUNT = 15;
const SEAT_ROW_GROWTH = 2;
const BLOCKED_SEATS = new Set([6, 8, 20, 23]);

const BODY_HEIGHT = 0.92;
const HEAD_RADIUS = 0.44;
const LABEL_WIDTH = 1024;
const LABEL_HEIGHT = 384;
const LABEL_SCALE = 3.1;
const BUBBLE_MS = 7000;
const BUBBLE_FADE_MS = 1400;
const EMOTE_MS = 2600;

const FACE_SWEEP = 0.82;
const FACE_LIFT = 1.012;

const clamp01 = (t) => Math.min(1, Math.max(0, t));

function buildFaceCap(radius, sweep) {
  const cap = new THREE.SphereGeometry(radius, 24, 14, 0, Math.PI * 2, 0, sweep);
  cap.rotateX(Math.PI / 2);

  const position = cap.attributes.position;
  const uv = cap.attributes.uv;
  const reach = radius * Math.sin(sweep);
  for (let i = 0; i < position.count; i += 1) {
    uv.setXY(
      i,
      0.5 + position.getX(i) / (2 * reach),
      0.5 + position.getY(i) / (2 * reach),
    );
  }
  uv.needsUpdate = true;
  return cap;
}

let bodyGeometry = null;
let headGeometry = null;
let faceGeometry = null;

function shared() {
  if (!bodyGeometry) {
    bodyGeometry = new THREE.CapsuleGeometry(0.34, BODY_HEIGHT * 0.5, 4, 10);
    headGeometry = new THREE.SphereGeometry(HEAD_RADIUS, 14, 10);
    faceGeometry = buildFaceCap(HEAD_RADIUS * FACE_LIFT, FACE_SWEEP);
  }
  return { bodyGeometry, headGeometry, faceGeometry };
}

export function buildSeatMap(reserved = [], clearance = 1.6) {
  const seats = [];
  for (let row = 0; row < SEAT_ROWS; row += 1) {
    const z = SEAT_FRONT_Z - row * SEAT_ROW_DEPTH;
    const y = 0.05 + row * SEAT_ROW_RISE;
    const count = SEAT_BASE_COUNT + row * SEAT_ROW_GROWTH;
    const offset = ((count - 1) * SEAT_SPACING) / 2;

    for (let index = 0; index < count; index += 1) {
      const stagger = row % 2 === 0 ? 0 : SEAT_SPACING / 2;
      const seat = new THREE.Vector3(index * SEAT_SPACING - offset + stagger, y, z);
      if (reserved.some((taken) => seat.distanceTo(taken) < clearance)) continue;
      seats.push(seat);
    }
  }
  return seats.filter((_, index) => !BLOCKED_SEATS.has(index));
}

function faceTexture(photo) {
  const canvas = document.createElement("canvas");
  canvas.width = 128;
  canvas.height = 128;
  const ctx = canvas.getContext("2d");
  ctx.fillStyle = "#ffd3ac";
  ctx.fillRect(0, 0, 128, 128);

  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.LinearFilter;
  texture.generateMipmaps = false;

  if (photo) {
    const image = new Image();
    image.crossOrigin = "anonymous";
    image.onload = () => {
      const side = Math.min(image.width, image.height);
      ctx.drawImage(image, (image.width - side) / 2, (image.height - side) / 2, side, side, 0, 0, 128, 128);
      texture.needsUpdate = true;
    };
    image.src = photo;
  }
  return texture;
}

class Watcher {
  constructor(person, seat, tint, overlay) {
    const { bodyGeometry: body, headGeometry: head, faceGeometry: face } = shared();

    this.id = person.id;
    this.name = person.name;
    this.seat = seat.clone();
    this.emote = null;
    this.emoteTime = 0;
    this.bubbleTime = 0;
    this.message = null;

    this.root = new THREE.Group();
    this.root.position.copy(seat);

    this.bodyMaterial = new THREE.MeshToonMaterial({
      color: tint,
      gradientMap: toonGradient(),
    });
    this.headMaterial = new THREE.MeshToonMaterial({
      color: 0xffd3ac,
      gradientMap: toonGradient(),
    });
    this.faceTexture = faceTexture(person.photo);
    this.faceMaterial = new THREE.MeshBasicMaterial({ map: this.faceTexture, toneMapped: false });

    const torso = new THREE.Mesh(body, this.bodyMaterial);
    torso.position.y = BODY_HEIGHT * 0.5;
    const skull = new THREE.Mesh(head, this.headMaterial);
    skull.position.y = BODY_HEIGHT + HEAD_RADIUS * 0.75;
    const facePlate = new THREE.Mesh(face, this.faceMaterial);
    facePlate.position.set(0, skull.position.y, 0);

    this.labelData = createLabel(this.name);
    this.label = this.labelData.sprite;
    this.label.scale.set(LABEL_SCALE, LABEL_SCALE * LABEL_ASPECT, 1);
    this.labelOffset = BODY_HEIGHT + HEAD_RADIUS * 2.6;

    this.root.add(torso, skull, facePlate);
    outlineOf(this.root, 0x0a0a0c, 0.035);
    overlay.add(this.label);
  }

  say(message) {
    this.message = message;
    this.bubbleTime = BUBBLE_MS;
    sayOnLabel(this.labelData, message);
  }

  highlight() {
    tintOutline(this.root, 0xffffff);
  }

  playEmote(emote) {
    this.restPose();
    this.emote = emote;
    this.emoteTime = 0;
  }

  faceTarget(target) {
    this.baseYaw = Math.atan2(target.x - this.root.position.x, target.z - this.root.position.z);
    this.root.rotation.y = this.baseYaw;
  }

  restPose() {
    this.root.position.copy(this.seat);
    this.root.rotation.set(0, this.baseYaw ?? 0, 0);
    this.root.scale.setScalar(1);
  }

  update(delta) {
    this.label.position.set(
      this.root.position.x,
      this.root.position.y + this.labelOffset,
      this.root.position.z,
    );

    if (this.bubbleTime > 0) {
      this.bubbleTime -= delta * 1000;
      if (this.bubbleTime <= 0) {
        this.message = null;
        sayOnLabel(this.labelData, null);
      } else if (this.bubbleTime < BUBBLE_FADE_MS) {
        fadeLabel(this.labelData, this.bubbleTime / BUBBLE_FADE_MS);
      }
    }

    if (!this.emote) return;
    this.emoteTime += delta * 1000;
    const t = clamp01(this.emoteTime / EMOTE_MS);
    const wave = Math.sin(t * Math.PI * 6);

    this.root.position.copy(this.seat);
    this.root.scale.setScalar(1);

    switch (this.emote) {
      case "wave":
        this.root.rotation.z = wave * 0.22;
        break;
      case "laugh":
        this.root.position.y = this.seat.y + Math.abs(wave) * 0.16;
        this.root.rotation.x = -0.18 - wave * 0.1;
        break;
      case "dance":
        this.root.position.y = this.seat.y + Math.abs(wave) * 0.24;
        this.root.rotation.y = (this.baseYaw ?? 0) + wave * 0.28;
        break;
      default:
        break;
    }

    if (t >= 1) {
      this.emote = null;
      this.restPose();
    }
  }

  dispose() {
    this.bodyMaterial.dispose();
    this.headMaterial.dispose();
    this.faceMaterial.dispose();
    this.faceTexture.dispose();
    disposeLabel(this.labelData);
  }
}

export class Crowd {
  constructor(scene, seats, focus, overlay) {
    this.scene = scene;
    this.overlay = overlay;
    this.seats = seats;
    this.focus = focus;
    this.watchers = new Map();
    this.viewerId = null;
    this.palette = [0x2f6f5e, 0x3b5f8a, 0x6b4a7a, 0x7a5a3a, 0x4a6b7a, 0x6b3a4a];
  }

  sync(people) {
    const seen = new Set();

    people.forEach((person) => {
      seen.add(person.id);
      const existing = this.watchers.get(person.id);
      if (existing) {
        if (existing.name !== person.name) {
          existing.name = person.name;
          existing.labelData.name = person.name;
          sayOnLabel(existing.labelData, existing.message);
        }
        return;
      }
      if (!this.seats.length) return;

      const seat = this.seats[person.seat % this.seats.length];
      const tint = this.palette[person.seat % this.palette.length];
      const watcher = new Watcher(person, seat, tint, this.overlay);
      watcher.faceTarget(this.focus);
      if (person.id === this.viewerId) watcher.highlight();
      this.scene.add(watcher.root);
      this.watchers.set(person.id, watcher);
    });

    for (const [id, watcher] of [...this.watchers]) {
      if (seen.has(id)) continue;
      this.scene.remove(watcher.root);
      this.overlay.remove(watcher.label);
      watcher.dispose();
      this.watchers.delete(id);
    }
  }

  setViewer(id) {
    this.viewerId = id;
    if (id && this.watchers.has(id)) this.watchers.get(id).highlight();
  }

  say(id, message) {
    this.watchers.get(id)?.say(message);
  }

  emote(id, emote) {
    this.watchers.get(id)?.playEmote(emote);
  }

  update(delta) {
    this.watchers.forEach((watcher) => watcher.update(delta));
  }

  dispose() {
    this.watchers.forEach((watcher) => {
      this.scene.remove(watcher.root);
      this.overlay.remove(watcher.label);
      watcher.dispose();
    });
    this.watchers.clear();
  }
}
